//! Expression transformation across Semantic IR.

use crate::ir::{InsnNode, InsnType, RegisterArg, SemanticFoldError};

use super::{
    SemanticExpression, SemanticNode, SemanticOperation, SemanticPredicate, SemanticStatement,
    SemanticStatementKind,
};

/// Typed callbacks for bottom-up Semantic IR expression transformations.
pub trait SemanticExpressionTransform {
    /// Operation-only rewrites can keep registers in their original shared
    /// storage and omit all input, result and binding callbacks.
    fn visits_registers(&self) -> bool {
        true
    }

    /// For an operation with only literal or register children, the transform
    /// can keep the original shared operation when all its callbacks would be
    /// identities and none would update the transform's own state.
    fn preserves_direct_operation(&self, _operation: &SemanticOperation) -> bool {
        false
    }

    fn transform_register(&mut self, register: RegisterArg) -> SemanticExpression {
        SemanticExpression::Register(register)
    }

    fn transform_operation(&mut self, operation: SemanticOperation) -> SemanticExpression {
        SemanticExpression::Operation(operation)
    }

    fn transform_select(
        &mut self,
        condition: SemanticPredicate,
        when_true: SemanticExpression,
        when_false: SemanticExpression,
    ) -> SemanticExpression {
        SemanticExpression::select(condition, when_true, when_false)
    }

    fn transform_result(&mut self, result: RegisterArg) -> RegisterArg {
        result
    }

    fn transform_binding(&mut self, binding: RegisterArg) -> RegisterArg {
        self.transform_result(binding)
    }
}

pub struct SemanticInstructions;

impl SemanticInstructions {
    pub fn transform<T>(root: &mut SemanticNode, transform: &mut T) -> Result<(), SemanticFoldError>
    where
        T: SemanticExpressionTransform + ?Sized,
    {
        Self::transform_tree(root, transform)
    }

    fn transform_tree<T>(
        node: &mut SemanticNode,
        transform: &mut T,
    ) -> Result<(), SemanticFoldError>
    where
        T: SemanticExpressionTransform + ?Sized,
    {
        match node {
            SemanticNode::Empty | SemanticNode::BasicBlock(_) | SemanticNode::Leave(_) => {}
            SemanticNode::Sequence(children) => {
                for child in children {
                    Self::transform_tree(child, transform)?;
                }
            }
            SemanticNode::If {
                then_node,
                else_node,
                ..
            } => {
                Self::transform_tree(then_node.as_mut(), transform)?;
                if let Some(else_node) = else_node {
                    Self::transform_tree(else_node.as_mut(), transform)?;
                }
            }
            SemanticNode::Loop { test, body, .. } => {
                Self::transform_tree(test.setup.as_mut(), transform)?;
                Self::transform_tree(body.as_mut(), transform)?;
            }
            SemanticNode::For { body, .. } => Self::transform_tree(body.as_mut(), transform)?,
            SemanticNode::ForEach { body, .. } => Self::transform_tree(body.as_mut(), transform)?,
            SemanticNode::Switch { cases, .. } => {
                for case in cases {
                    Self::transform_tree(&mut case.body, transform)?;
                }
            }
            SemanticNode::Try {
                body,
                catches,
                finally,
                ..
            } => {
                Self::transform_tree(body.as_mut(), transform)?;
                for catch in catches {
                    Self::transform_tree(&mut catch.body, transform)?;
                }
                if let Some(finally) = finally {
                    Self::transform_tree(finally.body.as_mut(), transform)?;
                }
            }
            SemanticNode::Synchronized { body, .. } | SemanticNode::Label { body, .. } => {
                Self::transform_tree(body.as_mut(), transform)?;
            }
        }
        Self::transform_node(node, transform)
    }

