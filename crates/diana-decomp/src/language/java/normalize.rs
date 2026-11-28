use super::{
    JavaAssignOp, JavaAstMutator, JavaAstRewriter, JavaBinaryOp, JavaCatch, JavaExpr,
    JavaIdentifier, JavaLiteral, JavaMethodBody, JavaPrimitiveType, JavaStmt, JavaSwitchCase,
    JavaType, JavaUnaryOp,
};
use crate::ir::{ArgType, MemberReference, MethodReference};

pub trait JavaAstTransform {
    type Error;

    fn apply(&mut self, body: &mut JavaMethodBody) -> Result<bool, Self::Error>;
}

#[derive(Debug, Default)]
pub struct JavaAstNormalizer;

#[derive(Debug, Clone)]
pub struct JavaMethodCompletion {
    return_type: JavaType,
}

impl JavaMethodCompletion {
    pub fn new(return_type: JavaType) -> Self {
        Self { return_type }
    }

    fn default_value(&self) -> Option<JavaExpr> {
        let literal = match self.return_type {
            JavaType::Primitive(JavaPrimitiveType::Void) => return None,
            JavaType::Primitive(JavaPrimitiveType::Boolean) => JavaLiteral::Boolean(false),
            JavaType::Primitive(JavaPrimitiveType::Long) => JavaLiteral::Long(0),
            JavaType::Primitive(JavaPrimitiveType::Float) => JavaLiteral::Float(0.0),
            JavaType::Primitive(JavaPrimitiveType::Double) => JavaLiteral::Double(0.0),
            JavaType::Primitive(JavaPrimitiveType::Char) => JavaLiteral::Character(0),
            JavaType::Primitive(
                JavaPrimitiveType::Byte | JavaPrimitiveType::Short | JavaPrimitiveType::Int,
            ) => JavaLiteral::Integer(0),
            JavaType::Class(_) | JavaType::Variable(_) | JavaType::Array(_) => JavaLiteral::Null,
        };
        Some(JavaExpr::Literal(literal))
    }
}

impl JavaAstTransform for JavaMethodCompletion {
    type Error = std::convert::Infallible;

    fn apply(&mut self, body: &mut JavaMethodBody) -> Result<bool, Self::Error> {
        if !TerminalBranchLinearizer::can_complete_normally(&body.root) {
            return Ok(false);
        }
        let Some(value) = self.default_value() else {
            return Ok(false);
        };
        let terminal = JavaStmt::Return(Some(value));
        match &mut body.root {
            JavaStmt::Block(statements) => statements.push(terminal),
            JavaStmt::Empty => body.root = JavaStmt::Block(vec![terminal]),
            root => {
                let preceding = std::mem::replace(root, JavaStmt::Empty);
                body.root = JavaStmt::Block(vec![preceding, terminal]);
            }
        }
        Ok(true)
    }
}

/// Re-expresses DEX `return-void` exits from a class initializer as structured
/// Java control flow. Java forbids explicit returns in initializer blocks.
#[derive(Debug, Default)]
pub struct JavaInitializerExitLowering;

impl JavaAstTransform for JavaInitializerExitLowering {
    type Error = super::JavaStructuralError;

    fn apply(&mut self, body: &mut JavaMethodBody) -> Result<bool, Self::Error> {
        let JavaStmt::Block(statements) = &mut body.root else {
            return Ok(false);
        };
        let mut changed = TerminalBranchLinearizer::apply_required_void_exit(statements);
        if changed {
            if let Some(last) = statements.last_mut() {
                changed |= JavaAstNormalizer::strip_terminal_void_return(last);
            }
            if matches!(statements.last(), Some(JavaStmt::Empty)) {
                statements.pop();
            }
        }
        Ok(changed)
    }
}

/// Flattens a materially larger terminal branch in a void method by returning
/// from the smaller branch first.
#[derive(Debug, Default)]
pub struct JavaVoidTailLinearizer;

impl JavaAstTransform for JavaVoidTailLinearizer {
    type Error = std::convert::Infallible;

    fn apply(&mut self, body: &mut JavaMethodBody) -> Result<bool, Self::Error> {
        let JavaStmt::Block(statements) = &mut body.root else {
            return Ok(false);
        };
        let Some(JavaStmt::If {
            then_stmt,
            else_stmt: Some(else_stmt),
            ..
        }) = statements.last()
        else {
            return Ok(false);
        };

        let then_cost = TerminalBranchLinearizer::statement_cost(then_stmt);
        let else_cost = TerminalBranchLinearizer::statement_cost(else_stmt);
        let then_terminal = !TerminalBranchLinearizer::can_complete_normally(then_stmt);
        let else_terminal = !TerminalBranchLinearizer::can_complete_normally(else_stmt);
        let materially_unbalanced = then_cost.max(else_cost)
            >= then_cost
                .min(else_cost)
                .saturating_add(TerminalBranchLinearizer::NESTING_PENALTY);
        if !then_terminal && !else_terminal && !materially_unbalanced {
            return Ok(false);
        }

        let choose_then = if then_terminal != else_terminal {
            then_terminal
        } else {
            then_cost <= else_cost
        };
        let Some(JavaStmt::If {
            condition,
            then_stmt,
            else_stmt: Some(else_stmt),
        }) = statements.pop()
        else {
            unreachable!("void tail candidate changed before rewrite");
        };
        let (condition, early, trailing) = if choose_then {
            (condition, *then_stmt, *else_stmt)
        } else {
            (condition.negated(), *else_stmt, *then_stmt)
        };
        let early = Self::with_return(early);
        statements.push(JavaStmt::If {
            condition,
            then_stmt: Box::new(early),
            else_stmt: None,
        });
        match trailing {
            JavaStmt::Block(trailing) => statements.extend(trailing),
            JavaStmt::Empty => {}
            statement => statements.push(statement),
        }
        Ok(true)
    }
}

impl JavaVoidTailLinearizer {
    fn with_return(statement: JavaStmt) -> JavaStmt {
        if !TerminalBranchLinearizer::can_complete_normally(&statement) {
            return statement;
        }
        match statement {
            JavaStmt::Block(mut statements) => {
                statements.push(JavaStmt::Return(None));
                JavaStmt::Block(statements)
            }
            JavaStmt::Empty => JavaStmt::Block(vec![JavaStmt::Return(None)]),
            statement => JavaStmt::Block(vec![statement, JavaStmt::Return(None)]),
        }
    }
}

impl JavaAstNormalizer {
    fn normalize(root: JavaStmt) -> Result<(JavaStmt, bool), super::JavaStructuralError> {
        let mut pending = vec![SyntaxTask::Visit(root)];
        let mut results = Vec::new();
        let mut changed = false;
        while let Some(task) = pending.pop() {
            match task {
                SyntaxTask::Visit(statement) => match statement {
                    JavaStmt::Block(children) => {
                        Self::schedule_block(children, 0, &mut pending, &mut results, &mut changed);
                    }
                    JavaStmt::Labeled { label, body } => {
                        pending.push(SyntaxTask::Rebuild(SyntaxFrame::Labeled(label)));
                        pending.push(SyntaxTask::Visit(*body));
                    }
                    JavaStmt::If {
                        condition,
                        then_stmt,
                        else_stmt,
                    } => {
                        pending.push(SyntaxTask::Rebuild(SyntaxFrame::If {
                            condition,
                            has_else: else_stmt.is_some(),
                        }));
                        if let Some(else_stmt) = else_stmt {
                            pending.push(SyntaxTask::Visit(*else_stmt));
                        }
                        pending.push(SyntaxTask::Visit(*then_stmt));
                    }
                    JavaStmt::While {
                        label,
                        condition,
                        body,
                    } => {
                        pending.push(SyntaxTask::Rebuild(SyntaxFrame::While { label, condition }));
                        pending.push(SyntaxTask::Visit(*body));
                    }
                    JavaStmt::DoWhile {
                        label,
                        body,
                        condition,
                    } => {
                        pending.push(SyntaxTask::Rebuild(SyntaxFrame::DoWhile {
                            label,
                            condition,
                        }));
                        pending.push(SyntaxTask::Visit(*body));
                    }
                    JavaStmt::For {
                        label,
                        init,
                        condition,
                        update,
                        body,
                    } => {
                        pending.push(SyntaxTask::Rebuild(SyntaxFrame::For {
                            label,
                            init,
                            condition,
                            update,
                        }));
                        pending.push(SyntaxTask::Visit(*body));
                    }
                    JavaStmt::ForEach {
                        label,
                        ty,
                        variable,
                        iterable,
                        body,
                    } => {
                        pending.push(SyntaxTask::Rebuild(SyntaxFrame::ForEach {
                            label,
                            ty,
                            variable,
                            iterable,
                        }));
                        pending.push(SyntaxTask::Visit(*body));
                    }
                    JavaStmt::Switch {
                        label,
                        selector,
                        mut cases,
                    } => {
                        let bodies = cases
                            .iter_mut()
                            .map(|case| JavaStmt::Block(std::mem::take(&mut case.body)))
                            .collect::<Vec<_>>();
                        pending.push(SyntaxTask::Rebuild(SyntaxFrame::Switch {
                            label,
                            selector,
                            cases,
                        }));
                        pending.extend(bodies.into_iter().rev().map(SyntaxTask::Visit));
                    }
                    JavaStmt::Try {
                        body,
                        mut catches,
                        finally,
                    } => {
                        let has_finally = finally.is_some();
                        let mut bodies =
                            Vec::with_capacity(1 + catches.len() + usize::from(has_finally));
                        bodies.push(*body);
                        bodies.extend(
                            catches
                                .iter_mut()
                                .map(|catch| std::mem::replace(&mut catch.body, JavaStmt::Empty)),
                        );
                        if let Some(finally) = finally {
                            bodies.push(*finally);
                        }
                        pending.push(SyntaxTask::Rebuild(SyntaxFrame::Try {
                            catches,
                            has_finally,
                        }));
                        pending.extend(bodies.into_iter().rev().map(SyntaxTask::Visit));
                    }
                    JavaStmt::Synchronized { lock, body } => {
                        pending.push(SyntaxTask::Rebuild(SyntaxFrame::Synchronized(lock)));
                        pending.push(SyntaxTask::Visit(*body));
                    }
                    leaf => results.push(leaf),
                },
                SyntaxTask::Block {
                    mut children,
                    index,
                } => {
                    children[index] = results
                        .pop()
                        .ok_or(super::JavaStructuralError::MalformedWorkStack)?;
                    Self::schedule_block(
                        children,
                        index + 1,
                        &mut pending,
                        &mut results,
                        &mut changed,
                    );
                }
                SyntaxTask::Rebuild(frame) => {
                    let count = frame.child_count();
                    let start = results
                        .len()
                        .checked_sub(count)
                        .ok_or(super::JavaStructuralError::MalformedWorkStack)?;
                    let (statement, local_change) = frame.rebuild(results.drain(start..))?;
                    changed |= local_change;
                    results.push(statement);
                }
            }
        }
        if results.len() != 1 {
            return Err(super::JavaStructuralError::MalformedWorkStack);
        }
        let root = results
            .pop()
            .ok_or(super::JavaStructuralError::MalformedWorkStack)?;
        Ok((root, changed))
    }

