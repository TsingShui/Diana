//! Central traversal contracts for Semantic IR.
//!
//! Analyses and transformations customize typed callbacks instead of
//! duplicating the complete `SemanticNode` recursion in every component.

use crate::ir::RegisterArg;

use super::{
    SemanticExpression, SemanticNode, SemanticOperation, SemanticPredicate, SemanticStatement,
    SemanticStatementKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticBindingKind {
    Catch,
    ForEach,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticFoldError {
    MalformedWorkStack,
    ChildArity { expected: usize, actual: usize },
    InstructionTree(crate::ir::InstructionTreeError),
    BooleanDomain(crate::ir::bdd::BddError),
    NonOperationStatement,
    NonOperationPredicate,
    CompletionChanged { transform: &'static str },
    BlockIdentityChanged { transform: &'static str },
    CfgIdentityChanged { transform: &'static str },
    ControlTopologyChanged { transform: &'static str },
}

impl std::fmt::Display for SemanticFoldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedWorkStack => formatter.write_str("malformed semantic fold stack"),
            Self::ChildArity { expected, actual } => write!(
                formatter,
                "semantic fold frame has {actual} children, expected {expected}"
            ),
            Self::InstructionTree(source) => {
                write!(formatter, "instruction tree transform failed: {source}")
            }
            Self::BooleanDomain(source) => {
                write!(formatter, "semantic Boolean domain failed: {source}")
            }
            Self::NonOperationStatement => {
                formatter.write_str("semantic effect statement was transformed into a value")
            }
            Self::NonOperationPredicate => formatter
                .write_str("semantic predicate test was transformed into a non-operation value"),
            Self::CompletionChanged { transform } => write!(
                formatter,
                "semantic transform {transform} changed observable control completion"
            ),
            Self::BlockIdentityChanged { transform } => write!(
                formatter,
                "semantic transform {transform} changed a required CFG block identity"
            ),
            Self::CfgIdentityChanged { transform } => write!(
                formatter,
                "semantic transform {transform} changed required CFG block or edge identity"
            ),
            Self::ControlTopologyChanged { transform } => write!(
                formatter,
                "semantic transform {transform} changed the structured control topology"
            ),
        }
    }
}

impl std::error::Error for SemanticFoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InstructionTree(source) => Some(source),
            Self::BooleanDomain(source) => Some(source),
            _ => None,
        }
    }
}

impl From<crate::ir::InstructionTreeError> for SemanticFoldError {
    fn from(source: crate::ir::InstructionTreeError) -> Self {
        Self::InstructionTree(source)
    }
}

impl From<crate::ir::bdd::BddError> for SemanticFoldError {
    fn from(source: crate::ir::bdd::BddError) -> Self {
        Self::BooleanDomain(source)
    }
}

/// Controls whether an owning semantic fold descends into a node's children.
///
/// `Emit` is useful for proven subtree rewrites whose replacement is already
/// complete. It prevents transforms from rebuilding their own recursive tree
/// walkers merely to stop at an ownership boundary.
pub enum SemanticFoldControl {
    Descend(SemanticNode),
    Emit(SemanticNode),
}

pub trait SemanticVisitor {
    /// Control-only analyses inspect node entries and exits. They can omit
    /// statements, predicates, expressions, and value bindings altogether.
    fn visits_values(&self) -> bool {
        true
    }

    fn visit_node(&mut self, node: &SemanticNode) {
        walk_node(self, node);
    }

    fn enter_node(&mut self, _node: &SemanticNode) {}

    fn exit_node(&mut self, _node: &SemanticNode) {}

    fn visit_statement(&mut self, statement: &SemanticStatement) {
        walk_statement(self, statement);
    }

    fn visit_predicate(&mut self, predicate: &SemanticPredicate) {
        walk_predicate(self, predicate);
    }

    fn visit_expression(&mut self, expression: &SemanticExpression) {
        walk_expression(self, expression);
    }

    fn visit_operation(&mut self, operation: &SemanticOperation) {
        walk_operation(self, operation);
    }

    fn enter_operation(&mut self, _operation: &SemanticOperation) {}

    fn exit_operation(&mut self, _operation: &SemanticOperation) {}

    fn visit_register(&mut self, _register: &RegisterArg) {}