    pub fn transform_node<T>(
        node: &mut SemanticNode,
        transform: &mut T,
    ) -> Result<(), SemanticFoldError>
    where
        T: SemanticExpressionTransform + ?Sized,
    {
        match node {
            SemanticNode::BasicBlock(block) => {
                for statement in &mut block.statements {
                    Self::transform_statement(statement, transform)?;
                }
            }
            SemanticNode::If { condition, .. } => {
                Self::transform_predicate(condition, transform)?;
            }
            SemanticNode::Loop { test, .. } => {
                Self::transform_predicate(&mut test.condition, transform)?;
            }
            SemanticNode::For {
                init,
                condition,
                update,
                ..
            } => {
                Self::transform_statement(init, transform)?;
                Self::transform_predicate(condition, transform)?;
                Self::transform_statement(update, transform)?;
            }
            SemanticNode::ForEach {
                variable, iterable, ..
            } => {
                Self::transform_register_binding(variable, transform)?;
                Self::transform_expression(iterable, transform)?;
            }
            SemanticNode::Switch { selector, .. }
            | SemanticNode::Synchronized { lock: selector, .. } => {
                Self::transform_expression(selector, transform)?;
            }
            SemanticNode::Try { catches, .. } => {
                for catch in catches {
                    if let Some(value) = &mut catch.exception_value {
                        Self::transform_register_binding(value, transform)?;
                    }
                }
            }
            SemanticNode::Leave(leave) => {
                if let Some(condition) = &mut leave.condition {
                    Self::transform_predicate(condition, transform)?;
                }
                if let Some(value) = leave.value_mut() {
                    Self::transform_expression(value, transform)?;
                }
            }
            SemanticNode::Empty | SemanticNode::Sequence(_) | SemanticNode::Label { .. } => {}
        }
        Ok(())
    }

    pub fn transform_statement<T>(
        statement: &mut SemanticStatement,
        transform: &mut T,
    ) -> Result<(), SemanticFoldError>
    where
        T: SemanticExpressionTransform + ?Sized,
    {
        match &mut statement.kind {
            SemanticStatementKind::Instruction(operation) => {
                thread_local! {
                    static PLACEHOLDER: SemanticOperation = SemanticOperation::from_parts(
                        InsnNode::new(InsnType::Nop, 0), Vec::new(), None);
                }
                let taken = std::mem::replace(operation, PLACEHOLDER.with(Clone::clone));
                let expression = Self::transform_root_operation(taken, transform)?;
                let SemanticExpression::Operation(transformed) = expression else {
                    return Err(SemanticFoldError::NonOperationStatement);
                };
                *operation = transformed;
            }
            SemanticStatementKind::Definition { result, value, .. } => {
                if transform.visits_registers() {
                    let original =
                        std::mem::replace(result, RegisterArg::new(0, crate::ir::ArgType::VOID));
                    *result = transform.transform_result(original);
                }
                Self::transform_expression(value, transform)?;
            }
        }
        Ok(())
    }

    pub fn transform_expression<T>(
        expression: &mut SemanticExpression,
        transform: &mut T,
    ) -> Result<(), SemanticFoldError>
    where
        T: SemanticExpressionTransform + ?Sized,
    {
        match expression {
            SemanticExpression::Literal(_) => return Ok(()),
            SemanticExpression::Register(_) => {
                Self::transform_leaf(expression, transform);
                return Ok(());
            }
            _ => {}
        }
        let root = Self::take_expression(expression);
        *expression = Self::transform_owned_expression(root, transform)?;
        Ok(())
    }

    fn take_expression(value: &mut SemanticExpression) -> SemanticExpression {
        std::mem::replace(
            value,
            SemanticExpression::Literal(crate::ir::LiteralArg::int(0)),
        )
    }

    fn transform_leaf<T: SemanticExpressionTransform + ?Sized>(
        value: &mut SemanticExpression,
        transform: &mut T,
    ) {
        if transform.visits_registers() && matches!(value, SemanticExpression::Register(_)) {
            let SemanticExpression::Register(register) = Self::take_expression(value) else {
                unreachable!()
            };
            *value = transform.transform_register(register);
        }
    }

