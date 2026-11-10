//! Shared traversal of owned expression children and selected evaluation edges.
//! This reads P7 syntax only; it discovers no execution, type or storage facts.

use super::build::{bug, push};
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Roots<'a> {
    head: [Option<ExprId>; 3],
    tail: &'a [ExprId],
}
impl<'a> Roots<'a> {
    pub fn one(expression: ExprId) -> Self {
        Self {
            head: [Some(expression), None, None],
            tail: &[],
        }
    }
    pub fn iter(self) -> impl DoubleEndedIterator<Item = ExprId> + Clone + 'a {
        self.head
            .into_iter()
            .flatten()
            .chain(self.tail.iter().copied())
    }
    fn children(code: &'a MethodCode, expression: &'a Expression) -> Self {
        let mut roots = Self {
            head: [None; 3],
            tail: &[],
        };
        let kind = &expression.kind;
        match kind {
            ExprKind::Move {
                expression: value, ..
            }
            | ExprKind::Initialize {
                expression: value, ..
            }
            | ExprKind::CheckCast { value, .. }
            | ExprKind::Monitor { lock: value, .. }
            | ExprKind::FillArray { array: value, .. }
            | ExprKind::Unary { value, .. }
            | ExprKind::ArrayLength(value)
            | ExprKind::InstanceOf { value, .. } => roots.head[0] = Some(*value),
            ExprKind::Binary { left, right, .. }
            | ExprKind::Compare { left, right, .. }
            | ExprKind::Array {
                array: left,
                index: right,
                ..
            } => {
                roots.head[0] = Some(*left);
                roots.head[1] = Some(*right);
            }
            ExprKind::Field { receiver, .. } => roots.head[0] = *receiver,
            ExprKind::FieldStore {
                receiver, value, ..
            } => {
                let natural = expression
                    .operation
                    .is_some_and(|op| code.emission.natural_store(StatementId(op.0)));
                roots.head = if natural {
                    [*receiver, Some(*value), None]
                } else {
                    [Some(*value), *receiver, None]
                };
            }
            ExprKind::ArrayStore {
                array,
                index,
                value,
                ..
            } => {
                let natural = expression
                    .operation
                    .is_some_and(|op| code.emission.natural_store(StatementId(op.0)));
                roots.head = if natural {
                    [Some(*array), Some(*index), Some(*value)]
                } else {
                    [Some(*value), Some(*array), Some(*index)]
                };
            }
            ExprKind::NewArray { length, .. } => roots.head[0] = Some(*length),
            ExprKind::Call(call) => {
                roots.head[0] = call.receiver;
                if let Arguments::Known(arguments) = &call.arguments {
                    roots.tail = arguments;
                }
            }
            ExprKind::New { arguments, .. }
            | ExprKind::FilledArray {
                elements: Arguments::Known(arguments),
                ..
            } => roots.tail = arguments,
            _ => {}
        }
        roots
    }
    /// Operands a statement root contributes, in original evaluation order.
    /// A moving or initializing root contributes its value; a store contributes
    /// its receiver/index/value sequence. Roots without readable operands have
    /// no carrier question.
    pub fn statement(
        code: &'a MethodCode,
        statement: StatementId,
    ) -> Result<Option<Self>, ExpressionError> {
        let source = code
            .statement(statement)
            .ok_or(bug("expression roots lost their statement"))?;
        let root = &code.expressions[source.expression.0 as usize];
        Ok(Some(match root.kind {
            ExprKind::Move { .. } | ExprKind::Initialize { .. } => Self::one(source.value(code)),
            ExprKind::FieldStore { .. } | ExprKind::ArrayStore { .. } => Self::children(code, root),
            _ => return Ok(None),
        }))
    }
    pub fn terminal(
        code: &'a MethodCode,
        block: crate::p2::BlockId,
    ) -> Result<Self, ExpressionError> {
        let terminal = code
            .terminals
            .get(block.0 as usize)
            .ok_or(bug("expression roots lost their terminal"))?;
        let mut roots = Self {
            head: [None; 3],
            tail: &[],
        };
        match &terminal.kind {
            TerminalKind::Return(Some(value))
            | TerminalKind::Throw(value)
            | TerminalKind::Switch {
                selector: value, ..
            } => roots.head[0] = Some(*value),
            TerminalKind::If { condition, .. } => match condition {
                Condition::Zero { value, .. } => roots.head[0] = Some(*value),
                Condition::Pair { left, right, .. } => {
                    roots.head[0] = Some(*left);
                    roots.head[1] = Some(*right);
                }
            },
            TerminalKind::None | TerminalKind::Goto(_) | TerminalKind::Return(None) => {}
        }
        Ok(roots)
    }
}

#[derive(Clone, Copy)]
pub(super) enum Scope {
    Carrier,
    Selected,
}
#[derive(Clone, Copy)]
pub(super) enum Event {
    Enter(ExprId),
    End(ExprId),
    Nested(ExprId),
}

pub(super) struct Walk {
    work: Vec<Event>,
    scope: Scope,
}
impl Walk {
    pub fn new(roots: Roots<'_>, scope: Scope) -> Result<Self, ExpressionError> {
        let mut work = Vec::new();
        for root in roots.iter().rev() {
            push(&mut work, Event::Enter(root))?;
        }
        Ok(Self { work, scope })
    }
    pub fn next(&mut self, code: &MethodCode) -> Result<Option<Event>, ExpressionError> {
        let Some(event) = self.work.pop() else {
            return Ok(None);
        };
        let Event::Enter(id) = event else {
            return Ok(Some(event));
        };
        let expression = code
            .expressions
            .get(id.0 as usize)
            .ok_or(bug("expression walk left its arena"))?;
        if let Some(evaluation) = expression.evaluation {
            if matches!(self.scope, Scope::Selected) {
                push(&mut self.work, Event::Enter(evaluation.expression))?;
            }
            return Ok(Some(Event::Nested(id)));
        }
        push(&mut self.work, Event::End(id))?;
        for child in Roots::children(code, expression).iter().rev() {
            push(&mut self.work, Event::Enter(child))?;
        }
        Ok(Some(event))
    }
}

pub(super) fn occurrences(
    code: &MethodCode,
    roots: Roots<'_>,
    occurrence: ExprId,
) -> Result<u8, ExpressionError> {
    let mut walk = Walk::new(roots, Scope::Carrier)?;
    let mut seen = 0u8;
    while let Some(event) = walk.next(code)? {
        if matches!(event, Event::Enter(id) | Event::Nested(id) if id == occurrence) {
            seen = seen.saturating_add(1);
            if seen > 1 {
                break;
            }
        }
    }
    Ok(seen)
}
