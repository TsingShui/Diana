//! Completion algebra for structured semantic IR.
//!
//! This is the single source of truth for normal and abrupt control outcomes.
//! Consumers must not infer reachability from rendered shapes or instruction
//! patterns.

use std::collections::BTreeSet;

use crate::ir::{BlockId, BoolExpr, RegionId};

use super::{
    SemanticBlock, SemanticLabel, SemanticLeave, SemanticLeaveKind, SemanticLoopControl,
    SemanticLoopKind, SemanticNode, SemanticPredicate, SemanticStatement, SemanticVisitor,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AbruptCompletion {
    FallThrough {
        block: BlockId,
        source: RegionId,
        destination: RegionId,
        target: RegionId,
        cleanup: Vec<RegionId>,
    },
    Jump {
        block: BlockId,
        source: RegionId,
        destination: RegionId,
        target: RegionId,
        cleanup: Vec<RegionId>,
    },
    BreakLabel {
        label: SemanticLabel,
        source: RegionId,
        destination: RegionId,
        target: RegionId,
        cleanup: Vec<RegionId>,
    },
    ContinueLabel {
        label: SemanticLabel,
        source: RegionId,
        destination: RegionId,
        target: RegionId,
        cleanup: Vec<RegionId>,
    },
    Return {
        source: RegionId,
        destination: RegionId,
        target: RegionId,
        cleanup: Vec<RegionId>,
    },
    Throw {
        source: RegionId,
        destination: RegionId,
        target: RegionId,
        cleanup: Vec<RegionId>,
    },
    NoReturnCall,
    Break {
        source: RegionId,
        destination: RegionId,
        target: RegionId,
        cleanup: Vec<RegionId>,
    },
    Continue {
        source: RegionId,
        destination: RegionId,
        target: RegionId,
        cleanup: Vec<RegionId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SemanticTransfer {
    FallThrough(BlockId, RegionId),
    Jump(BlockId, RegionId),
    BreakLabel(SemanticLabel),
    ContinueLabel(SemanticLabel),
    Return,
    Throw,
    Break(RegionId),
    Continue(RegionId),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SemanticControlTopology {
    tokens: Vec<ControlToken>,
}

impl SemanticControlTopology {
    #[cfg(test)]
    pub(crate) fn analyze(root: &SemanticNode) -> Self {
        let mut topology = Self { tokens: Vec::new() };
        topology.visit_node(root);
        topology
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ControlToken {
    Empty,
    Block(BlockId),
    Sequence(usize),
    If(bool),
    Loop(SemanticLoopControl, Option<BlockId>, SemanticLoopKind),
    For(SemanticLoopControl),
    ForEach(SemanticLoopControl),
    Switch(Option<RegionId>, Vec<bool>),
    Try(RegionId, Vec<RegionId>, Option<RegionId>),
    Synchronized(RegionId),
    Label(SemanticLabel),
    Leave(AbruptCompletion, bool),
    End,
}

impl SemanticVisitor for SemanticControlTopology {
    fn visits_values(&self) -> bool {
        false
    }

    fn enter_node(&mut self, node: &SemanticNode) {
        self.tokens.push(match node {
            SemanticNode::Empty => ControlToken::Empty,
            SemanticNode::BasicBlock(block) => ControlToken::Block(block.id),
            SemanticNode::Sequence(children) => ControlToken::Sequence(children.len()),
            SemanticNode::If { else_node, .. } => ControlToken::If(else_node.is_some()),
            SemanticNode::Loop {
                control,
                header,
                kind,
                ..
            } => ControlToken::Loop(*control, *header, *kind),
            SemanticNode::For { control, .. } => ControlToken::For(*control),
            SemanticNode::ForEach { control, .. } => ControlToken::ForEach(*control),
            SemanticNode::Switch { region, cases, .. } => {
                ControlToken::Switch(*region, cases.iter().map(|case| case.is_default).collect())
            }
            SemanticNode::Try {
                region,
                catches,
                finally,
                ..
            } => ControlToken::Try(
                *region,
                catches.iter().map(|catch| catch.region).collect(),
                finally.as_ref().map(|finally| finally.region),
            ),
            SemanticNode::Synchronized { region, .. } => ControlToken::Synchronized(*region),
            SemanticNode::Label { label, .. } => ControlToken::Label(*label),
            SemanticNode::Leave(leave) => ControlToken::Leave(
                AbruptCompletion::from_leave(leave),
                leave.condition.is_some(),
            ),
        });
    }

    fn exit_node(&mut self, node: &SemanticNode) {
        if matches!(
            node,
            SemanticNode::Sequence(_)
                | SemanticNode::If { .. }
                | SemanticNode::Loop { .. }
                | SemanticNode::For { .. }
                | SemanticNode::ForEach { .. }
                | SemanticNode::Switch { .. }
                | SemanticNode::Try { .. }
                | SemanticNode::Synchronized { .. }
                | SemanticNode::Label { .. }
        ) {
            self.tokens.push(ControlToken::End);
        }
    }
}

impl AbruptCompletion {
    fn from_leave(leave: &SemanticLeave) -> Self {
        let source = leave.source;
        let destination = leave.destination;
        let target = leave.target;
        let cleanup = leave.cleanup.clone();
        match &leave.kind {
            SemanticLeaveKind::FallThrough(block) => Self::FallThrough {
                block: *block,
                source,
                destination,
                target,
                cleanup,
            },
            SemanticLeaveKind::Jump(block) => Self::Jump {
                block: *block,
                source,
                destination,
                target,
                cleanup,
            },
            SemanticLeaveKind::BreakLabel(label) => Self::BreakLabel {
                label: *label,
                source,
                destination,
                target,
                cleanup,
            },
            SemanticLeaveKind::ContinueLabel(label) => Self::ContinueLabel {
                label: *label,
                source,
                destination,
                target,
                cleanup,
            },
            SemanticLeaveKind::Return(_) => Self::Return {
                source,
                destination,
                target,
                cleanup,
            },
            SemanticLeaveKind::Throw(_) => Self::Throw {
                source,
                destination,
                target,
                cleanup,
            },
            SemanticLeaveKind::Break => Self::Break {
                source,
                destination,
                target,
                cleanup,
            },
            SemanticLeaveKind::Continue => Self::Continue {
                source,
                destination,
                target,
                cleanup,
            },
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        matches!(self, Self::FallThrough { .. } | Self::Jump { .. })
    }

    fn is_open_to(&self, scope: RegionId) -> bool {
        matches!(
            self,
            Self::FallThrough { target, .. } | Self::Jump { target, .. } if *target == scope
        )
    }

    fn is_break_for_region(&self, region: RegionId) -> bool {
        matches!(self, Self::Break { target, .. } if *target == region)
    }

    fn is_continue_for_region(&self, region: RegionId) -> bool {
        matches!(self, Self::Continue { target, .. } if *target == region)
    }

    fn is_break_for_label(&self, expected: SemanticLabel) -> bool {
        matches!(self, Self::BreakLabel { label, .. } if *label == expected)
    }

    fn is_continue_for_label(&self, expected: SemanticLabel) -> bool {
        matches!(self, Self::ContinueLabel { label, .. } if *label == expected)
    }

    fn exits_loop(&self, region: RegionId, label: Option<SemanticLabel>) -> bool {
        match self {
            Self::Return { .. } | Self::Throw { .. } | Self::NoReturnCall => true,
            Self::FallThrough { target, .. }
            | Self::Jump { target, .. }
            | Self::Break { target, .. }
            | Self::Continue { target, .. } => *target != region,
            Self::BreakLabel { label: target, .. } | Self::ContinueLabel { label: target, .. } => {
                Some(*target) != label
            }
        }
    }

    fn transfer(&self) -> SemanticTransfer {
        match self {
            Self::FallThrough { block, target, .. } => {
                SemanticTransfer::FallThrough(*block, *target)
            }
            Self::Jump { block, target, .. } => SemanticTransfer::Jump(*block, *target),
            Self::BreakLabel { label, .. } => SemanticTransfer::BreakLabel(*label),
            Self::ContinueLabel { label, .. } => SemanticTransfer::ContinueLabel(*label),
            Self::Return { .. } => SemanticTransfer::Return,
            Self::Throw { .. } => SemanticTransfer::Throw,
            Self::NoReturnCall => SemanticTransfer::Throw,
            Self::Break { target, .. } => SemanticTransfer::Break(*target),
            Self::Continue { target, .. } => SemanticTransfer::Continue(*target),
        }
    }
}

impl SemanticTransfer {
    pub(crate) fn from_leave(leave: &SemanticLeave) -> Self {
        match &leave.kind {
            SemanticLeaveKind::FallThrough(block) => Self::FallThrough(*block, leave.target),
            SemanticLeaveKind::Jump(block) => Self::Jump(*block, leave.target),
            SemanticLeaveKind::BreakLabel(label) => Self::BreakLabel(*label),
            SemanticLeaveKind::ContinueLabel(label) => Self::ContinueLabel(*label),
            SemanticLeaveKind::Return(_) => Self::Return,
            SemanticLeaveKind::Throw(_) => Self::Throw,
            SemanticLeaveKind::Break => Self::Break(leave.target),
            SemanticLeaveKind::Continue => Self::Continue(leave.target),
        }
    }
}

/// Keep the smallest abrupt outcome inline. A straight-line return or throw
/// has one outcome and needs no tree allocation. Keeping the first outcome in
/// order also makes equality independent of branch traversal order.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AbruptOutcomes<O> {
    first: Option<O>,
    rest: BTreeSet<O>,
}

impl<O> Default for AbruptOutcomes<O> {
    fn default() -> Self {
        Self {
            first: None,
            rest: BTreeSet::new(),
        }
    }
}

impl<O: Ord> AbruptOutcomes<O> {
    fn iter(&self) -> impl Iterator<Item = &O> {
        self.first.iter().chain(&self.rest)
    }

    fn is_empty(&self) -> bool {
        self.first.is_none()
    }

    fn insert(&mut self, mut item: O) {
        let Some(first) = &mut self.first else {
            self.first = Some(item);
            return;
        };
        match item.cmp(first) {
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Less => std::mem::swap(&mut item, first),
            std::cmp::Ordering::Greater => {}
        }
        self.rest.insert(item);
    }

    fn extend(&mut self, mut other: Self) {
        if self.is_empty() {
            *self = other;
            return;
        }
        if self.rest.len() < other.rest.len() {
            std::mem::swap(self, &mut other);
        }
        if let Some(first) = other.first {
            self.insert(first);
            self.rest.extend(other.rest);
        }
    }

    fn retain(&mut self, mut keep: impl FnMut(&O) -> bool) {
        self.rest.retain(&mut keep);
        if self.first.as_ref().is_some_and(|item| !keep(item)) {
            self.first = self.rest.pop_first();
        }
    }
}

/// Consumers choose the identity they compare. Both projections use the same
/// completion interpreter, including the consumption of loop and label exits.
pub(crate) trait CompletionOutcome: Ord {
    fn of_leave(leave: &SemanticLeave) -> Self;
    fn no_return() -> Self;
    fn transfer(&self) -> SemanticTransfer;
}

impl CompletionOutcome for AbruptCompletion {
    fn of_leave(leave: &SemanticLeave) -> Self {
        Self::from_leave(leave)
    }
    fn no_return() -> Self {
        Self::NoReturnCall
    }
    fn transfer(&self) -> SemanticTransfer {
        AbruptCompletion::transfer(self)
    }
}

impl CompletionOutcome for SemanticTransfer {
    fn of_leave(leave: &SemanticLeave) -> Self {
        Self::from_leave(leave)
    }
    fn no_return() -> Self {
        Self::Throw
    }
    fn transfer(&self) -> SemanticTransfer {
        *self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionState<O> {
    normal: bool,
    abrupt: AbruptOutcomes<O>,
}

pub(crate) type SemanticCompletion = CompletionState<AbruptCompletion>;
pub(crate) type ControlCompletion = CompletionState<SemanticTransfer>;

impl ControlCompletion {
    /// A caller asking only for fallthrough does not consume the identities of
    /// abrupt exits. An unmatched switch or a skipped arm already provides a
    /// normal path regardless of what leaves its body. Other scoped transfers
    /// are resolved by the ordinary completion fold.
    pub(crate) fn normally_completes(root: &SemanticNode) -> bool {
        fn child(node: &SemanticNode) -> bool {
            match node {
                SemanticNode::Empty | SemanticNode::ForEach { .. } => true,
                SemanticNode::BasicBlock(block) => !block_has_no_return_call(block),
                SemanticNode::Leave(_) => false,
                SemanticNode::If {
                    condition,
                    else_node: None,
                    ..
                } if condition.constant_value() != Some(true) => true,
                SemanticNode::Switch { cases, .. } if !cases.iter().any(|case| case.is_default) => {
                    true
                }
                _ => ControlCompletion::analyze(node).normal,
            }
        }
        match root {
            SemanticNode::Sequence(children) => children.iter().all(child),
            node => child(node),
        }
    }

    pub(crate) fn void_method(mut self) -> Self {
        if self.normal {
            self.normal = false;
            self.abrupt.insert(SemanticTransfer::Return);
        }
        self
    }
}

impl<O: CompletionOutcome> CompletionState<O> {
    pub(crate) fn analyze(root: &SemanticNode) -> Self {
        let domain = SemanticCompletionDomain::<O>(std::marker::PhantomData);
        let child_outcomes =
            |node: &SemanticNode| match CompletionInterpreter::analyze(node, &domain) {
                Ok(completion) => completion,
                Err(error) => match error {},
            };
        match root {
            SemanticNode::Sequence(children) => Self::sequence(children.iter().map(child_outcomes)),
            _ => child_outcomes(root),
        }
    }

    pub(crate) fn can_complete_normally(&self) -> bool {
        self.normal
    }
}

impl SemanticCompletion {
    pub(crate) fn statement(statement: &SemanticStatement) -> Self {
        if statement_has_no_return_call(statement) {
            Self::abrupt(AbruptCompletion::NoReturnCall)
        } else {
            Self::normal()
        }
    }

    pub(crate) fn has_open_transfer(&self) -> bool {
        self.abrupt.iter().any(AbruptCompletion::is_open)
    }

    pub(crate) fn has_open_transfer_to(&self, scope: RegionId) -> bool {
        self.abrupt
            .iter()
            .any(|completion| completion.is_open_to(scope))
    }

    pub(crate) fn has_closed_transfer(&self) -> bool {
        self.abrupt.iter().any(|completion| !completion.is_open())
    }

    pub(crate) fn is_transfer_free(&self) -> bool {
        self.abrupt.is_empty()
    }

    pub(crate) fn has_break_to_region(&self, region: RegionId) -> bool {
        self.abrupt
            .iter()
            .any(|completion| completion.is_break_for_region(region))
    }

    pub(crate) fn has_break_to_label(&self, label: SemanticLabel) -> bool {
        self.abrupt
            .iter()
            .any(|completion| completion.is_break_for_label(label))
    }

    pub(crate) fn is_continue_to_region(&self, region: RegionId) -> bool {
        !self.normal
            && self.abrupt.first.is_some()
            && self.abrupt.rest.is_empty()
            && self
                .abrupt
                .iter()
                .all(|completion| completion.is_continue_for_region(region))
    }

    pub(crate) fn has_continue_to_region(&self, region: RegionId) -> bool {
        self.abrupt
            .iter()
            .any(|completion| completion.is_continue_for_region(region))
    }

    pub(crate) fn has_continue_to_label(&self, label: SemanticLabel) -> bool {
        self.abrupt
            .iter()
            .any(|completion| completion.is_continue_for_label(label))
    }

    pub(crate) fn exits_loop(&self, region: RegionId, label: Option<SemanticLabel>) -> bool {
        !self.normal
            && !self.abrupt.is_empty()
            && self
                .abrupt
                .iter()
                .all(|completion| completion.exits_loop(region, label))
    }
}

impl<O: CompletionOutcome> CompletionState<O> {
    fn normal() -> Self {
        Self {
            normal: true,
            abrupt: AbruptOutcomes::default(),
        }
    }

    fn abrupt(completion: O) -> Self {
        Self {
            normal: false,
            abrupt: AbruptOutcomes {
                first: Some(completion),
                rest: BTreeSet::new(),
            },
        }
    }

    pub(crate) fn sequence(children: impl IntoIterator<Item = Self>) -> Self {
        let mut result = Self::normal();
        for child in children {
            result.normal = child.normal;
            result.abrupt.extend(child.abrupt);
            if !result.normal {
                break;
            }
        }
        result
    }

    pub(crate) fn branch(
        condition: &SemanticPredicate,
        then_state: Self,
        else_state: Option<Self>,
    ) -> Self {
        match condition.constant_value() {
            Some(true) => then_state,
            Some(false) => else_state.unwrap_or_else(Self::normal),
            None => Self::alternatives([then_state, else_state.unwrap_or_else(Self::normal)]),
        }
    }

    fn alternatives(children: impl IntoIterator<Item = Self>) -> Self {
        let mut result = Self {
            normal: false,
            abrupt: AbruptOutcomes::default(),
        };
        for child in children {
            result.normal |= child.normal;
            result.abrupt.extend(child.abrupt);
        }
        result
    }

    fn consume_region_loop(mut self, region: Option<RegionId>) -> (Self, bool, bool) {
        let Some(region) = region else {
            return (self, false, false);
        };
        let has_break = self
            .abrupt
            .iter()
            .any(|completion| completion.transfer() == SemanticTransfer::Break(region));
        let has_continue = self
            .abrupt
            .iter()
            .any(|completion| completion.transfer() == SemanticTransfer::Continue(region));
        self.abrupt.retain(|completion| {
            completion.transfer() != SemanticTransfer::Break(region)
                && completion.transfer() != SemanticTransfer::Continue(region)
        });
        (self, has_break, has_continue)
    }

    fn consume_labeled_loop(mut self, label: SemanticLabel) -> (Self, bool, bool) {
        let has_break = self
            .abrupt
            .iter()
            .any(|completion| completion.transfer() == SemanticTransfer::BreakLabel(label));
        let has_continue = self
            .abrupt
            .iter()
            .any(|completion| completion.transfer() == SemanticTransfer::ContinueLabel(label));
        self.abrupt.retain(|completion| {
            completion.transfer() != SemanticTransfer::BreakLabel(label)
                && completion.transfer() != SemanticTransfer::ContinueLabel(label)
        });
        (self, has_break, has_continue)
    }

    fn consume_loop(self, control: SemanticLoopControl) -> (Self, bool, bool) {
        match control {
            SemanticLoopControl::Region(region) => self.consume_region_loop(Some(region)),
            SemanticLoopControl::Label(label) => self.consume_labeled_loop(label),
        }
    }

    fn consume_label(mut self, label: SemanticLabel) -> Self {
        let has_break = self
            .abrupt
            .iter()
            .any(|completion| completion.transfer() == SemanticTransfer::BreakLabel(label));
        self.abrupt
            .retain(|completion| completion.transfer() != SemanticTransfer::BreakLabel(label));
        self.normal |= has_break;
        self
    }
}

fn block_has_no_return_call(block: &SemanticBlock) -> bool {
    block.statements.iter().any(statement_has_no_return_call)
}

fn statement_has_no_return_call(statement: &SemanticStatement) -> bool {
    statement
        .instruction_ref()
        .is_some_and(|operation| operation.payload.no_return)
}

struct SemanticCompletionDomain<O>(std::marker::PhantomData<O>);

impl<O: CompletionOutcome> CompletionDomain for SemanticCompletionDomain<O> {
    type State = CompletionState<O>;
    type Error = std::convert::Infallible;

    fn normal(&self) -> Result<Self::State, Self::Error> {
        Ok(Self::State::normal())
    }

    fn no_return_call(&self) -> Result<Self::State, Self::Error> {
        Ok(Self::State::abrupt(O::no_return()))
    }

    fn leave(&self, leave: &SemanticLeave) -> Result<Self::State, Self::Error> {
        Ok(Self::State::abrupt(O::of_leave(leave)))
    }

    fn sequence(
        &self,
        children: impl IntoIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error> {
        Ok(Self::State::sequence(children))
    }

    fn branch(
        &self,
        condition: &SemanticPredicate,
        then_state: Self::State,
        else_state: Option<Self::State>,
    ) -> Result<Self::State, Self::Error> {
        Ok(Self::State::branch(condition, then_state, else_state))
    }

    fn loop_node(
        &self,
        control: SemanticLoopControl,
        kind: SemanticLoopKind,
        condition: &SemanticPredicate,
        setup: Self::State,
        body: Self::State,
    ) -> Result<Self::State, Self::Error> {
        let (mut body, has_break, has_continue) = body.consume_loop(control);
        let constant = condition.constant_value();
        let condition_true = constant == Some(true);
        let condition_false = constant == Some(false);
        let reaches_test = body.normal || has_continue;
        if matches!(kind, SemanticLoopKind::PostTested) {
            if reaches_test {
                body.abrupt.extend(setup.abrupt);
            }
            body.normal = has_break || (reaches_test && setup.normal && !condition_true);
            return Ok(body);
        }

        if !setup.normal {
            return Ok(setup);
        }
        if matches!(kind, SemanticLoopKind::PreTested) && condition_false {
            return Ok(setup);
        }
        body.abrupt.extend(setup.abrupt);
        body.normal = match kind {
            SemanticLoopKind::PreTested => !condition_true || has_break,
            SemanticLoopKind::Endless => has_break,
            SemanticLoopKind::PostTested => unreachable!(),
        };
        Ok(body)
    }

    fn for_node(
        &self,
        control: SemanticLoopControl,
        condition: &SemanticPredicate,
        body: Self::State,
    ) -> Result<Self::State, Self::Error> {
        let (mut body, has_break, _) = body.consume_loop(control);
        body.normal = condition.constant_value() != Some(true) || has_break;
        Ok(body)
    }

    fn for_each_node(
        &self,
        control: SemanticLoopControl,
        body: Self::State,
    ) -> Result<Self::State, Self::Error> {
        let (mut body, _, _) = body.consume_loop(control);
        body.normal = true;
        Ok(body)
    }

    fn switch_node(
        &self,
        region: Option<RegionId>,
        has_default: bool,
        cases: impl IntoIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error> {
        let mut result = Self::State::alternatives(cases);
        if let Some(region) = region {
            let has_break = result
                .abrupt
                .iter()
                .any(|completion| completion.transfer() == SemanticTransfer::Break(region));
            result
                .abrupt
                .retain(|completion| completion.transfer() != SemanticTransfer::Break(region));
            result.normal |= has_break;
        }
        result.normal |= !has_default;
        Ok(result)
    }

    fn try_node(
        &self,
        catches: usize,
        has_finally: bool,
        mut children: impl DoubleEndedIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error> {
        let finally = has_finally.then(|| children.next_back().expect("finally child"));
        let protected = Self::State::alternatives(children.take(catches + 1));
        let Some(finally) = finally else {
            return Ok(protected);
        };
        let mut result = CompletionState {
            normal: protected.normal && finally.normal,
            abrupt: finally.abrupt,
        };
        if finally.normal {
            result.abrupt.extend(protected.abrupt);
        }
        Ok(result)
    }

    fn synchronized(&self, body: Self::State) -> Result<Self::State, Self::Error> {
        Ok(body)
    }

    fn label(&self, label: SemanticLabel, body: Self::State) -> Result<Self::State, Self::Error> {
        Ok(body.consume_label(label))
    }
}

pub(crate) trait CompletionDomain {
    type State;
    type Error;

    fn normal(&self) -> Result<Self::State, Self::Error>;
    fn basic_block(&self, _block: &SemanticBlock) -> Result<Self::State, Self::Error> {
        self.normal()
    }
    fn no_return_call(&self) -> Result<Self::State, Self::Error> {
        self.normal()
    }
    fn leave(&self, leave: &SemanticLeave) -> Result<Self::State, Self::Error>;
    fn sequence(
        &self,
        children: impl IntoIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error>;
    fn branch(
        &self,
        condition: &SemanticPredicate,
        then_state: Self::State,
        else_state: Option<Self::State>,
    ) -> Result<Self::State, Self::Error>;
    fn loop_node(
        &self,
        control: SemanticLoopControl,
        kind: SemanticLoopKind,
        condition: &SemanticPredicate,
        setup: Self::State,
        body: Self::State,
    ) -> Result<Self::State, Self::Error>;
    fn for_node(
        &self,
        control: SemanticLoopControl,
        condition: &SemanticPredicate,
        body: Self::State,
    ) -> Result<Self::State, Self::Error>;
    fn for_each_node(
        &self,
        control: SemanticLoopControl,
        body: Self::State,
    ) -> Result<Self::State, Self::Error>;
    fn switch_node(
        &self,
        region: Option<RegionId>,
        has_default: bool,
        cases: impl IntoIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error>;
    fn try_node(
        &self,
        catches: usize,
        has_finally: bool,
        children: impl DoubleEndedIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error>;
    fn synchronized(&self, body: Self::State) -> Result<Self::State, Self::Error>;
    fn label(&self, label: SemanticLabel, body: Self::State) -> Result<Self::State, Self::Error>;
}

enum CompletionTask<'a> {
    Visit(&'a SemanticNode),
    Combine(CompletionFrame<'a>),
}

enum CompletionFrame<'a> {
    Sequence(&'a [SemanticNode]),
    If {
        condition: &'a SemanticPredicate,
        has_else: bool,
    },
    Loop {
        control: SemanticLoopControl,
        kind: SemanticLoopKind,
        condition: &'a SemanticPredicate,
    },
    For {
        control: SemanticLoopControl,
        condition: &'a SemanticPredicate,
    },
    ForEach {
        control: SemanticLoopControl,
    },
    Switch {
        region: Option<RegionId>,
        cases: usize,
        has_default: bool,
    },
    Try {
        catches: usize,
        has_finally: bool,
    },
    Synchronized,
    Label(SemanticLabel),
}

pub(crate) struct CompletionInterpreter;

impl CompletionInterpreter {
    pub(crate) fn analyze<D: CompletionDomain>(
        root: &SemanticNode,
        domain: &D,
    ) -> Result<D::State, D::Error> {
        Self::analyze_sequences(root, domain, |_, _| {})
    }

    fn leaf_state<D: CompletionDomain>(
        node: &SemanticNode,
        domain: &D,
    ) -> Option<Result<D::State, D::Error>> {
        match node {
            SemanticNode::Empty => Some(domain.normal()),
            SemanticNode::BasicBlock(block) => Some(if block_has_no_return_call(block) {
                domain.no_return_call()
            } else {
                domain.basic_block(block)
            }),
            SemanticNode::Leave(leave) => Some(domain.leave(leave)),
            SemanticNode::Sequence(_)
            | SemanticNode::If { .. }
            | SemanticNode::Loop { .. }
            | SemanticNode::For { .. }
            | SemanticNode::ForEach { .. }
            | SemanticNode::Switch { .. }
            | SemanticNode::Try { .. }
            | SemanticNode::Synchronized { .. }
            | SemanticNode::Label { .. } => None,
        }
    }

    /// Observe the already computed child states when the domain folds a
    /// sequence. Other children are consumed directly by their parent.
    pub(crate) fn analyze_sequences<D: CompletionDomain>(
        root: &SemanticNode,
        domain: &D,
        mut sequence: impl FnMut(&[SemanticNode], &[D::State]),
    ) -> Result<D::State, D::Error> {
        if let Some(state) = Self::leaf_state(root, domain) {
            return state;
        }
        let mut tasks = smallvec::SmallVec::<[CompletionTask; 4]>::new();
        let mut results = smallvec::SmallVec::<[D::State; 4]>::new();
        tasks.push(CompletionTask::Visit(root));
        while let Some(task) = tasks.pop() {
            match task {
                CompletionTask::Visit(node) => {
                    Self::schedule(node, domain, &mut tasks, &mut results)?
                }
                CompletionTask::Combine(frame) => {
                    let state = Self::combine(frame, &mut results, domain, &mut sequence)?;
                    results.push(state);
                }
            }
        }
        match results.pop() {
            Some(result) => Ok(result),
            None => domain.normal(),
        }
    }

    fn schedule<'a, D: CompletionDomain>(
        node: &'a SemanticNode,
        domain: &D,
        tasks: &mut smallvec::SmallVec<[CompletionTask<'a>; 4]>,
        results: &mut smallvec::SmallVec<[D::State; 4]>,
    ) -> Result<(), D::Error> {
        match node {
            SemanticNode::Empty => {
                let state = domain.normal()?;
                results.push(state);
            }
            SemanticNode::BasicBlock(block) => {
                let state = if block_has_no_return_call(block) {
                    domain.no_return_call()?
                } else {
                    domain.basic_block(block)?
                };
                results.push(state);
            }
            SemanticNode::Leave(leave) => {
                let state = domain.leave(leave)?;
                results.push(state);
            }
            SemanticNode::Sequence(children) => {
                tasks.push(CompletionTask::Combine(CompletionFrame::Sequence(children)));
                tasks.extend(children.iter().rev().map(CompletionTask::Visit));
            }
            SemanticNode::If {
                condition,
                then_node,
                else_node,
            } => {
                tasks.push(CompletionTask::Combine(CompletionFrame::If {
                    condition: &condition.value,
                    has_else: else_node.is_some(),
                }));
                if let Some(else_node) = else_node {
                    tasks.push(CompletionTask::Visit(else_node));
                }
                tasks.push(CompletionTask::Visit(then_node));
            }
            SemanticNode::Loop {
                control,
                kind,
                test,
                body,
                ..
            } => {
                tasks.push(CompletionTask::Combine(CompletionFrame::Loop {
                    control: *control,
                    kind: *kind,
                    condition: &test.condition.value,
                }));
                tasks.push(CompletionTask::Visit(body));
                tasks.push(CompletionTask::Visit(&test.setup));
            }
            SemanticNode::For {
                control,
                condition,
                body,
                ..
            } => {
                tasks.push(CompletionTask::Combine(CompletionFrame::For {
                    control: *control,
                    condition: &condition.value,
                }));
                tasks.push(CompletionTask::Visit(body));
            }
            SemanticNode::ForEach { control, body, .. } => {
                tasks.push(CompletionTask::Combine(CompletionFrame::ForEach {
                    control: *control,
                }));
                tasks.push(CompletionTask::Visit(body));
            }
            SemanticNode::Switch { region, cases, .. } => {
                tasks.push(CompletionTask::Combine(CompletionFrame::Switch {
                    region: *region,
                    cases: cases.len(),
                    has_default: cases.iter().any(|case| case.is_default),
                }));
                tasks.extend(
                    cases
                        .iter()
                        .rev()
                        .map(|case| CompletionTask::Visit(&case.body)),
                );
            }
            SemanticNode::Try {
                body,
                catches,
                finally,
                ..
            } => {
                tasks.push(CompletionTask::Combine(CompletionFrame::Try {
                    catches: catches.len(),
                    has_finally: finally.is_some(),
                }));
                if let Some(finally) = finally {
                    tasks.push(CompletionTask::Visit(&finally.body));
                }
                tasks.extend(
                    catches
                        .iter()
                        .rev()
                        .map(|catch| CompletionTask::Visit(&catch.body)),
                );
                tasks.push(CompletionTask::Visit(body));
            }
            SemanticNode::Synchronized { body, .. } => {
                tasks.push(CompletionTask::Combine(CompletionFrame::Synchronized));
                tasks.push(CompletionTask::Visit(body));
            }
            SemanticNode::Label { label, body } => {
                tasks.push(CompletionTask::Combine(CompletionFrame::Label(*label)));
                tasks.push(CompletionTask::Visit(body));
            }
        }
        Ok(())
    }

    fn combine<D: CompletionDomain>(
        frame: CompletionFrame<'_>,
        results: &mut smallvec::SmallVec<[D::State; 4]>,
        domain: &D,
        sequence: &mut impl FnMut(&[SemanticNode], &[D::State]),
    ) -> Result<D::State, D::Error> {
        match frame {
            CompletionFrame::Sequence(children) => {
                let start = results
                    .len()
                    .checked_sub(children.len())
                    .expect("completion sequence result stack");
                sequence(children, &results[start..]);
                domain.sequence(results.drain(start..))
            }
            CompletionFrame::If {
                condition,
                has_else,
            } => {
                let else_state =
                    has_else.then(|| results.pop().expect("completion else result stack"));
                let then_state = results.pop().expect("completion then result stack");
                domain.branch(condition, then_state, else_state)
            }
            CompletionFrame::Loop {
                control,
                kind,
                condition,
            } => {
                let body = results.pop().expect("completion loop body result stack");
                let setup = results.pop().expect("completion loop setup result stack");
                domain.loop_node(control, kind, condition, setup, body)
            }
            CompletionFrame::For { control, condition } => {
                let body = results.pop().expect("completion for body result stack");
                domain.for_node(control, condition, body)
            }
            CompletionFrame::ForEach { control } => {
                let body = results
                    .pop()
                    .expect("completion for-each body result stack");
                domain.for_each_node(control, body)
            }
            CompletionFrame::Switch {
                region,
                cases,
                has_default,
            } => {
                let start = results
                    .len()
                    .checked_sub(cases)
                    .expect("completion switch result stack");
                domain.switch_node(region, has_default, results.drain(start..))
            }
            CompletionFrame::Try {
                catches,
                has_finally,
            } => {
                let count = 1 + catches + usize::from(has_finally);
                let start = results
                    .len()
                    .checked_sub(count)
                    .expect("completion try result stack");
                domain.try_node(catches, has_finally, results.drain(start..))
            }
            CompletionFrame::Synchronized => {
                let body = results
                    .pop()
                    .expect("completion synchronized body result stack");
                domain.synchronized(body)
            }
            CompletionFrame::Label(label) => {
                let body = results.pop().expect("completion label body result stack");
                domain.label(label, body)
            }
        }
    }
}

/// Reaching predicate for normal fallthrough through structured code.
///
/// Acyclic decisions are exact. Constructs whose finite syntax cannot encode
/// all dynamic exits conservatively return an over-approximation, so consumers
/// may lose an optimization proof but never remove a reachable path.
pub(crate) struct SemanticFallthrough;

impl SemanticFallthrough {
    pub(crate) fn analyze(root: &SemanticNode) -> Result<BoolExpr, super::SemanticFoldError> {
        CompletionInterpreter::analyze(root, &FallthroughDomain)
    }
}

struct FallthroughDomain;

impl CompletionDomain for FallthroughDomain {
    type State = BoolExpr;
    type Error = super::SemanticFoldError;

    fn normal(&self) -> Result<Self::State, Self::Error> {
        Ok(BoolExpr::True)
    }

    fn no_return_call(&self) -> Result<Self::State, Self::Error> {
        Ok(BoolExpr::False)
    }

    fn leave(&self, _leave: &SemanticLeave) -> Result<Self::State, Self::Error> {
        Ok(BoolExpr::False)
    }

    fn sequence(
        &self,
        children: impl IntoIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error> {
        Ok(BoolExpr::and(children.into_iter().collect()))
    }

    fn branch(
        &self,
        condition: &SemanticPredicate,
        then_state: Self::State,
        else_state: Option<Self::State>,
    ) -> Result<Self::State, Self::Error> {
        let condition = condition.domain()?;
        Ok(BoolExpr::or(vec![
            BoolExpr::and(vec![condition.clone(), then_state]),
            BoolExpr::and(vec![
                BoolExpr::not(condition),
                else_state.unwrap_or(BoolExpr::True),
            ]),
        ]))
    }

    fn loop_node(
        &self,
        _control: SemanticLoopControl,
        kind: SemanticLoopKind,
        _condition: &SemanticPredicate,
        setup: Self::State,
        _body: Self::State,
    ) -> Result<Self::State, Self::Error> {
        Ok(match kind {
            SemanticLoopKind::PreTested | SemanticLoopKind::Endless => setup,
            SemanticLoopKind::PostTested => BoolExpr::True,
        })
    }

    fn for_node(
        &self,
        _control: SemanticLoopControl,
        _condition: &SemanticPredicate,
        _body: Self::State,
    ) -> Result<Self::State, Self::Error> {
        Ok(BoolExpr::True)
    }

    fn for_each_node(
        &self,
        _control: SemanticLoopControl,
        _body: Self::State,
    ) -> Result<Self::State, Self::Error> {
        Ok(BoolExpr::True)
    }

    fn switch_node(
        &self,
        _region: Option<RegionId>,
        _has_default: bool,
        _cases: impl IntoIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error> {
        Ok(BoolExpr::True)
    }

    fn try_node(
        &self,
        _catches: usize,
        _has_finally: bool,
        _children: impl DoubleEndedIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error> {
        Ok(BoolExpr::True)
    }

    fn synchronized(&self, body: Self::State) -> Result<Self::State, Self::Error> {
        Ok(body)
    }

    fn label(&self, _label: SemanticLabel, _body: Self::State) -> Result<Self::State, Self::Error> {
        Ok(BoolExpr::True)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BlockId, RegionId, SemanticSwitchCase};

    fn leave(kind: SemanticLeaveKind, target: RegionId) -> SemanticNode {
        SemanticNode::Leave(SemanticLeave {
            site: None,
            condition: None,
            kind,
            edge: None,
            origin: None,
            source: target,
            destination: target,
            target,
            cleanup: Vec::new(),
        })
    }

    #[test]
    fn label_break_completes_the_owned_label() {
        let region = RegionId::new(1);
        let label = SemanticLabel::block(region, BlockId::new(2));
        let node = SemanticNode::Label {
            label,
            body: Box::new(leave(SemanticLeaveKind::BreakLabel(label), region)),
        };

        assert!(SemanticCompletion::analyze(&node).can_complete_normally());
    }

    #[test]
    fn exhaustive_terminal_switch_does_not_complete_normally() {
        let region = RegionId::new(1);
        let node = SemanticNode::Switch {
            region: Some(region),
            selector: super::super::SemanticOperand::new(crate::ir::SemanticExpression::Literal(
                crate::ir::LiteralArg::new(0, crate::ir::ArgType::INT),
            )),
            cases: vec![
                SemanticSwitchCase {
                    values: vec![0],
                    is_default: false,
                    body: leave(SemanticLeaveKind::Return(None), region),
                },
                SemanticSwitchCase {
                    values: Vec::new(),
                    is_default: true,
                    body: leave(
                        SemanticLeaveKind::Throw(crate::ir::SemanticExpression::Literal(
                            crate::ir::LiteralArg::new(0, crate::ir::ArgType::INT),
                        )),
                        region,
                    ),
                },
            ],
        };

        assert!(!SemanticCompletion::analyze(&node).can_complete_normally());
    }

    #[test]
    fn switch_without_default_keeps_the_unmatched_path() {
        let region = RegionId::new(1);
        let node = SemanticNode::Switch {
            region: Some(region),
            selector: super::super::SemanticOperand::new(crate::ir::SemanticExpression::Literal(
                crate::ir::LiteralArg::new(0, crate::ir::ArgType::INT),
            )),
            cases: vec![SemanticSwitchCase {
                values: vec![0],
                is_default: false,
                body: leave(SemanticLeaveKind::Return(None), region),
            }],
        };

        assert!(SemanticCompletion::analyze(&node).can_complete_normally());
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/control_completion.rs"]
mod control_completion;