    fn direct_operands(op: &SemanticOperation) -> bool {
        op.operands()
            .iter()
            .chain(op.compound_target())
            .all(|value| {
                matches!(
                    value,
                    SemanticExpression::Register(_) | SemanticExpression::Literal(_)
                )
            })
    }

    fn finish_operation<T: SemanticExpressionTransform + ?Sized>(
        mut op: SemanticOperation,
        transform: &mut T,
    ) -> SemanticExpression {
        if transform.visits_registers() && op.result.is_some() {
            op.result = op
                .result
                .take()
                .map(|result| transform.transform_result(result));
        }
        transform.transform_operation(op)
    }

    fn transform_direct_operation<T: SemanticExpressionTransform + ?Sized>(
        mut op: SemanticOperation,
        transform: &mut T,
    ) -> SemanticExpression {
        if transform.preserves_direct_operation(&op) {
            return SemanticExpression::Operation(op);
        }
        if transform.visits_registers() {
            for child in op.children_mut() {
                Self::transform_leaf(child, transform);
            }
        }
        Self::finish_operation(op, transform)
    }

    fn transform_root_operation<T: SemanticExpressionTransform + ?Sized>(
        op: SemanticOperation,
        transform: &mut T,
    ) -> Result<SemanticExpression, SemanticFoldError> {
        if Self::direct_operands(&op) {
            return Ok(Self::transform_direct_operation(op, transform));
        }
        Self::transform_owned_expression(SemanticExpression::Operation(op), transform)
    }