    fn visit_binding(&mut self, _kind: SemanticBindingKind, register: &RegisterArg) {
        self.visit_register(register);
    }
}

pub trait SemanticFolder {
    type Error: std::error::Error + From<SemanticFoldError>;

    /// True only when folding this entire subtree leaves both it and the
    /// folder state unchanged. Such children stay in their parent's storage.
    fn preserves_subtree(&self, _node: &SemanticNode) -> bool {
        false
    }

    fn enter_node(&mut self, _node: &SemanticNode) {}

    fn begin_node(&mut self, node: SemanticNode) -> Result<SemanticFoldControl, Self::Error> {
        Ok(SemanticFoldControl::Descend(node))
    }

    fn fold_node(&mut self, node: SemanticNode) -> Result<SemanticNode, Self::Error> {
        fold_node(self, node)
    }

    fn finish_node(&mut self, node: SemanticNode) -> Result<SemanticNode, Self::Error> {
        Ok(node)
    }
}

pub fn walk_node<V: SemanticVisitor + ?Sized>(visitor: &mut V, node: &SemanticNode) {
    let values = visitor.visits_values();
    let mut pending = smallvec::SmallVec::<[VisitTask; 8]>::new();
    pending.push(VisitTask::Enter(node));
    while let Some(task) = pending.pop() {
        let node = match task {
            VisitTask::Exit(node) => {
                visitor.exit_node(node);
                continue;
            }
            VisitTask::Statement(statement) => {
                visitor.visit_statement(statement);
                continue;
            }
            VisitTask::Predicate(predicate) => {
                visitor.visit_predicate(predicate);
                continue;
            }
            VisitTask::Expression(expression) => {
                visitor.visit_expression(expression);
                continue;
            }
            VisitTask::Binding(kind, register) => {
                visitor.visit_binding(kind, register);
                continue;
            }
            VisitTask::Enter(node) => node,
        };
        visitor.enter_node(node);
        if !matches!(
            node,
            SemanticNode::Empty | SemanticNode::BasicBlock(_) | SemanticNode::Leave(_)
        ) {
            pending.push(VisitTask::Exit(node));
        }
        match node {
            SemanticNode::Empty => visitor.exit_node(node),
            SemanticNode::BasicBlock(block) => {
                if values {
                    for statement in &block.statements {
                        visitor.visit_statement(statement);
                    }
                }
                visitor.exit_node(node);
            }
            SemanticNode::Sequence(children) => {
                pending.extend(children.iter().rev().map(VisitTask::Enter))
            }
            SemanticNode::If {
                condition,
                then_node,
                else_node,
            } => {
                if let Some(node) = else_node {
                    pending.push(VisitTask::Enter(node));
                }
                pending.push(VisitTask::Enter(then_node));
                if values {
                    pending.push(VisitTask::Predicate(condition));
                }
            }
            SemanticNode::Loop {
                kind, test, body, ..
            } => match kind {
                super::SemanticLoopKind::PreTested => {
                    pending.push(VisitTask::Enter(body));
                    if values {
                        pending.push(VisitTask::Predicate(&test.condition));
                    }
                    pending.push(VisitTask::Enter(&test.setup));
                }
                super::SemanticLoopKind::PostTested => {
                    if values {
                        pending.push(VisitTask::Predicate(&test.condition));
                    }
                    pending.push(VisitTask::Enter(&test.setup));
                    pending.push(VisitTask::Enter(body));
                }
                super::SemanticLoopKind::Endless => {
                    pending.push(VisitTask::Enter(body));
                    pending.push(VisitTask::Enter(&test.setup));
                }
            },
            SemanticNode::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                if values {
                    pending.push(VisitTask::Statement(update));
                }
                pending.push(VisitTask::Enter(body));
                if values {
                    pending.push(VisitTask::Predicate(condition));
                    pending.push(VisitTask::Statement(init));
                }
            }
            SemanticNode::ForEach {
                variable,
                iterable,
                body,
                ..
            } => {
                pending.push(VisitTask::Enter(body));
                if values {
                    pending.push(VisitTask::Expression(iterable));
                    pending.push(VisitTask::Binding(SemanticBindingKind::ForEach, variable));
                }
            }
            SemanticNode::Switch {
                selector, cases, ..
            } => {
                pending.extend(cases.iter().rev().map(|case| VisitTask::Enter(&case.body)));
                if values {
                    pending.push(VisitTask::Expression(selector));
                }
            }
            SemanticNode::Try {
                body,
                catches,
                finally,
                ..
            } => {
                if let Some(finally) = finally {
                    pending.push(VisitTask::Enter(&finally.body));
                }
                for catch in catches.iter().rev() {
                    pending.push(VisitTask::Enter(&catch.body));
                    if values {
                        if let Some(register) = &catch.exception_value {
                            pending.push(VisitTask::Binding(SemanticBindingKind::Catch, register));
                        }
                    }
                }
                pending.push(VisitTask::Enter(body));
            }
            SemanticNode::Synchronized { lock, body, .. } => {
                pending.push(VisitTask::Enter(body));
                if values {
                    pending.push(VisitTask::Expression(lock));
                }
            }
            SemanticNode::Label { body, .. } => {
                pending.push(VisitTask::Enter(body));
            }
            SemanticNode::Leave(leave) => {
                if values {
                    if let Some(value) = leave.value() {
                        visitor.visit_expression(value);
                    }
                    if let Some(condition) = &leave.condition {
                        visitor.visit_predicate(condition);
                    }
                }
                visitor.exit_node(node);
            }
        }
    }
}