    fn schedule_block(
        mut children: Vec<JavaStmt>,
        start: usize,
        pending: &mut Vec<SyntaxTask>,
        results: &mut Vec<JavaStmt>,
        changed: &mut bool,
    ) {
        for index in start..children.len() {
            let child = &mut children[index];
            match child {
                JavaStmt::Empty
                | JavaStmt::RecoveryFailure(_)
                | JavaStmt::RecoveryGap(_)
                | JavaStmt::Return(_)
                | JavaStmt::Variable { .. }
                | JavaStmt::ConstructorInvocation { .. }
                | JavaStmt::Expression(_)
                | JavaStmt::Assign { .. }
                | JavaStmt::Throw(_)
                | JavaStmt::Break(_)
                | JavaStmt::Continue(_) => continue,
                _ => {
                    let child = std::mem::replace(child, JavaStmt::Empty);
                    pending.push(SyntaxTask::Block { children, index });
                    pending.push(SyntaxTask::Visit(child));
                    return;
                }
            }
        }
        let (children, local_change) = Self::flatten(children);
        *changed |= local_change;
        results.push(JavaStmt::Block(children));
    }

    fn flatten(mut flattened: Vec<JavaStmt>) -> (Vec<JavaStmt>, bool) {
        let nested = flattened
            .iter()
            .any(|statement| matches!(statement, JavaStmt::Block(_)));
        let mut changed = nested;
        if nested {
            let mut statements = Vec::with_capacity(flattened.len());
            for child in flattened {
                match child {
                    JavaStmt::Block(children) => statements.extend(children),
                    JavaStmt::Empty => {}
                    child => statements.push(child),
                }
            }
            flattened = statements;
        }
        let before = flattened.len();
        let mut has_if = false;
        let mut has_switch = false;
        flattened.retain(|statement| match statement {
            JavaStmt::Empty => nested,
            JavaStmt::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                let keep = else_stmt.is_some()
                    || !Self::is_empty(then_stmt)
                    || !Self::pure_condition(condition);
                has_if |= keep;
                keep
            }
            JavaStmt::Switch { .. } => {
                has_switch = true;
                true
            }
            _ => true,
        });
        changed |= flattened.len() != before;
        if has_switch {
            changed |= StringSwitchRecovery::apply(&mut flattened);
        }
        if has_if {
            changed |= ConditionalResultJoin::apply(&mut flattened);
            changed |= TerminalBranchLinearizer::join_boolean_guards(&mut flattened);
        }
        if has_switch {
            changed |= SwitchResultReturn::apply(&mut flattened);
        }
        if has_if {
            changed |= TerminalBranchLinearizer::apply(&mut flattened);
        }
        (flattened, changed)
    }

    fn protection(
        body: JavaStmt,
        catches: Vec<JavaCatch>,
        finally: Option<Box<JavaStmt>>,
    ) -> (JavaStmt, bool) {
        let empty_finally = finally.as_deref().is_some_and(|finally| match finally {
            JavaStmt::Empty => true,
            JavaStmt::Block(statements) => statements.is_empty(),
            _ => false,
        });
        if catches.is_empty() && empty_finally {
            return (body, true);
        }
        let empty_body = match &body {
            JavaStmt::Empty => true,
            JavaStmt::Block(statements) => statements.is_empty(),
            _ => false,
        };
        if empty_body && finally.is_none() {
            return (JavaStmt::Empty, true);
        }
        match (catches.is_empty() && finally.is_some(), body) {
            (
                true,
                JavaStmt::Try {
                    body,
                    catches,
                    finally: None,
                },
            ) => (
                JavaStmt::Try {
                    body,
                    catches,
                    finally,
                },
                true,
            ),
            (_, body) => (
                JavaStmt::Try {
                    body: Box::new(body),
                    catches,
                    finally,
                },
                false,
            ),
        }
    }

    fn foreach(
        label: Option<JavaIdentifier>,
        ty: JavaType,
        variable: JavaIdentifier,
        iterable: JavaExpr,
        body: JavaStmt,
    ) -> (JavaStmt, bool) {
        let JavaStmt::Block(mut statements) = body else {
            return (
                JavaStmt::ForEach {
                    label,
                    ty,
                    variable,
                    iterable,
                    body: Box::new(body),
                },
                false,
            );
        };
        let Some(JavaStmt::Variable {
            ty: target_type,
            name: target,
            value:
                Some(JavaExpr::Cast {
                    ty: cast_type,
                    value,
                }),
        }) = statements.first()
        else {
            return (
                JavaStmt::ForEach {
                    label,
                    ty,
                    variable,
                    iterable,
                    body: Box::new(JavaStmt::Block(statements)),
                },
                false,
            );
        };
        if target_type != cast_type
            || !matches!(value.as_ref(), JavaExpr::Name(source) if source == &variable)
        {
            return (
                JavaStmt::ForEach {
                    label,
                    ty,
                    variable,
                    iterable,
                    body: Box::new(JavaStmt::Block(statements)),
                },
                false,
            );
        }
        let mut uses = NameUseCounter {
            target: &variable,
            count: 0,
        };
        for statement in statements.iter().skip(1).cloned() {
            uses.rewrite_statement(statement);
        }
        if uses.count != 0 {
            return (
                JavaStmt::ForEach {
                    label,
                    ty,
                    variable,
                    iterable,
                    body: Box::new(JavaStmt::Block(statements)),
                },
                false,
            );
        }
        let target_type = target_type.clone();
        let target = target.clone();
        statements.remove(0);
        (
            JavaStmt::ForEach {
                label,
                ty: target_type,
                variable: target,
                iterable,
                body: Box::new(JavaStmt::Block(statements)),
            },
            true,
        )
    }

    fn conditional(
        condition: JavaExpr,
        then_stmt: JavaStmt,
        else_stmt: Option<JavaStmt>,
    ) -> (JavaStmt, bool) {
        if let Some(else_stmt) = else_stmt {
            let then_empty = Self::is_empty(&then_stmt);
            let else_empty = Self::is_empty(&else_stmt);
            if then_empty && !else_empty {
                return (
                    JavaStmt::If {
                        condition: condition.negated(),
                        then_stmt: Box::new(else_stmt),
                        else_stmt: None,
                    },
                    true,
                );
            }
            if !then_empty && else_empty {
                return (
                    JavaStmt::If {
                        condition,
                        then_stmt: Box::new(then_stmt),
                        else_stmt: None,
                    },
                    true,
                );
            }
            return (
                JavaStmt::If {
                    condition,
                    then_stmt: Box::new(then_stmt),
                    else_stmt: Some(Box::new(else_stmt)),
                },
                false,
            );
        }
        let then_stmt = match then_stmt {
            JavaStmt::Block(mut statements)
                if matches!(
                    statements.as_slice(),
                    [JavaStmt::If {
                        else_stmt: None,
                        ..
                    }]
                ) =>
            {
                statements.pop().expect("single nested conditional")
            }
            statement => statement,
        };
        match then_stmt {
            JavaStmt::If {
                condition: nested,
                then_stmt,
                else_stmt: None,
            } => (
                JavaStmt::If {
                    condition: JavaExpr::Binary {
                        left: Box::new(condition),
                        op: JavaBinaryOp::LogicalAnd,
                        right: Box::new(nested),
                    },
                    then_stmt,
                    else_stmt: None,
                },
                true,
            ),
            statement => (
                JavaStmt::If {
                    condition,
                    then_stmt: Box::new(statement),
                    else_stmt: None,
                },
                false,
            ),
        }
    }

    fn is_empty(statement: &JavaStmt) -> bool {
        matches!(statement, JavaStmt::Empty)
            || matches!(statement, JavaStmt::Block(statements) if statements.is_empty())
    }

    fn pure_condition(expression: &JavaExpr) -> bool {
        match expression {
            JavaExpr::Allocate(_)
            | JavaExpr::DirectInit { .. }
            | JavaExpr::Unavailable { .. }
            | JavaExpr::RecordOperation { .. } => false,
            JavaExpr::This
            | JavaExpr::QualifiedThis(_)
            | JavaExpr::Super
            | JavaExpr::Name(_)
            | JavaExpr::Literal(_)
            | JavaExpr::ClassLiteral(_) => true,
            JavaExpr::Unary {
                op: JavaUnaryOp::LogicalNot,
                operand,
            }
            | JavaExpr::InstanceOf { value: operand, .. } => Self::pure_condition(operand),
            JavaExpr::Binary { left, op, right } => {
                !matches!(op, JavaBinaryOp::Divide | JavaBinaryOp::Remainder)
                    && Self::pure_condition(left)
                    && Self::pure_condition(right)
            }
            JavaExpr::Conditional {
                condition,
                when_true,
                when_false,
            } => {
                Self::pure_condition(condition)
                    && Self::pure_condition(when_true)
                    && Self::pure_condition(when_false)
            }
            JavaExpr::Cast { .. }
            | JavaExpr::Field { .. }
            | JavaExpr::ArrayAccess { .. }
            | JavaExpr::Call { .. }
            | JavaExpr::MethodReference { .. }
            | JavaExpr::Lambda { .. }
            | JavaExpr::BlockLambda { .. }
            | JavaExpr::New { .. }
            | JavaExpr::NewArray { .. }
            | JavaExpr::StaticField { .. }
            | JavaExpr::Update { .. }
            | JavaExpr::Assignment { .. }
            | JavaExpr::Unary { .. } => false,
        }
    }

    fn strip_terminal_void_return(statement: &mut JavaStmt) -> bool {
        match statement {
            JavaStmt::Return(None) => {
                *statement = JavaStmt::Empty;
                true
            }
            JavaStmt::Block(statements) => {
                let changed = statements
                    .last_mut()
                    .is_some_and(Self::strip_terminal_void_return);
                if matches!(statements.last(), Some(JavaStmt::Empty)) {
                    statements.pop();
                }
                changed
            }
            JavaStmt::Synchronized { body, .. } => Self::strip_terminal_void_return(body),
            _ => false,
        }
    }
}

/// Joins the two complementary uses of a synthetic boolean result.
///
/// Value recovery can spell a short-circuit branch as:
/// `boolean c = false; if (a) { c = b; if (c) x; } if (!a || !c) y;`.
/// When `a` is stable across the sequence, this is exactly
/// `if (a && b) x; else y;`. Keeping the rewrite narrow is important because
/// calls and field reads in `a` may intentionally be evaluated twice.
struct ConditionalResultJoin;

impl ConditionalResultJoin {
    fn apply(statements: &mut Vec<JavaStmt>) -> bool {
        let mut changed = false;
        let mut index = 0usize;
        while index + 2 < statements.len() {
            let Some(replacement) = Self::candidate(&statements[index..index + 3]) else {
                index += 1;
                continue;
            };
            statements.splice(index..index + 3, std::iter::once(replacement));
            changed = true;
            index += 1;
        }
        changed
    }

