//! Promotion of a loop-entry control guard into the Java loop condition.

use crate::ir::{
    SemanticLeaveKind, SemanticLoopControl, SemanticLoopKind, SemanticNode, SemanticPredicate,
};

pub(super) struct LoopGuardCondition;

impl LoopGuardCondition {
    pub(super) fn apply(node: SemanticNode) -> SemanticNode {
        let SemanticNode::Loop {
            control,
            header,
            kind: SemanticLoopKind::PreTested,
            mut test,
            body,
        } = node
        else {
            return node;
        };
        let Some((guard, guarded_body)) = GuardedIteration::analyze(control, &body) else {
            return SemanticNode::Loop {
                control,
                header,
                kind: SemanticLoopKind::PreTested,
                test,
                body,
            };
        };
        test.condition.value = Self::and(test.condition.value, guard);
        SemanticNode::Loop {
            control,
            header,
            kind: SemanticLoopKind::PreTested,
            test,
            body: Box::new(guarded_body),
        }
    }

    fn and(left: SemanticPredicate, right: SemanticPredicate) -> SemanticPredicate {
        match (left, right) {
            (SemanticPredicate::False, _) | (_, SemanticPredicate::False) => {
                SemanticPredicate::False
            }
            (SemanticPredicate::True, right) => right,
            (left, SemanticPredicate::True) => left,
            (SemanticPredicate::And(mut left), SemanticPredicate::And(right)) => {
                left.extend(right);
                SemanticPredicate::And(left)
            }
            (SemanticPredicate::And(mut terms), right) => {
                terms.push(right);
                SemanticPredicate::And(terms)
            }
            (left, SemanticPredicate::And(mut terms)) => {
                terms.insert(0, left);
                SemanticPredicate::And(terms)
            }
            (left, right) => SemanticPredicate::And(vec![left, right]),
        }
    }

    /// Provides a lexical iteration tail for the counted-loop proof.
    pub(super) fn expose_iteration_tail(node: &SemanticNode) -> Option<SemanticNode> {
        let SemanticNode::Loop {
            control,
            header,
            kind: SemanticLoopKind::PreTested,
            test,
            body,
        } = node
        else {
            return None;
        };
        let body = GuardedIteration::remove_terminal_continue(body, *control)
            .or_else(|| GuardedIteration::separate_trailing_exit(body, *control))?;
        Some(SemanticNode::Loop {
            control: *control,
            header: *header,
            kind: SemanticLoopKind::PreTested,
            test: test.clone(),
            body: Box::new(body),
        })
    }
}

struct GuardedIteration;