    fn transform_owned_expression<T>(
        root: SemanticExpression,
        transform: &mut T,
    ) -> Result<SemanticExpression, SemanticFoldError>
    where
        T: SemanticExpressionTransform + ?Sized,
    {
        let mut pending = vec![ExpressionTask::Visit(root)];
        let mut results = Vec::new();
        let mut operations = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                ExpressionTask::Visit(SemanticExpression::Register(register)) => {
                    results.push(if transform.visits_registers() {
                        transform.transform_register(register)
                    } else {
                        SemanticExpression::Register(register)
                    });
                }
                ExpressionTask::Visit(SemanticExpression::Literal(literal)) => {
                    results.push(SemanticExpression::Literal(literal));
                }
                ExpressionTask::Visit(SemanticExpression::Operation(mut operation)) => {
                    if Self::direct_operands(&operation) {
                        results.push(Self::transform_direct_operation(operation, transform));
                        continue;
                    }
                    pending.push(ExpressionTask::FinishOperation);
                    pending.extend(
                        operation
                            .children_mut()
                            .rev()
                            .map(|value| ExpressionTask::Visit(Self::take_expression(value))),
                    );
                    operations.push(operation);
                }
                ExpressionTask::Visit(SemanticExpression::Select {
                    condition,
                    when_true,
                    when_false,
                }) => {
                    pending.push(ExpressionTask::Select(condition));
                    pending.push(ExpressionTask::Visit(*when_false));
                    pending.push(ExpressionTask::Visit(*when_true));
                }
                ExpressionTask::FinishOperation => {
                    let mut operation = operations
                        .pop()
                        .ok_or(SemanticFoldError::MalformedWorkStack)?;
                    let count = operation.operands().len()
                        + usize::from(operation.compound_target().is_some());
                    let start = results
                        .len()
                        .checked_sub(count)
                        .ok_or(SemanticFoldError::MalformedWorkStack)?;
                    for (slot, child) in operation.children_mut().zip(results.drain(start..)) {
                        *slot = child;
                    }
                    results.push(Self::finish_operation(operation, transform));
                }
                ExpressionTask::Select(mut condition) => {
                    let when_false = results.pop().ok_or(SemanticFoldError::MalformedWorkStack)?;
                    let when_true = results.pop().ok_or(SemanticFoldError::MalformedWorkStack)?;
                    Self::transform_predicate(&mut condition, transform)?;
                    results.push(transform.transform_select(condition, when_true, when_false));
                }
            }
        }
        if results.len() != 1 {
            return Err(SemanticFoldError::MalformedWorkStack);
        }
        results.pop().ok_or(SemanticFoldError::MalformedWorkStack)
    }

    pub fn transform_predicate<T>(
        predicate: &mut SemanticPredicate,
        transform: &mut T,
    ) -> Result<(), SemanticFoldError>
    where
        T: SemanticExpressionTransform + ?Sized,
    {
        if matches!(
            predicate,
            SemanticPredicate::True | SemanticPredicate::False
        ) {
            return Ok(());
        }
        let root = std::mem::replace(predicate, SemanticPredicate::True);
        if let SemanticPredicate::Test(operation) = root {
            *predicate = Self::transform_test(operation, transform)?;
            return Ok(());
        }
        let mut pending = vec![PredicateTask::Visit(root)];
        let mut results = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                PredicateTask::Visit(SemanticPredicate::Test(operation)) => {
                    results.push(Self::transform_test(operation, transform)?);
                }
                PredicateTask::Visit(SemanticPredicate::Not(inner)) => {
                    pending.push(PredicateTask::Not);
                    pending.push(PredicateTask::Visit(*inner));
                }
                PredicateTask::Visit(SemanticPredicate::And(terms)) => {
                    pending.push(PredicateTask::Junction {
                        count: terms.len(),
                        conjunction: true,
                    });
                    pending.extend(terms.into_iter().rev().map(PredicateTask::Visit));
                }
                PredicateTask::Visit(SemanticPredicate::Or(terms)) => {
                    pending.push(PredicateTask::Junction {
                        count: terms.len(),
                        conjunction: false,
                    });
                    pending.extend(terms.into_iter().rev().map(PredicateTask::Visit));
                }
                PredicateTask::Visit(leaf) => results.push(leaf),
                PredicateTask::Not => {
                    let inner = results.pop().ok_or(SemanticFoldError::MalformedWorkStack)?;
                    results.push(SemanticPredicate::Not(Box::new(inner)));
                }
                PredicateTask::Junction { count, conjunction } => {
                    let start = results
                        .len()
                        .checked_sub(count)
                        .ok_or(SemanticFoldError::MalformedWorkStack)?;
                    let terms = results.drain(start..).collect();
                    results.push(if conjunction {
                        SemanticPredicate::And(terms)
                    } else {
                        SemanticPredicate::Or(terms)
                    });
                }
            }
        }
        if results.len() != 1 {
            return Err(SemanticFoldError::MalformedWorkStack);
        }
        *predicate = results.pop().ok_or(SemanticFoldError::MalformedWorkStack)?;
        Ok(())
    }

    fn transform_test<T: SemanticExpressionTransform + ?Sized>(
        operation: SemanticOperation,
        transform: &mut T,
    ) -> Result<SemanticPredicate, SemanticFoldError> {
        match Self::transform_root_operation(operation, transform)? {
            SemanticExpression::Operation(operation) => Ok(SemanticPredicate::Test(operation)),
            _ => Err(SemanticFoldError::NonOperationPredicate),
        }
    }

    fn transform_register_binding<T>(
        register: &mut RegisterArg,
        transform: &mut T,
    ) -> Result<(), SemanticFoldError>
    where
        T: SemanticExpressionTransform + ?Sized,
    {
        if transform.visits_registers() {
            let original =
                std::mem::replace(register, RegisterArg::new(0, crate::ir::ArgType::VOID));
            *register = transform.transform_binding(original);
        }
        Ok(())
    }
}

enum ExpressionTask {
    Visit(SemanticExpression),
    FinishOperation,
    Select(SemanticPredicate),
}

enum PredicateTask {
    Visit(SemanticPredicate),
    Not,
    Junction { count: usize, conjunction: bool },
}