    fn candidate(window: &[JavaStmt]) -> Option<JavaStmt> {
        let [JavaStmt::Variable {
            ty: JavaType::Primitive(JavaPrimitiveType::Boolean),
            name,
            value: Some(JavaExpr::Literal(JavaLiteral::Boolean(false))),
        }, JavaStmt::If {
            condition: guard,
            then_stmt: guarded,
            else_stmt: None,
        }, JavaStmt::If {
            condition: complement,
            then_stmt: fallback,
            else_stmt: None,
        }] = window
        else {
            return None;
        };
        let guarded = Self::statements(guarded)?;
        let [JavaStmt::Assign {
            target: JavaExpr::Name(assigned),
            op: JavaAssignOp::Assign,
            value,
        }, JavaStmt::If {
            condition: JavaExpr::Name(tested),
            then_stmt: success,
            else_stmt: None,
        }] = guarded
        else {
            return None;
        };
        if assigned != name
            || tested != name
            || !Self::is_complement(complement, guard, name)
            || !Self::stable(guard)
            || SwitchResultReturn::expression_uses(value, name)
            || SwitchResultReturn::statement_uses(success, name)
            || SwitchResultReturn::statement_uses(fallback, name)
        {
            return None;
        }

        let mut guard_names = std::collections::BTreeSet::new();
        Self::collect_names(guard, &mut guard_names);
        let mut writes = NameWriteDetector {
            targets: &guard_names,
            found: false,
        };
        writes.rewrite_expression(value.clone());
        writes.rewrite_statement(success.as_ref().clone());
        if writes.found {
            return None;
        }

        Some(JavaStmt::If {
            condition: JavaExpr::Binary {
                left: Box::new(guard.clone()),
                op: JavaBinaryOp::LogicalAnd,
                right: Box::new(value.clone()),
            },
            then_stmt: Box::new(success.as_ref().clone()),
            else_stmt: Some(Box::new(fallback.as_ref().clone())),
        })
    }

    fn statements(statement: &JavaStmt) -> Option<&[JavaStmt]> {
        match statement {
            JavaStmt::Block(statements) => Some(statements),
            _ => None,
        }
    }

    fn is_complement(expression: &JavaExpr, guard: &JavaExpr, result: &JavaIdentifier) -> bool {
        let JavaExpr::Binary {
            left,
            op: JavaBinaryOp::LogicalOr,
            right,
        } = expression
        else {
            return false;
        };
        let not_guard = guard.clone().negated();
        (left.as_ref() == &not_guard && Self::is_negated_name(right, result))
            || (right.as_ref() == &not_guard && Self::is_negated_name(left, result))
    }

    fn is_negated_name(expression: &JavaExpr, target: &JavaIdentifier) -> bool {
        matches!(
            expression,
            JavaExpr::Unary {
                op: JavaUnaryOp::LogicalNot,
                operand,
            } if matches!(operand.as_ref(), JavaExpr::Name(name) if name == target)
        )
    }

    fn stable(expression: &JavaExpr) -> bool {
        let mut pending = vec![expression];
        while let Some(expression) = pending.pop() {
            match expression {
                JavaExpr::Allocate(_)
                | JavaExpr::DirectInit { .. }
                | JavaExpr::Unavailable { .. }
                | JavaExpr::RecordOperation { .. } => return false,
                JavaExpr::This
                | JavaExpr::QualifiedThis(_)
                | JavaExpr::Super
                | JavaExpr::Name(_)
                | JavaExpr::Literal(_)
                | JavaExpr::ClassLiteral(_) => {}
                JavaExpr::Unary { operand, .. }
                | JavaExpr::Cast { value: operand, .. }
                | JavaExpr::InstanceOf { value: operand, .. } => pending.push(operand),
                JavaExpr::Binary { left, right, .. } => {
                    pending.extend([left.as_ref(), right.as_ref()]);
                }
                JavaExpr::Conditional {
                    condition,
                    when_true,
                    when_false,
                } => pending.extend([condition.as_ref(), when_true.as_ref(), when_false.as_ref()]),
                JavaExpr::Field { .. }
                | JavaExpr::StaticField { .. }
                | JavaExpr::ArrayAccess { .. }
                | JavaExpr::Call { .. }
                | JavaExpr::MethodReference { .. }
                | JavaExpr::Lambda { .. }
                | JavaExpr::BlockLambda { .. }
                | JavaExpr::New { .. }
                | JavaExpr::NewArray { .. }
                | JavaExpr::Update { .. }
                | JavaExpr::Assignment { .. } => return false,
            }
        }
        true
    }

    fn collect_names(
        expression: &JavaExpr,
        names: &mut std::collections::BTreeSet<JavaIdentifier>,
    ) {
        struct Collector<'a>(&'a mut std::collections::BTreeSet<JavaIdentifier>);
        impl JavaAstRewriter for Collector<'_> {
            fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
                if let JavaExpr::Name(name) = &expression {
                    self.0.insert(name.clone());
                }
                expression
            }
        }
        Collector(names).rewrite_expression(expression.clone());
    }
}

struct NameWriteDetector<'a> {
    targets: &'a std::collections::BTreeSet<JavaIdentifier>,
    found: bool,
}

impl JavaAstRewriter for NameWriteDetector<'_> {
    fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        let target = match &expression {
            JavaExpr::Update { target, .. } | JavaExpr::Assignment { target, .. } => {
                Some(target.as_ref())
            }
            _ => None,
        };
        if matches!(target, Some(JavaExpr::Name(name)) if self.targets.contains(name)) {
            self.found = true;
        }
        expression
    }

    fn finish_statement(&mut self, statement: JavaStmt) -> JavaStmt {
        let target = match &statement {
            JavaStmt::Variable { name, .. } => Some(name),
            JavaStmt::Assign {
                target: JavaExpr::Name(name),
                ..
            } => Some(name),
            _ => None,
        };
        if target.is_some_and(|name| self.targets.contains(name)) {
            self.found = true;
        }
        statement
    }
}

/// Reconstructs the two-switch lowering used for Java string switches.
///
/// `javac`/D8 first switches on `value.hashCode()`, stores a synthetic integer
/// selector after an `equals` check, and then switches on that selector.  The
/// source form is both shorter and single-evaluates the string selector.  This
/// recovery verifies the original String calls, each hash and each fallback
/// assignment in the canonical one-string-per-hash shape.
struct StringSwitchRecovery;

impl StringSwitchRecovery {
    fn apply(statements: &mut Vec<JavaStmt>) -> bool {
        let mut changed = false;
        let mut index = 0usize;
        while index + 2 < statements.len() {
            let Some((synthetic, replacement)) = Self::candidate(&statements[index..index + 3])
            else {
                index += 1;
                continue;
            };
            let used_outside = statements[..index]
                .iter()
                .chain(&statements[index + 3..])
                .any(|statement| SwitchResultReturn::statement_uses(statement, &synthetic));
            if used_outside {
                index += 1;
                continue;
            }
            statements.splice(index..index + 3, std::iter::once(replacement));
            changed = true;
            index += 1;
        }
        changed
    }

    fn candidate(window: &[JavaStmt]) -> Option<(JavaIdentifier, JavaStmt)> {
        let [JavaStmt::Variable {
            ty: JavaType::Primitive(JavaPrimitiveType::Int),
            name: synthetic,
            value: Some(JavaExpr::Literal(JavaLiteral::Integer(initial))),
        }, JavaStmt::Switch {
            label: None,
            selector: hash_selector,
            cases: hash_cases,
        }, JavaStmt::Switch {
            label: None,
            selector: JavaExpr::Name(action_selector),
            cases: action_cases,
        }] = window
        else {
            return None;
        };
        if action_selector != synthetic {
            return None;
        }
        let source = Self::hash_receiver(hash_selector)?;
        if hash_cases.iter().filter(|case| case.is_default).count() != 1 {
            return None;
        }
        let default_case = hash_cases.iter().find(|case| case.is_default)?;
        if !default_case.labels.is_empty() {
            return None;
        }
        let fallback = Self::fallback_selector(&default_case.body, synthetic)?;

        let mut mappings = Vec::new();
        for case in hash_cases.iter().filter(|case| !case.is_default) {
            let [JavaExpr::Literal(JavaLiteral::Integer(hash))] = case.labels.as_slice() else {
                return None;
            };
            let (literal, selected, rejected) =
                Self::hash_case(&case.body, synthetic, source, *initial)?;
            if rejected != fallback
                || Self::string_hash(literal.as_utf16()) != *hash
                || selected == fallback
                || mappings
                    .iter()
                    .any(|(existing, _): &(i32, JavaLiteral)| existing == &selected)
            {
                return None;
            }
            mappings.push((selected, JavaLiteral::String(literal.clone())));
        }
        if mappings.is_empty() {
            return None;
        }

        let mut recovered_cases = Vec::with_capacity(action_cases.len());
        for case in action_cases {
            if case
                .body
                .iter()
                .any(|statement| SwitchResultReturn::statement_uses(statement, synthetic))
            {
                return None;
            }
            if case.is_default {
                if !case.labels.is_empty() {
                    return None;
                }
                recovered_cases.push(case.clone());
                continue;
            }
            let mut labels = Vec::new();
            for label in &case.labels {
                let JavaExpr::Literal(JavaLiteral::Integer(selected)) = label else {
                    return None;
                };
                let (_, literal) = mappings
                    .iter()
                    .find(|(candidate, _)| candidate == selected)?;
                labels.push(JavaExpr::Literal(literal.clone()));
            }
            if labels.is_empty() {
                return None;
            }
            recovered_cases.push(JavaSwitchCase {
                labels,
                body: case.body.clone(),
                is_default: false,
            });
        }

        Some((
            synthetic.clone(),
            JavaStmt::Switch {
                label: None,
                selector: JavaExpr::Name(source.clone()),
                cases: recovered_cases,
            },
        ))
    }

    fn hash_receiver(selector: &JavaExpr) -> Option<&JavaIdentifier> {
        let JavaExpr::Call {
            receiver: Some(receiver),
            owner: None,
            virtual_target: Some(target),
            type_arguments,
            method,
            args,
        } = selector
        else {
            return None;
        };
        if !type_arguments.is_empty() || method.as_str() != "hashCode" || !args.is_empty() {
            return None;
        }
        let target = Self::string_method(target)?;
        if target.name != "hashCode"
            || !target.descriptor.parameters.is_empty()
            || target.descriptor.return_type != ArgType::INT
        {
            return None;
        }
        let JavaExpr::Name(source) = receiver.as_ref() else {
            return None;
        };
        Some(source)
    }