impl GuardedIteration {
    // The continuing arm skips a straight-line abrupt exit. Keep that exit
    // and its effects in the inverted branch, then use iteration fallthrough.
    fn separate_trailing_exit(
        body: &SemanticNode,
        control: SemanticLoopControl,
    ) -> Option<SemanticNode> {
        let SemanticNode::Sequence(nodes) = body else {
            return None;
        };
        let mut meaningful = nodes
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, node)| !Self::empty(node));
        let (_, exit) = meaningful.next()?;
        let leave = Self::single_leave(exit)?;
        if leave.condition.is_some()
            || !matches!(
                leave.kind,
                SemanticLeaveKind::Break
                    | SemanticLeaveKind::BreakLabel(_)
                    | SemanticLeaveKind::Return(_)
                    | SemanticLeaveKind::Throw(_)
            )
        {
            return None;
        }
        let (index, decision) =
            meaningful.find(|(_, node)| !matches!(node, SemanticNode::BasicBlock(_)))?;
        let SemanticNode::If {
            condition,
            then_node,
            else_node,
        } = decision
        else {
            return None;
        };
        if !else_node.as_deref().is_none_or(Self::empty) {
            return None;
        }
        let continuation = Self::remove_terminal_continue(then_node, control)?;
        let exit = SemanticNode::sequence(
            else_node
                .iter()
                .map(|node| node.as_ref().clone())
                .chain(nodes[index + 1..].iter().cloned()),
        );
        let mut condition = condition.clone();
        condition.value = condition.value.negate();
        let guard = SemanticNode::If {
            condition,
            then_node: Box::new(exit),
            else_node: None,
        };
        Some(SemanticNode::sequence(
            nodes[..index].iter().cloned().chain([guard, continuation]),
        ))
    }

    fn analyze(
        control: SemanticLoopControl,
        body: &SemanticNode,
    ) -> Option<(SemanticPredicate, SemanticNode)> {
        if let Some(guarded) = Self::guard_with_trailing_exit(control, body) {
            return Some(guarded);
        }
        let (prefix, decision, suffix) = Self::single_decision(body)?;
        let SemanticNode::If {
            condition,
            then_node,
            else_node,
        } = decision
        else {
            return None;
        };
        let else_node = else_node.as_deref()?;
        let (invert, continuing, exiting) = if Self::is_local_break(then_node, control) {
            (true, else_node, then_node.as_ref())
        } else if Self::is_local_break(else_node, control) {
            (false, then_node.as_ref(), else_node)
        } else {
            return None;
        };
        let continuing = Self::remove_terminal_continue(continuing, control)?;
        let mut nodes = prefix.to_vec();
        nodes.push(continuing);
        nodes.extend(Self::empty_blocks(exiting));
        nodes.extend_from_slice(suffix);
        let condition = condition.value.clone();
        let guard = if invert {
            condition.negate()
        } else {
            condition
        };
        Some((guard, SemanticNode::sequence(nodes)))
    }

    fn guard_with_trailing_exit(
        control: SemanticLoopControl,
        body: &SemanticNode,
    ) -> Option<(SemanticPredicate, SemanticNode)> {
        let nodes = match body {
            SemanticNode::Sequence(nodes) => nodes,
            node => std::slice::from_ref(node),
        };
        let mut meaningful = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| !Self::empty(node));
        let (guard_index, guard) = meaningful.next()?;
        let (exit_index, exit) = meaningful.next()?;
        if meaningful.next().is_some() || !Self::is_local_break(exit, control) {
            return None;
        }
        let SemanticNode::If {
            condition,
            then_node,
            else_node,
        } = guard
        else {
            return None;
        };
        else_node.as_deref().is_none_or(Self::empty).then_some(())?;
        let continuing = Self::remove_terminal_continue(then_node, control)?;
        let mut result = nodes[..guard_index].to_vec();
        result.push(continuing);
        if let Some(else_node) = else_node {
            result.extend(Self::empty_blocks(else_node));
        }
        result.extend(Self::empty_blocks(exit));
        result.extend(nodes[exit_index + 1..].iter().cloned());
        Some((condition.value.clone(), SemanticNode::sequence(result)))
    }

    fn single_decision(
        body: &SemanticNode,
    ) -> Option<(&[SemanticNode], &SemanticNode, &[SemanticNode])> {
        let nodes = match body {
            SemanticNode::Sequence(nodes) => nodes.as_slice(),
            node => std::slice::from_ref(node),
        };
        let mut meaningful = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| !Self::empty(node));
        let (index, decision) = meaningful.next()?;
        meaningful
            .next()
            .is_none()
            .then_some((&nodes[..index], decision, &nodes[index + 1..]))
    }

    fn remove_terminal_continue(
        body: &SemanticNode,
        control: SemanticLoopControl,
    ) -> Option<SemanticNode> {
        let nodes = match body {
            SemanticNode::Sequence(nodes) => nodes.as_slice(),
            node => std::slice::from_ref(node),
        };
        let index = nodes.iter().rposition(|node| !Self::empty(node))?;
        Self::is_local_continue(&nodes[index], control).then_some(())?;
        Some(SemanticNode::sequence(
            nodes[..index].iter().chain(&nodes[index + 1..]).cloned(),
        ))
    }

    fn is_local_break(node: &SemanticNode, control: SemanticLoopControl) -> bool {
        Self::single_leave(node).is_some_and(|leave| match control {
            SemanticLoopControl::Region(region) => {
                matches!(leave.kind, SemanticLeaveKind::Break) && leave.target == region
            }
            SemanticLoopControl::Label(label) => {
                matches!(leave.kind, SemanticLeaveKind::BreakLabel(target) if target == label)
            }
        })
    }

    fn is_local_continue(node: &SemanticNode, control: SemanticLoopControl) -> bool {
        Self::single_leave(node).is_some_and(|leave| {
            leave.condition.is_none()
                && match control {
                    SemanticLoopControl::Region(region) => {
                        matches!(leave.kind, SemanticLeaveKind::Continue) && leave.target == region
                    }
                    SemanticLoopControl::Label(label) => {
                        matches!(leave.kind, SemanticLeaveKind::ContinueLabel(target) if target == label)
                    }
                }
        })
    }

    fn single_leave(node: &SemanticNode) -> Option<&crate::ir::SemanticLeave> {
        match node {
            SemanticNode::Leave(leave) if leave.cleanup.is_empty() => Some(leave),
            SemanticNode::Sequence(nodes) => {
                let mut meaningful = nodes.iter().filter(|node| !Self::empty(node));
                let leave = meaningful.next().and_then(Self::single_leave)?;
                meaningful.next().is_none().then_some(leave)
            }
            _ => None,
        }
    }

    fn empty(node: &SemanticNode) -> bool {
        match node {
            SemanticNode::Empty => true,
            SemanticNode::BasicBlock(block) => block.statements.is_empty(),
            SemanticNode::Sequence(nodes) => nodes.iter().all(Self::empty),
            _ => false,
        }
    }

    fn empty_blocks(root: &SemanticNode) -> Vec<SemanticNode> {
        let mut pending = vec![root];
        let mut blocks = Vec::new();
        while let Some(node) = pending.pop() {
            match node {
                SemanticNode::BasicBlock(block) if block.statements.is_empty() => {
                    blocks.push(node.clone())
                }
                SemanticNode::Sequence(nodes) => pending.extend(nodes.iter().rev()),
                SemanticNode::If {
                    then_node,
                    else_node,
                    ..
                } => {
                    if let Some(else_node) = else_node {
                        pending.push(else_node);
                    }
                    pending.push(then_node);
                }
                _ => {}
            }
        }
        blocks
    }
}