enum VisitTask<'a> {
    Enter(&'a SemanticNode),
    Exit(&'a SemanticNode),
    Statement(&'a SemanticStatement),
    Predicate(&'a SemanticPredicate),
    Expression(&'a SemanticExpression),
    Binding(SemanticBindingKind, &'a RegisterArg),
}

/// Edit basic blocks in fold order while their enclosing control nodes
/// remain in place. Statements owned by loop headers are separate sites.
pub(super) fn for_each_block_mut(
    root: &mut SemanticNode,
    mut edit: impl FnMut(&mut super::SemanticBlock),
) {
    let mut pending = smallvec::SmallVec::<[&mut SemanticNode; 8]>::new();
    let mut current = root;
    loop {
        match current {
            SemanticNode::Empty | SemanticNode::Leave(_) => {}
            SemanticNode::BasicBlock(block) => edit(block),
            SemanticNode::Sequence(nodes) => pending.extend(nodes.iter_mut().rev()),
            SemanticNode::If {
                then_node,
                else_node,
                ..
            } => {
                pending.extend(else_node.as_deref_mut());
                current = then_node;
                continue;
            }
            SemanticNode::Loop { test, body, .. } => {
                pending.push(body);
                current = &mut test.setup;
                continue;
            }
            SemanticNode::For { body, .. }
            | SemanticNode::ForEach { body, .. }
            | SemanticNode::Synchronized { body, .. }
            | SemanticNode::Label { body, .. } => {
                current = body;
                continue;
            }
            SemanticNode::Switch { cases, .. } => {
                pending.extend(cases.iter_mut().rev().map(|case| &mut case.body));
            }
            SemanticNode::Try {
                body,
                catches,
                finally,
                ..
            } => {
                pending.extend(finally.iter_mut().map(|cleanup| cleanup.body.as_mut()));
                pending.extend(catches.iter_mut().rev().map(|catch| &mut catch.body));
                current = body;
                continue;
            }
        }
        let Some(next) = pending.pop() else { break };
        current = next;
    }
}

pub fn fold_node<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    mut node: SemanticNode,
) -> Result<SemanticNode, F::Error> {
    // A frame retains its child storage while the folder owns the child.
    // Only the active ancestor chain is on the stack, even for a wide switch.
    if folder.preserves_subtree(&node) {
        return Ok(node);
    }
    let mut parents = Vec::<(SemanticNode, usize)>::new();
    loop {
        folder.enter_node(&node);
        let mut result = match folder.begin_node(node)? {
            SemanticFoldControl::Emit(node) => node,
            SemanticFoldControl::Descend(mut parent) => {
                let mut index = 0;
                if let Some(first) = take_fold_child(folder, &mut parent, &mut index) {
                    node = first;
                    parents.push((parent, index));
                    continue;
                }
                folder.finish_node(parent)?
            }
        };
        loop {
            let Some((parent, index)) = parents.last_mut() else {
                return Ok(result);
            };
            *fold_child(parent, *index).ok_or(SemanticFoldError::MalformedWorkStack)? = result;
            *index += 1;
            if let Some(next) = take_fold_child(folder, parent, index) {
                node = next;
                break;
            }
            let (parent, _) = parents.pop().ok_or(SemanticFoldError::MalformedWorkStack)?;
            result = folder.finish_node(parent)?;
        }
    }
}

fn take_fold_child<F: SemanticFolder + ?Sized>(
    folder: &F,
    parent: &mut SemanticNode,
    index: &mut usize,
) -> Option<SemanticNode> {
    loop {
        let child = fold_child(parent, *index)?;
        if !folder.preserves_subtree(child) {
            return Some(std::mem::replace(child, SemanticNode::Empty));
        }
        *index += 1;
    }
}

/// Child order is the folder's bottom-up reconstruction order. Condition
/// expressions, catch bindings and other value slots belong to their owner.
fn fold_child(node: &mut SemanticNode, index: usize) -> Option<&mut SemanticNode> {
    match node {
        SemanticNode::Empty | SemanticNode::BasicBlock(_) | SemanticNode::Leave(_) => None,
        SemanticNode::Sequence(nodes) => nodes.get_mut(index),
        SemanticNode::If {
            then_node,
            else_node,
            ..
        } => match index {
            0 => Some(then_node),
            1 => else_node.as_deref_mut(),
            _ => None,
        },
        SemanticNode::Loop { test, body, .. } => match index {
            0 => Some(&mut test.setup),
            1 => Some(body),
            _ => None,
        },
        SemanticNode::For { body, .. }
        | SemanticNode::ForEach { body, .. }
        | SemanticNode::Synchronized { body, .. }
        | SemanticNode::Label { body, .. } => (index == 0).then_some(body.as_mut()),
        SemanticNode::Switch { cases, .. } => cases.get_mut(index).map(|case| &mut case.body),
        SemanticNode::Try {
            body,
            catches,
            finally,
            ..
        } => {
            if index == 0 {
                Some(body)
            } else if index - 1 == catches.len() {
                finally.as_mut().map(|cleanup| cleanup.body.as_mut())
            } else {
                catches.get_mut(index - 1).map(|catch| &mut catch.body)
            }
        }
    }
}

pub fn walk_statement<V: SemanticVisitor + ?Sized>(visitor: &mut V, statement: &SemanticStatement) {
    match &statement.kind {
        SemanticStatementKind::Instruction(operation) => visitor.visit_operation(operation),
        SemanticStatementKind::Definition { value, .. } => visitor.visit_expression(value),
    }
}

pub fn walk_expression<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    expression: &SemanticExpression,
) {
    let mut expression = expression;
    let mut pending = Vec::new();
    loop {
        match expression {
            SemanticExpression::Register(register) => visitor.visit_register(register),
            SemanticExpression::Literal(_) => {}
            SemanticExpression::Operation(operation) => visitor.visit_operation(operation),
            SemanticExpression::Select {
                condition,
                when_true,
                when_false,
            } => {
                visitor.visit_predicate(condition);
                pending.push(when_false.as_ref());
                expression = when_true;
                continue;
            }
        }
        let Some(next) = pending.pop() else { break };
        expression = next;
    }
}

pub fn walk_predicate<V: SemanticVisitor + ?Sized>(visitor: &mut V, predicate: &SemanticPredicate) {
    let mut predicate = predicate;
    let mut pending = Vec::new();
    loop {
        match predicate {
            SemanticPredicate::Test(operation) => visitor.visit_operation(operation),
            SemanticPredicate::Not(inner) => {
                predicate = inner;
                continue;
            }
            SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                if let Some((first, rest)) = terms.split_first() {
                    pending.extend(rest.iter().rev());
                    predicate = first;
                    continue;
                }
            }
            SemanticPredicate::True | SemanticPredicate::False => {}
        }
        let Some(next) = pending.pop() else { break };
        predicate = next;
    }
}

pub fn walk_operation<V: SemanticVisitor + ?Sized>(visitor: &mut V, operation: &SemanticOperation) {
    visitor.enter_operation(operation);
    for operand in operation.operands() {
        visitor.visit_expression(operand);
    }
    if let Some(target) = operation.compound_target() {
        visitor.visit_expression(target);
    }
    visitor.exit_operation(operation);
}