    fn hash_case<'a>(
        body: &'a [JavaStmt],
        synthetic: &JavaIdentifier,
        source: &JavaIdentifier,
        initial: i32,
    ) -> Option<(&'a crate::ir::Utf16String, i32, i32)> {
        let [JavaStmt::If {
            condition,
            then_stmt,
            else_stmt: None,
        }, JavaStmt::Assign {
            target: JavaExpr::Name(rejected_target),
            op: JavaAssignOp::Assign,
            value: JavaExpr::Literal(JavaLiteral::Integer(rejected)),
        }, JavaStmt::Break(None)] = body
        else {
            return None;
        };
        if rejected_target != synthetic {
            return None;
        }
        let literal = Self::equals_literal(condition, source)?;
        let selected = Self::selected_value(then_stmt, synthetic, initial)?;
        Some((literal, selected, *rejected))
    }

    fn equals_literal<'a>(
        condition: &'a JavaExpr,
        source: &JavaIdentifier,
    ) -> Option<&'a crate::ir::Utf16String> {
        let JavaExpr::Call {
            receiver: Some(receiver),
            owner: None,
            virtual_target: Some(target),
            type_arguments,
            method,
            args,
        } = condition
        else {
            return None;
        };
        let [JavaExpr::Literal(JavaLiteral::String(literal))] = args.as_slice() else {
            return None;
        };
        let target = Self::string_method(target)?;
        (type_arguments.is_empty()
            && method.as_str() == "equals"
            && target.name == "equals"
            && target.descriptor.return_type == ArgType::BOOLEAN
            && matches!(target.descriptor.parameters.as_slice(), [parameter] if parameter.as_object() == Some("java/lang/Object"))
            && matches!(receiver.as_ref(), JavaExpr::Name(candidate) if candidate == source))
        .then_some(literal)
    }

    fn string_method(target: &MemberReference) -> Option<&MethodReference> {
        let MemberReference::Method(method) = target else {
            return None;
        };
        (method.owner.as_object() == Some("java/lang/String")).then_some(method)
    }

    fn selected_value(
        statement: &JavaStmt,
        synthetic: &JavaIdentifier,
        initial: i32,
    ) -> Option<i32> {
        let statements = match statement {
            JavaStmt::Block(statements) => statements.as_slice(),
            JavaStmt::Break(None) => return Some(initial),
            _ => return None,
        };
        match statements {
            [JavaStmt::Break(None)] => Some(initial),
            [JavaStmt::Assign {
                target: JavaExpr::Name(target),
                op: JavaAssignOp::Assign,
                value: JavaExpr::Literal(JavaLiteral::Integer(value)),
            }, JavaStmt::Break(None)]
                if target == synthetic =>
            {
                Some(*value)
            }
            _ => None,
        }
    }

    fn fallback_selector(body: &[JavaStmt], synthetic: &JavaIdentifier) -> Option<i32> {
        let [JavaStmt::Assign {
            target: JavaExpr::Name(target),
            op: JavaAssignOp::Assign,
            value: JavaExpr::Literal(JavaLiteral::Integer(value)),
        }, JavaStmt::Break(None)] = body
        else {
            return None;
        };
        (target == synthetic).then_some(*value)
    }

    fn string_hash(units: &[u16]) -> i32 {
        units.iter().fold(0i32, |hash, unit| {
            hash.wrapping_mul(31).wrapping_add(i32::from(*unit))
        })
    }
}

/// Replaces a synthetic result local around a total switch with direct returns.
///
/// DEX has no expression-valued switch, so value recovery can naturally produce
/// `T result = null; switch (...) { case ...: result = value; break; } return result;`.
/// Once every switch entry is known to terminate, keeping the accumulator only
/// obscures the original source shape. This rewrite is deliberately conservative:
/// it requires a side-effect-free initializer, a default case, and no other use
/// of the local.
struct SwitchResultReturn;

impl SwitchResultReturn {
    fn apply(statements: &mut Vec<JavaStmt>) -> bool {
        let mut changed = false;
        let mut index = 0usize;
        while index + 2 < statements.len() {
            let Some(replacement) = Self::candidate(&statements[index..index + 3]) else {
                index += 1;
                continue;
            };
            statements.splice(index..index + 3, std::iter::once(replacement));
            changed = true;
            index += 1;
        }
        changed
    }

    fn candidate(window: &[JavaStmt]) -> Option<JavaStmt> {
        let [JavaStmt::Variable {
            name,
            value: initializer,
            ..
        }, JavaStmt::Switch {
            label: None,
            selector,
            cases,
        }, terminal] = window
        else {
            return None;
        };
        let projection = match terminal {
            JavaStmt::Return(Some(JavaExpr::Name(returned))) if returned == name => None,
            JavaStmt::Return(Some(JavaExpr::Cast { ty, value })) if matches!(value.as_ref(), JavaExpr::Name(returned) if returned == name) => {
                Some(ty)
            }
            _ => return None,
        };
        if !initializer
            .as_ref()
            .is_none_or(|value| matches!(value, JavaExpr::Literal(_)))
            || !cases.iter().any(|case| case.is_default)
            || Self::expression_uses(selector, name)
        {
            return None;
        }

        let mut cases = cases.clone();
        for case in &mut cases {
            if case
                .labels
                .iter()
                .any(|label| Self::expression_uses(label, name))
                || !Self::rewrite_case(&mut case.body, name, projection)
            {
                return None;
            }
        }
        if TerminalBranchLinearizer::switch_can_complete_normally(None, &cases) {
            return None;
        }
        Some(JavaStmt::Switch {
            label: None,
            selector: selector.clone(),
            cases,
        })
    }

    fn rewrite_case(
        body: &mut Vec<JavaStmt>,
        target: &JavaIdentifier,
        projection: Option<&JavaType>,
    ) -> bool {
        if body.is_empty() {
            // An empty case falls through to the next case. The final completion
            // check below proves that it still reaches a terminal case body.
            return true;
        }

        if matches!(body.last(), Some(JavaStmt::Return(_) | JavaStmt::Throw(_))) {
            return !body
                .iter()
                .any(|statement| Self::statement_uses(statement, target));
        }

        let Some(JavaStmt::Break(None)) = body.last() else {
            return false;
        };
        let Some(JavaStmt::Assign {
            target: JavaExpr::Name(assigned),
            op: JavaAssignOp::Assign,
            value,
        }) = body.get(body.len().saturating_sub(2))
        else {
            return false;
        };
        if assigned != target
            || Self::expression_uses(value, target)
            || body[..body.len() - 2]
                .iter()
                .any(|statement| Self::statement_uses(statement, target))
        {
            return false;
        }

        let value = match projection {
            Some(ty) if !matches!(value, JavaExpr::Cast { ty: cast, .. } if cast == ty) => {
                JavaExpr::Cast {
                    ty: ty.clone(),
                    value: Box::new(value.clone()),
                }
            }
            _ => value.clone(),
        };
        body.truncate(body.len() - 2);
        body.push(JavaStmt::Return(Some(value)));
        true
    }

    fn expression_uses(expression: &JavaExpr, target: &JavaIdentifier) -> bool {
        let mut counter = NameUseCounter { target, count: 0 };
        counter.rewrite_expression(expression.clone());
        counter.count != 0
    }

    fn statement_uses(statement: &JavaStmt, target: &JavaIdentifier) -> bool {
        let mut counter = NameUseCounter { target, count: 0 };
        counter.rewrite_statement(statement.clone());
        counter.count != 0
    }
}

struct TerminalBranchLinearizer;

pub(super) fn statements_can_complete_normally(statements: &[JavaStmt]) -> bool {
    TerminalBranchLinearizer::sequence_can_complete_normally(statements)
}

impl TerminalBranchLinearizer {
    const NESTING_PENALTY: usize = 12;

    fn join_boolean_guards(statements: &mut Vec<JavaStmt>) -> bool {
        let same_return = |left: &JavaStmt, right: &JavaStmt| {
            Self::guard_return(left).is_some_and(|value| Self::guard_return(right) == Some(value))
        };
        let Some(first) = statements
            .windows(2)
            .position(|pair| same_return(&pair[0], &pair[1]))
        else {
            return false;
        };
        let mut tail = statements.split_off(first).into_iter().peekable();
        while let Some(statement) = tail.next() {
            if !tail
                .peek()
                .is_some_and(|next| same_return(&statement, next))
            {
                statements.push(statement);
                continue;
            }
            let JavaStmt::If {
                condition,
                then_stmt,
                ..
            } = statement
            else {
                unreachable!("the boolean guard was selected above");
            };
            let value = Self::boolean_return(&then_stmt);
            let mut operands = vec![condition];
            while tail
                .peek()
                .is_some_and(|next| Self::guard_return(next) == value)
            {
                let Some(JavaStmt::If { condition, .. }) = tail.next() else {
                    unreachable!("the next boolean guard was selected above");
                };
                operands.push(condition);
            }
            let count = operands.len();
            // A true left guard used to return before evaluating the right
            // guard. Logical OR keeps that short-circuit boundary, including
            // effects and exceptions in either condition.
            statements.push(JavaStmt::If {
                condition: Self::ordered_disjunction(&mut operands.into_iter(), count),
                then_stmt,
                else_stmt: None,
            });
        }
        true
    }

    // Keep long adjacent runs shallow while preserving operand order.
    fn ordered_disjunction(
        operands: &mut impl Iterator<Item = JavaExpr>,
        count: usize,
    ) -> JavaExpr {
        if count == 1 {
            return operands.next().expect("one disjunction operand");
        }
        let left = count / 2;
        JavaExpr::Binary {
            left: Box::new(Self::ordered_disjunction(operands, left)),
            op: JavaBinaryOp::LogicalOr,
            right: Box::new(Self::ordered_disjunction(operands, count - left)),
        }
    }

    fn guard_return(statement: &JavaStmt) -> Option<bool> {
        let JavaStmt::If {
            then_stmt,
            else_stmt: None,
            ..
        } = statement
        else {
            return None;
        };
        Self::boolean_return(then_stmt)
    }

    fn boolean_return(statement: &JavaStmt) -> Option<bool> {
        let body = match statement {
            JavaStmt::Block(statements) => match statements.as_slice() {
                [only] => only,
                _ => return None,
            },
            body => body,
        };
        match body {
            JavaStmt::Return(Some(JavaExpr::Literal(JavaLiteral::Boolean(value)))) => Some(*value),
            _ => None,
        }
    }

    fn apply(statements: &mut Vec<JavaStmt>) -> bool {
        Self::linearize_candidates(statements, Self::next_candidate)
    }

    fn apply_required_void_exit(statements: &mut Vec<JavaStmt>) -> bool {
        Self::linearize_candidates(statements, Self::next_required_void_exit)
    }

    fn linearize_candidates(
        statements: &mut Vec<JavaStmt>,
        next: impl Fn(&[JavaStmt], usize) -> Option<(usize, TerminalBranchCandidate)>,
    ) -> bool {
        let mut changed = false;
        let mut start = 0;
        while let Some((index, candidate)) = next(statements, start) {
            Self::linearize(statements, index, candidate);
            start = index + 1;
            changed = true;
        }
        changed
    }

    fn guards(
        statements: &[JavaStmt],
        start: usize,
    ) -> impl Iterator<Item = (usize, &JavaExpr, &JavaStmt)> {
        statements
            .iter()
            .enumerate()
            .take(statements.len().saturating_sub(1))
            .skip(start)
            .filter_map(|(index, stmt)| match stmt {
                JavaStmt::If {
                    condition,
                    then_stmt,
                    else_stmt: None,
                } => Some((index, condition, then_stmt.as_ref())),
                _ => None,
            })
    }

    fn last_exit_after(statements: &[JavaStmt], index: usize) -> Option<usize> {
        let start = index + 1;
        statements[start..]
            .iter()
            .rposition(|statement| !Self::can_complete_normally(statement))
            .map(|offset| start + offset)
    }

    fn linearize(statements: &mut Vec<JavaStmt>, index: usize, candidate: TerminalBranchCandidate) {
        let tail = statements.split_off(index + 1);
        let Some(JavaStmt::If {
            condition,
            then_stmt,
            else_stmt: None,
        }) = statements.pop()
        else {
            unreachable!("terminal branch candidate changed during linearization");
        };
        let repeated_tail = candidate.duplicates_tail.then(|| tail.clone());
        statements.push(JavaStmt::If {
            condition: condition.negated(),
            then_stmt: Box::new(JavaStmt::Block(tail)),
            else_stmt: None,
        });
        match *then_stmt {
            JavaStmt::Block(body) => statements.extend(body),
            JavaStmt::Empty => {}
            statement => statements.push(statement),
        }
        if let Some(tail) = repeated_tail {
            statements.extend(tail);
        }
    }

    fn next_candidate(
        statements: &[JavaStmt],
        start: usize,
    ) -> Option<(usize, TerminalBranchCandidate)> {
        let mut guards = Self::guards(statements, start);
        let first = guards.next()?;
        // A suffix can stop normal execution exactly when it includes a
        // non-completing statement. A later guard cannot reintroduce that exit.
        let last_exit = Self::last_exit_after(statements, first.0)?;

        // Store only increases in the maximum depth as the suffix grows.
        // These records monotonically expire while testing guards left to right.
        let mut depths = smallvec::SmallVec::<[(usize, usize); 4]>::new();
        let mut maximum = 0;
        for (index, statement) in statements.iter().enumerate().skip(first.0 + 1).rev() {
            let depth = Self::nesting_depth(statement);
            if depth > maximum {
                maximum = depth;
                depths.push((index, depth));
            }
        }

        for (index, condition, branch) in std::iter::once(first).chain(guards) {
            if index >= last_exit {
                break;
            }
            while depths
                .last()
                .is_some_and(|(position, _)| *position <= index)
            {
                depths.pop();
            }
            let tail_depth = depths.last().map_or(0, |(_, depth)| *depth);
            let branch_depth = Self::nesting_depth(branch);
            // Before the edit the guard adds one level to the branch. After
            // it the guard adds that level to the tail instead.
            if branch_depth < tail_depth
                || (branch_depth == tail_depth && !condition.negation_reduces_cost())
            {
                continue;
            }
            let duplicates_tail = Self::can_complete_normally(branch);
            if duplicates_tail {
                let budget = if branch_depth > tail_depth {
                    Self::NESTING_PENALTY
                } else {
                    0
                };
                if statements[index + 1..]
                    .iter()
                    .try_fold(budget, |left, statement| {
                        left.checked_sub(Self::statement_cost(statement))
                    })
                    .is_none()
                {
                    continue;
                }
            }
            return Some((index, TerminalBranchCandidate { duplicates_tail }));
        }
        None
    }

    fn next_required_void_exit(
        statements: &[JavaStmt],
        start: usize,
    ) -> Option<(usize, TerminalBranchCandidate)> {
        let (index, _, branch) = Self::guards(statements, start)
            .find(|(_, _, branch)| Self::terminates_with_void_return(branch))?;
        Self::last_exit_after(statements, index)?;
        Some((
            index,
            TerminalBranchCandidate {
                duplicates_tail: Self::can_complete_normally(branch),
            },
        ))
    }

    fn terminates_with_void_return(statement: &JavaStmt) -> bool {
        match statement {
            JavaStmt::Return(None) => true,
            JavaStmt::Block(statements) => statements
                .last()
                .is_some_and(Self::terminates_with_void_return),
            JavaStmt::Synchronized { body, .. } => Self::terminates_with_void_return(body),
            JavaStmt::If {
                then_stmt,
                else_stmt: Some(else_stmt),
                ..
            } => {
                Self::terminates_with_void_return(then_stmt)
                    && Self::terminates_with_void_return(else_stmt)
            }
            _ => false,
        }
    }

    fn sequence_can_complete_normally(statements: &[JavaStmt]) -> bool {
        statements.iter().all(Self::can_complete_normally)
    }

    fn can_complete_normally(statement: &JavaStmt) -> bool {
        match statement {
            JavaStmt::Return(_)
            | JavaStmt::Throw(_)
            | JavaStmt::Break(_)
            | JavaStmt::Continue(_) => false,
            JavaStmt::Block(statements) => Self::sequence_can_complete_normally(statements),
            JavaStmt::If {
                then_stmt,
                else_stmt: Some(else_stmt),
                ..
            } => Self::can_complete_normally(then_stmt) || Self::can_complete_normally(else_stmt),
            JavaStmt::Synchronized { body, .. } => Self::can_complete_normally(body),
            JavaStmt::While {
                label,
                condition,
                body,
            } => {
                !Self::is_true(condition)
                    || Self::contains_exiting_break(body, label.as_ref(), true)
            }
            JavaStmt::DoWhile {
                label,
                body,
                condition,
            } => {
                Self::contains_exiting_break(body, label.as_ref(), true)
                    || (Self::can_complete_normally(body) && !Self::is_true(condition))
            }
            JavaStmt::For {
                label,
                condition,
                body,
                ..
            } => {
                condition
                    .as_ref()
                    .is_some_and(|condition| !Self::is_true(condition))
                    || Self::contains_exiting_break(body, label.as_ref(), true)
            }
            JavaStmt::Switch { label, cases, .. } => {
                Self::switch_can_complete_normally(label.as_ref(), cases)
            }
            JavaStmt::Try {
                body,
                catches,
                finally,
            } => {
                let protected_completes = Self::can_complete_normally(body)
                    || catches
                        .iter()
                        .any(|catch| Self::can_complete_normally(&catch.body));
                let finally_completes = finally.as_deref().is_none_or(Self::can_complete_normally);
                protected_completes && finally_completes
            }
            JavaStmt::Empty
            | JavaStmt::RecoveryFailure(_)
            | JavaStmt::RecoveryGap(_)
            | JavaStmt::Variable { .. }
            | JavaStmt::Expression(_)
            | JavaStmt::ConstructorInvocation { .. }
            | JavaStmt::Assign { .. }
            | JavaStmt::If {
                else_stmt: None, ..
            }
            | JavaStmt::Labeled { .. }
            | JavaStmt::ForEach { .. } => true,
        }
    }

    fn is_true(expression: &JavaExpr) -> bool {
        matches!(
            expression,
            JavaExpr::Literal(super::JavaLiteral::Boolean(true))
        )
    }

    fn switch_can_complete_normally(
        label: Option<&JavaIdentifier>,
        cases: &[JavaSwitchCase],
    ) -> bool {
        if cases.is_empty() || !cases.iter().any(|case| case.is_default) {
            return true;
        }
        if cases
            .last()
            .is_some_and(|case| Self::sequence_can_complete_normally(&case.body))
        {
            return true;
        }
        cases.iter().any(|case| {
            case.body
                .iter()
                .any(|statement| Self::contains_exiting_break(statement, label, true))
        })
    }

    fn contains_exiting_break(
        root: &JavaStmt,
        target_label: Option<&JavaIdentifier>,
        direct: bool,
    ) -> bool {
        let mut pending = vec![(root, direct)];
        while let Some((statement, direct)) = pending.pop() {
            match statement {
                JavaStmt::Break(None) if direct => return true,
                JavaStmt::Break(Some(label)) if target_label == Some(label) => return true,
                JavaStmt::Block(statements) => {
                    pending.extend(statements.iter().map(|statement| (statement, direct)));
                }
                JavaStmt::If {
                    then_stmt,
                    else_stmt,
                    ..
                } => {
                    pending.push((then_stmt, direct));
                    pending.extend(else_stmt.as_deref().map(|statement| (statement, direct)));
                }
                JavaStmt::Labeled { body, .. } | JavaStmt::Synchronized { body, .. } => {
                    pending.push((body, direct));
                }
                JavaStmt::Try {
                    body,
                    catches,
                    finally,
                } => {
                    pending.push((body, direct));
                    pending.extend(catches.iter().map(|catch| (&catch.body, direct)));
                    pending.extend(finally.as_deref().map(|statement| (statement, direct)));
                }
                JavaStmt::While { body, .. }
                | JavaStmt::DoWhile { body, .. }
                | JavaStmt::For { body, .. }
                | JavaStmt::ForEach { body, .. } => {
                    if target_label.is_some() {
                        pending.push((body, false));
                    }
                }
                JavaStmt::Switch { cases, .. } => {
                    if target_label.is_some() {
                        pending.extend(
                            cases
                                .iter()
                                .flat_map(|case| &case.body)
                                .map(|statement| (statement, false)),
                        );
                    }
                }
                JavaStmt::Empty
                | JavaStmt::RecoveryFailure(_)
                | JavaStmt::RecoveryGap(_)
                | JavaStmt::Variable { .. }
                | JavaStmt::Expression(_)
                | JavaStmt::ConstructorInvocation { .. }
                | JavaStmt::Assign { .. }
                | JavaStmt::Return(_)
                | JavaStmt::Throw(_)
                | JavaStmt::Break(None)
                | JavaStmt::Break(Some(_))
                | JavaStmt::Continue(_) => {}
            }
        }
        false
    }

    fn nesting_depth(statement: &JavaStmt) -> usize {
        match statement {
            JavaStmt::Block(statements) => statements
                .iter()
                .map(Self::nesting_depth)
                .max()
                .unwrap_or_default(),
            JavaStmt::If {
                then_stmt,
                else_stmt,
                ..
            } => {
                1 + Self::nesting_depth(then_stmt).max(
                    else_stmt
                        .as_deref()
                        .map(Self::nesting_depth)
                        .unwrap_or_default(),
                )
            }
            JavaStmt::Labeled { body, .. }
            | JavaStmt::While { body, .. }
            | JavaStmt::DoWhile { body, .. }
            | JavaStmt::For { body, .. }
            | JavaStmt::ForEach { body, .. }
            | JavaStmt::Synchronized { body, .. } => 1 + Self::nesting_depth(body),
            JavaStmt::Switch { cases, .. } => {
                1 + cases
                    .iter()
                    .flat_map(|case| &case.body)
                    .map(Self::nesting_depth)
                    .max()
                    .unwrap_or_default()
            }
            JavaStmt::Try {
                body,
                catches,
                finally,
            } => {
                let branches = std::iter::once(body.as_ref())
                    .chain(catches.iter().map(|catch| &catch.body))
                    .chain(finally.as_deref());
                1 + branches.map(Self::nesting_depth).max().unwrap_or_default()
            }
            JavaStmt::Empty
            | JavaStmt::RecoveryFailure(_)
            | JavaStmt::RecoveryGap(_)
            | JavaStmt::Variable { .. }
            | JavaStmt::Expression(_)
            | JavaStmt::ConstructorInvocation { .. }
            | JavaStmt::Assign { .. }
            | JavaStmt::Return(_)
            | JavaStmt::Throw(_)
            | JavaStmt::Break(_)
            | JavaStmt::Continue(_) => 0,
        }
    }

    fn sequence_cost(statements: &[JavaStmt]) -> usize {
        statements.iter().map(Self::statement_cost).sum()
    }

    fn statement_cost(statement: &JavaStmt) -> usize {
        match statement {
            JavaStmt::Empty | JavaStmt::RecoveryFailure(_) | JavaStmt::RecoveryGap(_) => 0,
            JavaStmt::Variable { value, .. } => {
                1 + value.as_ref().map(JavaExpr::cost).unwrap_or_default()
            }
            JavaStmt::Expression(expression) | JavaStmt::Throw(expression) => 1 + expression.cost(),
            JavaStmt::ConstructorInvocation { args, .. } => {
                1 + args.iter().map(JavaExpr::cost).sum::<usize>()
            }
            JavaStmt::Assign { target, value, .. } => 1 + target.cost() + value.cost(),
            JavaStmt::Return(value) => 1 + value.as_ref().map(JavaExpr::cost).unwrap_or_default(),
            JavaStmt::Break(_) | JavaStmt::Continue(_) => 1,
            JavaStmt::Block(statements) => Self::sequence_cost(statements),
            JavaStmt::Labeled { body, .. }
            | JavaStmt::While { body, .. }
            | JavaStmt::DoWhile { body, .. }
            | JavaStmt::ForEach { body, .. }
            | JavaStmt::Synchronized { body, .. } => 1 + Self::statement_cost(body),
            JavaStmt::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                1 + Self::sequence_cost(init)
                    + condition.as_ref().map(JavaExpr::cost).unwrap_or_default()
                    + update.iter().map(JavaExpr::cost).sum::<usize>()
                    + Self::statement_cost(body)
            }
            JavaStmt::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                1 + condition.cost()
                    + Self::statement_cost(then_stmt)
                    + else_stmt
                        .as_deref()
                        .map(Self::statement_cost)
                        .unwrap_or_default()
            }
            JavaStmt::Switch {
                selector, cases, ..
            } => {
                1 + selector.cost()
                    + cases
                        .iter()
                        .map(|case| Self::sequence_cost(&case.body))
                        .sum::<usize>()
            }
            JavaStmt::Try {
                body,
                catches,
                finally,
            } => {
                1 + Self::statement_cost(body)
                    + catches
                        .iter()
                        .map(|catch| Self::statement_cost(&catch.body))
                        .sum::<usize>()
                    + finally
                        .as_deref()
                        .map(Self::statement_cost)
                        .unwrap_or_default()
            }
        }
    }
}

struct TerminalBranchCandidate {
    duplicates_tail: bool,
}

pub(crate) struct MethodNormalization {
    pub changed: bool,
    pub has_array_initializers: bool,
    pub has_uninitialized_variables: bool,
}

impl JavaAstNormalizer {
    pub(crate) fn normalize_method(
        &mut self,
        body: &mut JavaMethodBody,
    ) -> Result<MethodNormalization, super::JavaStructuralError> {
        let (root, mut changed) =
            Self::normalize(std::mem::replace(&mut body.root, JavaStmt::Empty))?;
        let mut conditional_simplifier = ConditionalExpressionSimplifier::default();
        let mut root = conditional_simplifier.rewrite_statement(root);
        changed |= conditional_simplifier.changed;
        if let JavaStmt::Block(statements) = &mut root {
            if let Some(last) = statements.last_mut() {
                changed |= Self::strip_terminal_void_return(last);
            }
            if matches!(statements.last(), Some(JavaStmt::Empty)) {
                statements.pop();
                changed = true;
            }
        }
        body.root = root;
        Ok(MethodNormalization {
            changed,
            has_array_initializers: conditional_simplifier.has_array_initializers,
            has_uninitialized_variables: conditional_simplifier.has_uninitialized_variables,
        })
    }
}

impl JavaAstTransform for JavaAstNormalizer {
    type Error = super::JavaStructuralError;

    fn apply(&mut self, body: &mut JavaMethodBody) -> Result<bool, Self::Error> {
        Ok(self.normalize_method(body)?.changed)
    }
}

#[derive(Debug, Default)]
struct ConditionalExpressionSimplifier {
    changed: bool,
    has_array_initializers: bool,
    has_uninitialized_variables: bool,
}

impl JavaAstMutator for ConditionalExpressionSimplifier {
    fn after_statement(&mut self, statement: &mut JavaStmt) {
        if let JavaStmt::Variable { value, .. } = statement {
            match value {
                None => self.has_uninitialized_variables = true,
                Some(JavaExpr::NewArray { .. } | JavaExpr::Cast { .. }) => {
                    self.has_array_initializers = true
                }
                Some(_) => {}
            }
        }
    }

    fn after_expression(&mut self, expression: &mut JavaExpr) {
        let JavaExpr::Conditional {
            condition,
            when_true,
            when_false,
        } = expression
        else {
            return;
        };
        let JavaExpr::Conditional {
            condition: nested_condition,
            when_true: nested_true,
            when_false: nested_false,
        } = when_false.as_mut()
        else {
            return;
        };
        if condition.as_ref() == nested_condition.as_ref()
            && when_true.as_ref() == nested_true.as_ref()
            && ConditionalResultJoin::stable(condition.as_ref())
        {
            **when_false = std::mem::replace(nested_false.as_mut(), JavaExpr::This);
            self.changed = true;
        }
    }
}

enum SyntaxTask {
    Visit(JavaStmt),
    Block {
        children: Vec<JavaStmt>,
        index: usize,
    },
    Rebuild(SyntaxFrame),
}

enum SyntaxFrame {
    Labeled(JavaIdentifier),
    If {
        condition: JavaExpr,
        has_else: bool,
    },
    While {
        label: Option<JavaIdentifier>,
        condition: JavaExpr,
    },
    DoWhile {
        label: Option<JavaIdentifier>,
        condition: JavaExpr,
    },
    For {
        label: Option<JavaIdentifier>,
        init: Vec<JavaStmt>,
        condition: Option<JavaExpr>,
        update: Vec<JavaExpr>,
    },
    ForEach {
        label: Option<JavaIdentifier>,
        ty: JavaType,
        variable: JavaIdentifier,
        iterable: JavaExpr,
    },
    Switch {
        label: Option<JavaIdentifier>,
        selector: JavaExpr,
        cases: Vec<JavaSwitchCase>,
    },
    Try {
        catches: Vec<JavaCatch>,
        has_finally: bool,
    },
    Synchronized(JavaExpr),
}

impl SyntaxFrame {
    fn child_count(&self) -> usize {
        match self {
            Self::Labeled(_) => 1,
            Self::If { has_else, .. } => 1 + usize::from(*has_else),
            Self::While { .. }
            | Self::DoWhile { .. }
            | Self::For { .. }
            | Self::ForEach { .. }
            | Self::Synchronized(_) => 1,
            Self::Switch { cases, .. } => cases.len(),
            Self::Try {
                catches,
                has_finally,
            } => 1 + catches.len() + usize::from(*has_finally),
        }
    }

    fn rebuild(
        self,
        mut children: impl ExactSizeIterator<Item = JavaStmt>,
    ) -> Result<(JavaStmt, bool), super::JavaStructuralError> {
        let expected = self.child_count();
        if children.len() != expected {
            return Err(super::JavaStructuralError::ChildArity {
                expected,
                actual: children.len(),
            });
        }
        let statement = match self {
            Self::Labeled(label) => JavaStmt::Labeled {
                label,
                body: Box::new(Self::child(&mut children)?),
            },
            Self::If {
                condition,
                has_else,
            } => {
                let then_stmt = Self::child(&mut children)?;
                let else_stmt = if has_else {
                    Some(Self::child(&mut children)?)
                } else {
                    None
                };
                return Ok(JavaAstNormalizer::conditional(
                    condition, then_stmt, else_stmt,
                ));
            }
            Self::While { label, condition } => JavaStmt::While {
                label,
                condition,
                body: Box::new(Self::child(&mut children)?),
            },
            Self::DoWhile { label, condition } => JavaStmt::DoWhile {
                label,
                body: Box::new(Self::child(&mut children)?),
                condition,
            },
            Self::For {
                label,
                init,
                condition,
                update,
            } => JavaStmt::For {
                label,
                init,
                condition,
                update,
                body: Box::new(Self::child(&mut children)?),
            },
            Self::ForEach {
                label,
                ty,
                variable,
                iterable,
            } => {
                return Ok(JavaAstNormalizer::foreach(
                    label,
                    ty,
                    variable,
                    iterable,
                    Self::child(&mut children)?,
                ));
            }
            Self::Switch {
                label,
                selector,
                mut cases,
            } => {
                for case in &mut cases {
                    case.body = match Self::child(&mut children)? {
                        JavaStmt::Block(body) => body,
                        JavaStmt::Empty => Vec::new(),
                        statement => vec![statement],
                    };
                }
                JavaStmt::Switch {
                    label,
                    selector,
                    cases,
                }
            }
            Self::Try {
                mut catches,
                has_finally,
            } => {
                let body = Self::child(&mut children)?;
                for catch in &mut catches {
                    catch.body = Self::child(&mut children)?;
                }
                let finally = if has_finally {
                    Some(Box::new(Self::child(&mut children)?))
                } else {
                    None
                };
                return Ok(JavaAstNormalizer::protection(body, catches, finally));
            }
            Self::Synchronized(lock) => JavaStmt::Synchronized {
                lock,
                body: Box::new(Self::child(&mut children)?),
            },
        };
        Ok((statement, false))
    }

    fn child(
        children: &mut impl Iterator<Item = JavaStmt>,
    ) -> Result<JavaStmt, super::JavaStructuralError> {
        children
            .next()
            .ok_or(super::JavaStructuralError::MalformedWorkStack)
    }
}

struct NameUseCounter<'a> {
    target: &'a JavaIdentifier,
    count: usize,
}

impl JavaAstRewriter for NameUseCounter<'_> {
    fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        if matches!(&expression, JavaExpr::Name(name) if name == self.target) {
            self.count += 1;
        }
        expression
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(name: &str) -> JavaStmt {
        JavaStmt::Expression(JavaExpr::Name(JavaIdentifier::from_dex(name)))
    }

    fn repeated_conditional(condition: JavaExpr) -> JavaExpr {
        JavaExpr::Conditional {
            condition: Box::new(condition.clone()),
            when_true: Box::new(JavaExpr::Literal(JavaLiteral::Integer(1))),
            when_false: Box::new(JavaExpr::Conditional {
                condition: Box::new(condition),
                when_true: Box::new(JavaExpr::Literal(JavaLiteral::Integer(1))),
                when_false: Box::new(JavaExpr::Literal(JavaLiteral::Integer(0))),
            }),
        }
    }

    #[test]
    fn simplifies_repeated_stable_conditional_branch() {
        let condition = JavaExpr::Name(JavaIdentifier::from_dex("enabled"));
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![JavaStmt::Variable {
                ty: JavaType::int(),
                name: JavaIdentifier::from_dex("value"),
                value: Some(repeated_conditional(condition.clone())),
            }]),
        };

        assert!(JavaAstNormalizer.apply(&mut body).unwrap());
        let JavaStmt::Block(statements) = &body.root else {
            panic!("expected method block");
        };
        let JavaStmt::Variable {
            value: Some(value), ..
        } = &statements[0]
        else {
            panic!("expected variable initializer");
        };
        assert_eq!(
            value,
            &JavaExpr::Conditional {
                condition: Box::new(condition),
                when_true: Box::new(JavaExpr::Literal(JavaLiteral::Integer(1))),
                when_false: Box::new(JavaExpr::Literal(JavaLiteral::Integer(0))),
            }
        );
    }

    #[test]
    fn keeps_repeated_effectful_conditional_branch() {
        let condition = JavaExpr::Field {
            owner: Box::new(JavaExpr::This),
            name: JavaIdentifier::from_dex("enabled"),
        };
        let expression = repeated_conditional(condition);
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![JavaStmt::Variable {
                ty: JavaType::int(),
                name: JavaIdentifier::from_dex("value"),
                value: Some(expression.clone()),
            }]),
        };

        assert!(!JavaAstNormalizer.apply(&mut body).unwrap());
        let JavaStmt::Block(statements) = &body.root else {
            panic!("expected method block");
        };
        let JavaStmt::Variable {
            value: Some(value), ..
        } = &statements[0]
        else {
            panic!("expected variable initializer");
        };
        assert_eq!(value, &expression);
    }

    #[test]
    fn removes_pure_empty_if() {
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![
                JavaStmt::If {
                    condition: JavaExpr::Binary {
                        left: Box::new(JavaExpr::Name(JavaIdentifier::from_dex("ready"))),
                        op: JavaBinaryOp::Equal,
                        right: Box::new(JavaExpr::Literal(JavaLiteral::Boolean(true))),
                    },
                    then_stmt: Box::new(JavaStmt::Empty),
                    else_stmt: None,
                },
                marker("work"),
            ]),
        };

        assert!(JavaAstNormalizer.apply(&mut body).unwrap());
        assert!(matches!(
            body.root,
            JavaStmt::Block(statements)
                if statements.len() == 1
                    && matches!(&statements[0], JavaStmt::Expression(JavaExpr::Name(name)) if name.as_str() == "work")
        ));
    }

    #[test]
    fn keeps_effectful_empty_if() {
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![JavaStmt::If {
                condition: JavaExpr::Call {
                    receiver: None,
                    owner: None,
                    virtual_target: None,
                    type_arguments: Vec::new(),
                    method: JavaIdentifier::from_dex("ready"),
                    args: Vec::new(),
                },
                then_stmt: Box::new(JavaStmt::Empty),
                else_stmt: None,
            }]),
        };
        let original = body.clone();

        assert!(!JavaAstNormalizer.apply(&mut body).unwrap());
        assert_eq!(body.root, original.root);
    }

    #[test]
    fn void_tail_returns_from_small_then_branch() {
        let condition = JavaExpr::Name(JavaIdentifier::from_dex("skip"));
        let trailing = (0..8)
            .map(|index| marker(&format!("work{index}")))
            .collect::<Vec<_>>();
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![JavaStmt::If {
                condition: condition.clone(),
                then_stmt: Box::new(JavaStmt::Block(vec![marker("skipWork")])),
                else_stmt: Some(Box::new(JavaStmt::Block(trailing.clone()))),
            }]),
        };

        assert!(JavaVoidTailLinearizer.apply(&mut body).unwrap());
        let JavaStmt::Block(statements) = &body.root else {
            panic!("expected method block");
        };
        let JavaStmt::If {
            condition: actual,
            then_stmt,
            else_stmt: None,
        } = &statements[0]
        else {
            panic!("expected guard return");
        };
        assert_eq!(actual, &condition);
        assert!(matches!(
            then_stmt.as_ref(),
            JavaStmt::Block(branch) if matches!(branch.last(), Some(JavaStmt::Return(None)))
        ));
        assert_eq!(&statements[1..], trailing.as_slice());
    }

    #[test]
    fn void_tail_inverts_condition_for_small_else_branch() {
        let condition = JavaExpr::Name(JavaIdentifier::from_dex("ready"));
        let trailing = (0..8)
            .map(|index| marker(&format!("work{index}")))
            .collect::<Vec<_>>();
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![JavaStmt::If {
                condition: condition.clone(),
                then_stmt: Box::new(JavaStmt::Block(trailing.clone())),
                else_stmt: Some(Box::new(JavaStmt::Block(vec![marker("fallback")]))),
            }]),
        };

        assert!(JavaVoidTailLinearizer.apply(&mut body).unwrap());
        let JavaStmt::Block(statements) = &body.root else {
            panic!("expected method block");
        };
        assert!(matches!(
            &statements[0],
            JavaStmt::If { condition: actual, else_stmt: None, .. }
                if actual == &condition.clone().negated()
        ));
        assert_eq!(&statements[1..], trailing.as_slice());
    }

    #[test]
    fn void_tail_keeps_balanced_if_else() {
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![JavaStmt::If {
                condition: JavaExpr::Name(JavaIdentifier::from_dex("condition")),
                then_stmt: Box::new(marker("left")),
                else_stmt: Some(Box::new(marker("right"))),
            }]),
        };
        let original = body.clone();

        assert!(!JavaVoidTailLinearizer.apply(&mut body).unwrap());
        assert_eq!(body.root, original.root);
    }

    #[test]
    fn void_tail_does_not_duplicate_existing_return() {
        let trailing = (0..8)
            .map(|index| marker(&format!("work{index}")))
            .collect::<Vec<_>>();
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![JavaStmt::If {
                condition: JavaExpr::Name(JavaIdentifier::from_dex("done")),
                then_stmt: Box::new(JavaStmt::Return(None)),
                else_stmt: Some(Box::new(JavaStmt::Block(trailing))),
            }]),
        };

        assert!(JavaVoidTailLinearizer.apply(&mut body).unwrap());
        let JavaStmt::Block(statements) = &body.root else {
            panic!("expected method block");
        };
        assert!(matches!(
            &statements[0],
            JavaStmt::If { then_stmt, else_stmt: None, .. }
                if matches!(then_stmt.as_ref(), JavaStmt::Return(None))
        ));
    }

    fn complementary_condition(guard: JavaExpr) -> JavaMethodBody {
        let result = JavaIdentifier::from_dex("condition");
        JavaMethodBody {
            root: JavaStmt::Block(vec![
                JavaStmt::Variable {
                    ty: JavaType::boolean(),
                    name: result.clone(),
                    value: Some(JavaExpr::Literal(JavaLiteral::Boolean(false))),
                },
                JavaStmt::If {
                    condition: guard.clone(),
                    then_stmt: Box::new(JavaStmt::Block(vec![
                        JavaStmt::Assign {
                            target: JavaExpr::Name(result.clone()),
                            op: JavaAssignOp::Assign,
                            value: JavaExpr::Name(JavaIdentifier::from_dex("predicate")),
                        },
                        JavaStmt::If {
                            condition: JavaExpr::Name(result.clone()),
                            then_stmt: Box::new(JavaStmt::Expression(JavaExpr::Name(
                                JavaIdentifier::from_dex("success"),
                            ))),
                            else_stmt: None,
                        },
                    ])),
                    else_stmt: None,
                },
                JavaStmt::If {
                    condition: JavaExpr::Binary {
                        left: Box::new(guard.negated()),
                        op: JavaBinaryOp::LogicalOr,
                        right: Box::new(JavaExpr::Unary {
                            op: JavaUnaryOp::LogicalNot,
                            operand: Box::new(JavaExpr::Name(result)),
                        }),
                    },
                    then_stmt: Box::new(JavaStmt::Expression(JavaExpr::Name(
                        JavaIdentifier::from_dex("fallback"),
                    ))),
                    else_stmt: None,
                },
            ]),
        }
    }

    #[test]
    fn joins_complementary_boolean_result_branches() {
        let guard = JavaExpr::Binary {
            left: Box::new(JavaExpr::Name(JavaIdentifier::from_dex("flags"))),
            op: JavaBinaryOp::Equal,
            right: Box::new(JavaExpr::Literal(JavaLiteral::Integer(2))),
        };
        let mut body = complementary_condition(guard.clone());

        assert!(JavaAstNormalizer.apply(&mut body).unwrap());
        let JavaStmt::Block(statements) = &body.root else {
            panic!("expected method block");
        };
        let [JavaStmt::If {
            condition,
            then_stmt,
            else_stmt: Some(else_stmt),
        }] = statements.as_slice()
        else {
            panic!("expected joined branch");
        };
        assert!(matches!(
            condition,
            JavaExpr::Binary {
                left,
                op: JavaBinaryOp::LogicalAnd,
                right,
            } if left.as_ref() == &guard
                && matches!(right.as_ref(), JavaExpr::Name(name) if name.as_str() == "predicate")
        ));
        assert!(
            matches!(then_stmt.as_ref(), JavaStmt::Expression(JavaExpr::Name(name)) if name.as_str() == "success")
        );
        assert!(
            matches!(else_stmt.as_ref(), JavaStmt::Expression(JavaExpr::Name(name)) if name.as_str() == "fallback")
        );
    }

    #[test]
    fn keeps_complementary_result_when_guard_may_have_effects() {
        let mut body = complementary_condition(JavaExpr::Call {
            receiver: None,
            owner: None,
            virtual_target: None,
            type_arguments: Vec::new(),
            method: JavaIdentifier::from_dex("guard"),
            args: Vec::new(),
        });
        let original = body.clone();

        assert!(!JavaAstNormalizer.apply(&mut body).unwrap());
        assert_eq!(body.root, original.root);
    }

    #[test]
    fn keeps_complementary_result_when_success_rewrites_guard_input() {
        let guard_name = JavaIdentifier::from_dex("guard");
        let mut body = complementary_condition(JavaExpr::Name(guard_name.clone()));
        let JavaStmt::Block(statements) = &mut body.root else {
            unreachable!();
        };
        let JavaStmt::If { then_stmt, .. } = &mut statements[1] else {
            unreachable!();
        };
        let JavaStmt::Block(guarded) = then_stmt.as_mut() else {
            unreachable!();
        };
        let JavaStmt::If { then_stmt, .. } = &mut guarded[1] else {
            unreachable!();
        };
        **then_stmt = JavaStmt::Assign {
            target: JavaExpr::Name(guard_name),
            op: JavaAssignOp::Assign,
            value: JavaExpr::Literal(JavaLiteral::Boolean(false)),
        };
        let original = body.clone();

        assert!(!JavaAstNormalizer.apply(&mut body).unwrap());
        assert_eq!(body.root, original.root);
    }

    fn assign_integer(name: &JavaIdentifier, value: i32) -> JavaStmt {
        JavaStmt::Assign {
            target: JavaExpr::Name(name.clone()),
            op: JavaAssignOp::Assign,
            value: JavaExpr::Literal(JavaLiteral::Integer(value)),
        }
    }

    const STRING_HASH: &str = "Ljava/lang/String;->hashCode()I";
    const STRING_EQUALS: &str = "Ljava/lang/String;->equals(Ljava/lang/Object;)Z";

    fn string_call(
        receiver: &JavaIdentifier,
        method: &str,
        member: Option<&str>,
        args: Vec<JavaExpr>,
    ) -> JavaExpr {
        JavaExpr::Call {
            receiver: Some(Box::new(JavaExpr::Name(receiver.clone()))),
            owner: None,
            virtual_target: member.map(|member| {
                std::sync::Arc::new(MemberReference::Method(member.parse().unwrap()))
            }),
            type_arguments: Vec::new(),
            method: JavaIdentifier::from_dex(method),
            args,
        }
    }

    fn lowered_string_switch() -> JavaMethodBody {
        string_switch_with_targets(Some(STRING_HASH), Some(STRING_EQUALS))
    }

    fn string_switch_with_targets(
        hash_target: Option<&str>,
        equals_target: Option<&str>,
    ) -> JavaMethodBody {
        let source = JavaIdentifier::from_dex("source");
        let synthetic = JavaIdentifier::from_dex("selector");
        let hash_case = |literal: &str, selected: i32| JavaSwitchCase {
            labels: vec![JavaExpr::Literal(JavaLiteral::Integer(
                StringSwitchRecovery::string_hash(&literal.encode_utf16().collect::<Vec<_>>()),
            ))],
            body: vec![
                JavaStmt::If {
                    condition: string_call(
                        &source,
                        "equals",
                        equals_target,
                        vec![JavaExpr::Literal(JavaLiteral::String(literal.into()))],
                    ),
                    then_stmt: Box::new(JavaStmt::Block(if selected == 0 {
                        vec![JavaStmt::Break(None)]
                    } else {
                        vec![assign_integer(&synthetic, selected), JavaStmt::Break(None)]
                    })),
                    else_stmt: None,
                },
                assign_integer(&synthetic, -1),
                JavaStmt::Break(None),
            ],
            is_default: false,
        };
        JavaMethodBody {
            root: JavaStmt::Block(vec![
                JavaStmt::Variable {
                    ty: JavaType::Primitive(JavaPrimitiveType::Int),
                    name: synthetic.clone(),
                    value: Some(JavaExpr::Literal(JavaLiteral::Integer(0))),
                },
                JavaStmt::Switch {
                    label: None,
                    selector: string_call(&source, "hashCode", hash_target, Vec::new()),
                    cases: vec![
                        hash_case("alpha", 0),
                        hash_case("beta", 1),
                        JavaSwitchCase {
                            labels: Vec::new(),
                            body: vec![assign_integer(&synthetic, -1), JavaStmt::Break(None)],
                            is_default: true,
                        },
                    ],
                },
                JavaStmt::Switch {
                    label: None,
                    selector: JavaExpr::Name(synthetic),
                    cases: vec![
                        JavaSwitchCase {
                            labels: vec![JavaExpr::Literal(JavaLiteral::Integer(0))],
                            body: vec![marker("first"), JavaStmt::Break(None)],
                            is_default: false,
                        },
                        JavaSwitchCase {
                            labels: vec![JavaExpr::Literal(JavaLiteral::Integer(1))],
                            body: vec![marker("second"), JavaStmt::Break(None)],
                            is_default: false,
                        },
                        JavaSwitchCase {
                            labels: Vec::new(),
                            body: vec![marker("fallback"), JavaStmt::Break(None)],
                            is_default: true,
                        },
                    ],
                },
            ]),
        }
    }

    #[test]
    fn reconstructs_verified_string_switch() {
        let mut body = lowered_string_switch();

        assert!(JavaAstNormalizer.apply(&mut body).unwrap());
        let JavaStmt::Block(statements) = &body.root else {
            panic!("expected method block");
        };
        let [JavaStmt::Switch {
            selector: JavaExpr::Name(source),
            cases,
            ..
        }] = statements.as_slice()
        else {
            panic!("expected recovered string switch");
        };
        assert_eq!(source.as_str(), "source");
        assert!(matches!(
            cases[0].labels.as_slice(),
            [JavaExpr::Literal(JavaLiteral::String(value))] if value.to_string_lossy() == "alpha"
        ));
        assert!(matches!(
            cases[1].labels.as_slice(),
            [JavaExpr::Literal(JavaLiteral::String(value))] if value.to_string_lossy() == "beta"
        ));
        assert!(cases[2].is_default);
    }

    #[test]
    fn keeps_string_switch_lowering_without_both_original_string_calls() {
        for (hash, equals) in [
            (None, Some(STRING_EQUALS)),
            (Some("Lfixture/Key;->hashCode()I"), Some(STRING_EQUALS)),
            (Some("Ljava/lang/String;->hashCode()J"), Some(STRING_EQUALS)),
            (Some(STRING_HASH), None),
            (
                Some(STRING_HASH),
                Some("Lfixture/Key;->equals(Ljava/lang/Object;)Z"),
            ),
            (
                Some(STRING_HASH),
                Some("Ljava/lang/String;->equals(Ljava/lang/String;)Z"),
            ),
        ] {
            let mut body = string_switch_with_targets(hash, equals);
            let JavaStmt::Block(statements) = &mut body.root else {
                unreachable!();
            };
            let original = statements.clone();

            assert!(
                !StringSwitchRecovery::apply(statements),
                "{hash:?} {equals:?}"
            );
            assert_eq!(*statements, original);
        }
    }

    #[test]
    fn keeps_string_switch_lowering_when_hash_is_not_verified() {
        let mut body = lowered_string_switch();
        let JavaStmt::Block(statements) = &mut body.root else {
            unreachable!();
        };
        let JavaStmt::Switch { cases, .. } = &mut statements[1] else {
            unreachable!();
        };
        cases[0].labels[0] = JavaExpr::Literal(JavaLiteral::Integer(0));

        JavaAstNormalizer.apply(&mut body).unwrap();
        let JavaStmt::Block(statements) = &body.root else {
            unreachable!();
        };
        assert_eq!(
            statements
                .iter()
                .filter(|statement| matches!(statement, JavaStmt::Switch { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn keeps_string_switch_selector_when_used_after_dispatch() {
        let mut body = lowered_string_switch();
        let JavaStmt::Block(statements) = &mut body.root else {
            unreachable!();
        };
        statements.push(JavaStmt::Expression(JavaExpr::Name(
            JavaIdentifier::from_dex("selector"),
        )));

        JavaAstNormalizer.apply(&mut body).unwrap();
        let JavaStmt::Block(statements) = &body.root else {
            unreachable!();
        };
        assert!(matches!(
            statements.first(),
            Some(JavaStmt::Variable { .. })
        ));
    }

    fn result_switch(has_default: bool, initializer: Option<JavaExpr>) -> JavaMethodBody {
        let result = JavaIdentifier::from_dex("result");
        JavaMethodBody {
            root: JavaStmt::Block(vec![
                JavaStmt::Variable {
                    ty: JavaType::Variable(JavaIdentifier::from_dex("T")),
                    name: result.clone(),
                    value: initializer,
                },
                JavaStmt::Switch {
                    label: None,
                    selector: JavaExpr::Name(JavaIdentifier::from_dex("selector")),
                    cases: vec![
                        JavaSwitchCase {
                            labels: vec![JavaExpr::Literal(JavaLiteral::Integer(0))],
                            body: vec![
                                JavaStmt::Assign {
                                    target: JavaExpr::Name(result.clone()),
                                    op: JavaAssignOp::Assign,
                                    value: JavaExpr::Name(JavaIdentifier::from_dex("first")),
                                },
                                JavaStmt::Break(None),
                            ],
                            is_default: false,
                        },
                        JavaSwitchCase {
                            labels: Vec::new(),
                            body: vec![JavaStmt::Throw(JavaExpr::Name(JavaIdentifier::from_dex(
                                "invalid",
                            )))],
                            is_default: has_default,
                        },
                    ],
                },
                JavaStmt::Return(Some(JavaExpr::Name(result))),
            ]),
        }
    }

    #[test]
    fn total_switch_returns_values_directly() {
        let mut body = result_switch(true, Some(JavaExpr::Literal(JavaLiteral::Null)));

        assert!(JavaAstNormalizer.apply(&mut body).unwrap());
        let JavaStmt::Block(statements) = &body.root else {
            panic!("expected method block");
        };
        let [JavaStmt::Switch { cases, .. }] = statements.as_slice() else {
            panic!("expected direct switch");
        };
        assert!(matches!(
            cases[0].body.as_slice(),
            [JavaStmt::Return(Some(JavaExpr::Name(value)))] if value.as_str() == "first"
        ));
        assert!(matches!(cases[1].body.as_slice(), [JavaStmt::Throw(_)]));
    }

    #[test]
    fn total_switch_preserves_final_result_cast() {
        let mut body = result_switch(true, Some(JavaExpr::Literal(JavaLiteral::Null)));
        let JavaStmt::Block(statements) = &mut body.root else {
            unreachable!();
        };
        statements[2] = JavaStmt::Return(Some(JavaExpr::Cast {
            ty: JavaType::Variable(JavaIdentifier::from_dex("R")),
            value: Box::new(JavaExpr::Name(JavaIdentifier::from_dex("result"))),
        }));

        assert!(JavaAstNormalizer.apply(&mut body).unwrap());
        let JavaStmt::Block(statements) = &body.root else {
            panic!("expected method block");
        };
        let [JavaStmt::Switch { cases, .. }] = statements.as_slice() else {
            panic!("expected direct switch");
        };
        assert!(matches!(
            cases[0].body.as_slice(),
            [JavaStmt::Return(Some(JavaExpr::Cast { ty: JavaType::Variable(ty), value }))]
                if ty.as_str() == "R"
                    && matches!(value.as_ref(), JavaExpr::Name(name) if name.as_str() == "first")
        ));
    }

    #[test]
    fn switch_result_local_is_kept_without_default_case() {
        let mut body = result_switch(false, Some(JavaExpr::Literal(JavaLiteral::Null)));
        let original = body.clone();

        assert!(!JavaAstNormalizer.apply(&mut body).unwrap());
        assert_eq!(body.root, original.root);
    }

    #[test]
    fn switch_result_local_is_kept_when_initializer_has_effects() {
        let mut body = result_switch(
            true,
            Some(JavaExpr::Call {
                receiver: None,
                owner: None,
                virtual_target: None,
                type_arguments: Vec::new(),
                method: JavaIdentifier::from_dex("initialize"),
                args: Vec::new(),
            }),
        );
        let original = body.clone();

        assert!(!JavaAstNormalizer.apply(&mut body).unwrap());
        assert_eq!(body.root, original.root);
    }

    #[test]
    fn switch_result_local_is_kept_when_a_case_reads_it() {
        let mut body = result_switch(true, Some(JavaExpr::Literal(JavaLiteral::Null)));
        let JavaStmt::Block(statements) = &mut body.root else {
            unreachable!();
        };
        let JavaStmt::Switch { cases, .. } = &mut statements[1] else {
            unreachable!();
        };
        let JavaStmt::Assign { value, .. } = &mut cases[0].body[0] else {
            unreachable!();
        };
        *value = JavaExpr::Name(JavaIdentifier::from_dex("result"));
        let original = body.clone();

        assert!(!JavaAstNormalizer.apply(&mut body).unwrap());
        assert_eq!(body.root, original.root);
    }

    #[test]
    fn removes_try_with_only_an_empty_finally() {
        let statement = JavaStmt::Expression(JavaExpr::Name(JavaIdentifier::from_dex("work")));
        let mut body = JavaMethodBody {
            root: JavaStmt::Try {
                body: Box::new(statement.clone()),
                catches: Vec::new(),
                finally: Some(Box::new(JavaStmt::Block(Vec::new()))),
            },
        };

        assert!(JavaAstNormalizer.apply(&mut body).unwrap());
        assert_eq!(body.root, statement);
    }

    #[test]
    fn source_completion_appends_default_return_after_semantic_no_return_call() {
        let call = JavaStmt::Expression(JavaExpr::Name(JavaIdentifier::from_dex("neverReturns")));
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![call.clone()]),
        };

        assert!(
            JavaMethodCompletion::new(JavaType::Variable(JavaIdentifier::from_dex("T")))
                .apply(&mut body)
                .unwrap()
        );
        assert_eq!(
            body.root,
            JavaStmt::Block(vec![
                call,
                JavaStmt::Return(Some(JavaExpr::Literal(JavaLiteral::Null))),
            ])
        );
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/java_terminal_guard_tails.rs"]
mod tail_tests;
