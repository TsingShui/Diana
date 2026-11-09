//! Independent executor for the delivered scalar/reference fixture subset.
//! No CFG, producer analysis or reconstruction code is invoked here.

use diana_decomp::p2::BranchOp;
use diana_decomp::p5::{LocalKind, Materialization};
use diana_decomp::p7::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reference {
    Null,
    // Integer-only consumers do not construct the reference fixture values.
    #[allow(dead_code)]
    Array {
        identity: u32,
        length: i32,
    },
    Object(u32),
}
impl Reference {
    fn identity(self) -> Option<u32> {
        match self {
            Self::Null => None,
            Self::Array { identity, .. } | Self::Object(identity) => Some(identity),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Value {
    Int(i32),
    Reference(Reference),
    Unit,
}
impl Value {
    pub fn int(self) -> i32 {
        let Self::Int(value) = self else {
            panic!("expected integer")
        };
        value
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fault {
    Cast,
    NullArray,
    Helper(usize),
    Unsupported(&'static str),
}

pub struct Executor<'a, F> {
    code: &'a MethodCode,
    locals: Vec<Option<Value>>,
    calls: F,
}
impl<'a, F: FnMut(&str, &[Value]) -> Result<Value, Fault>> Executor<'a, F> {
    pub fn new(code: &'a MethodCode, parameters: &[Value], calls: F) -> Self {
        let mut locals = vec![None; code.locals().len()];
        for local in code.locals() {
            if let LocalKind::Parameter {
                role: diana_decomp::p3::EntryRole::Parameter(index),
                ..
            } = local.original.kind()
            {
                locals[local.original.representative().0 as usize] =
                    parameters.get(index as usize).copied();
            }
        }
        Self {
            code,
            locals,
            calls,
        }
    }
    fn cast(&self, value: Value, reference: ReferenceId) -> Result<Value, Fault> {
        let Ok(ReferenceDeclaration::Type { type_use }) =
            self.code.references()[reference.0 as usize].declaration
        else {
            return Err(Fault::Unsupported("cast type"));
        };
        if self.code.types()[type_use.0 as usize].units != "[I".encode_utf16().collect::<Vec<_>>() {
            return Err(Fault::Unsupported("cast target"));
        }
        match value {
            Value::Reference(Reference::Null | Reference::Array { .. }) => Ok(value),
            Value::Reference(Reference::Object(_)) => Err(Fault::Cast),
            _ => Err(Fault::Unsupported("cast operand")),
        }
    }
    pub fn expression(&mut self, id: ExprId, carrier: CheckCarrier) -> Result<Value, Fault> {
        let expression = &self.code.expressions()[id.0 as usize];
        if let ExprKind::Read { storage, .. } = expression.kind
            && let Some(check) = self.code.emission().inline_check(carrier, id, storage)
        {
            let value = self.locals[storage.0 as usize]
                .ok_or(Fault::Unsupported("unbound checked local"))?;
            return self.cast(value, check.reference);
        }
        if let Some(evaluation) = expression.evaluation {
            let source = &self.code.operations()[evaluation.operation.0 as usize];
            let statement = source
                .statement
                .ok_or(Fault::Unsupported("nested statement"))?;
            let value =
                self.expression(evaluation.expression, CheckCarrier::Statement(statement))?;
            if let EvaluationResult::Store(local) = evaluation.result {
                self.locals[local.0 as usize] = Some(value);
            }
            return Ok(value);
        }
        match &expression.kind {
            ExprKind::Read { storage, .. } => {
                self.locals[storage.0 as usize].ok_or(Fault::Unsupported("unbound local"))
            }
            ExprKind::Literal { bits, .. } if bits.width == diana_decomp::SlotWidth::One => {
                Ok(Value::Int((bits.value as u32) as i32))
            }
            ExprKind::Binary {
                op: diana_decomp::p2::BinaryOp::AddInt,
                left,
                right,
            } => {
                let left = self.expression(*left, carrier)?.int();
                Ok(Value::Int(
                    left.wrapping_add(self.expression(*right, carrier)?.int()),
                ))
            }
            ExprKind::ArrayLength(array) => match self.expression(*array, carrier)? {
                Value::Reference(Reference::Array { length, .. }) => Ok(Value::Int(length)),
                Value::Reference(Reference::Null) => Err(Fault::NullArray),
                _ => Err(Fault::Unsupported("array length operand")),
            },
            ExprKind::Call(call) => {
                if call.receiver.is_some() {
                    return Err(Fault::Unsupported("instance helper"));
                }
                let Arguments::Known(arguments) = &call.arguments else {
                    return Err(Fault::Unsupported("helper arguments"));
                };
                let values = arguments
                    .iter()
                    .map(|&argument| self.expression(argument, carrier))
                    .collect::<Result<Vec<_>, _>>()?;
                let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { name, .. })) =
                    &self.code.references()[call.target.0 as usize].declaration
                else {
                    return Err(Fault::Unsupported("helper target"));
                };
                (self.calls)(&String::from_utf16(&name.units).unwrap(), &values)
            }
            _ => Err(Fault::Unsupported("expression")),
        }
    }
    fn predicate(&mut self, test: PredicateEmission) -> Result<bool, Fault> {
        let phases = &self.code.emission().terminals()[test.terminal.0 as usize];
        if !phases.before.is_empty() || !phases.snapshots.is_empty() || !phases.after.is_empty() {
            return Err(Fault::Unsupported("predicate phases"));
        }
        let TerminalKind::If { condition, .. } =
            &self.code.terminals()[test.terminal.0 as usize].kind
        else {
            return Err(Fault::Unsupported("predicate"));
        };
        let carrier = CheckCarrier::Terminal(test.terminal);
        let (op, left, right) = match condition {
            Condition::Pair {
                op, left, right, ..
            } => (
                *op,
                self.expression(*left, carrier)?,
                self.expression(*right, carrier)?,
            ),
            Condition::Zero { op, value, .. } => {
                let left = self.expression(*value, carrier)?;
                let right = match left {
                    Value::Int(_) => Value::Int(0),
                    Value::Reference(_) => Value::Reference(Reference::Null),
                    _ => return Err(Fault::Unsupported("zero predicate")),
                };
                (*op, left, right)
            }
        };
        let equal = match (left, right) {
            (Value::Int(left), Value::Int(right)) => left == right,
            (Value::Reference(left), Value::Reference(right)) => {
                left.identity() == right.identity()
            }
            _ => return Err(Fault::Unsupported("comparison operands")),
        };
        let result = match op {
            BranchOp::Eq | BranchOp::Eqz => equal,
            BranchOp::Ne | BranchOp::Nez => !equal,
            _ => return Err(Fault::Unsupported("comparison operator")),
        };
        Ok(result != test.negate)
    }
    fn statement(
        &mut self,
        operation: diana_decomp::p2::OpId,
        id: StatementId,
    ) -> Result<(), Fault> {
        if self.code.nested_operation(operation)
            || self.code.origin_only_operation(operation)
            || self.code.emission().omitted_nop(id)
            || self.code.emission().omitted_literal(id)
            || self.code.emission().omitted_check(id).is_some()
        {
            return Ok(());
        }
        let phases = &self.code.emission().operations()[operation.0 as usize];
        if !phases.before.is_empty() || !phases.snapshots.is_empty() || !phases.after.is_empty() {
            return Err(Fault::Unsupported("statement phases"));
        }
        let statement = &self.code.statements()[id.0 as usize];
        let value = match statement.kind {
            StatementKind::Move { expression, .. } | StatementKind::Evaluate { expression } => {
                let value = self.expression(expression, CheckCarrier::Statement(id))?;
                if let Some(check) = self.code.emission().definition_check(id) {
                    self.cast(value, check.reference)?
                } else {
                    value
                }
            }
            StatementKind::CheckCast { value, reference } => {
                let value = self.expression(value, CheckCarrier::Statement(id))?;
                self.cast(value, reference)?;
                Value::Unit
            }
            StatementKind::CaptureException => Value::Reference(Reference::Object(u32::MAX)),
            _ => return Err(Fault::Unsupported("statement")),
        };
        for output in &statement.outputs {
            match output.materialization() {
                Materialization::Store(local) | Materialization::ExceptionCapture(local) => {
                    self.locals[local.0 as usize] = Some(value)
                }
                _ => return Err(Fault::Unsupported("materialization")),
            }
        }
        Ok(())
    }
    fn control(&mut self, id: ControlId) -> Result<Option<Value>, Fault> {
        let control = &self.code.controls()[id.0 as usize];
        if self.code.control_omitted(id) {
            return Ok(None);
        }
        if control.continuation.is_some() {
            return Err(Fault::Unsupported("control completion"));
        }
        match &control.layout {
            EmissionLayout::Sequence | EmissionLayout::Labelled => {
                for &child in control.children() {
                    if let Some(value) = self.control(child)? {
                        return Ok(Some(value));
                    }
                }
                Ok(None)
            }
            EmissionLayout::Block(node) => {
                let block = &self.code.emission().blocks()[node.0 as usize];
                for action in &block.actions {
                    match *action {
                        BodyAction::Statement {
                            operation,
                            statement,
                        } => self.statement(operation, statement)?,
                        BodyAction::Receive { handler }
                            if self.code.handlers()[handler as usize].receives.is_empty() => {}
                        _ => return Err(Fault::Unsupported("body event")),
                    }
                }
                match &block.end {
                    BlockEnd::Terminal(block) => {
                        let terminal = &self.code.terminals()[block.0 as usize];
                        let phases = &self.code.emission().terminals()[block.0 as usize];
                        if !phases.before.is_empty()
                            || !phases.snapshots.is_empty()
                            || !phases.after.is_empty()
                        {
                            return Err(Fault::Unsupported("terminal phases"));
                        }
                        match terminal.kind {
                            TerminalKind::Return(Some(value)) => Ok(Some(
                                self.expression(value, CheckCarrier::Terminal(*block))?,
                            )),
                            TerminalKind::Return(None) => Ok(Some(Value::Unit)),
                            _ => Err(Fault::Unsupported("terminal")),
                        }
                    }
                    BlockEnd::Transfers(transfers) => {
                        if transfers.iter().any(|transfer| {
                            !self.code.emission().transfers()[transfer.0 as usize]
                                .copies
                                .is_empty()
                                || !matches!(
                                    self.code.emission().transfers()[transfer.0 as usize].spelling,
                                    TransferSpelling::Fallthrough
                                )
                        }) {
                            return Err(Fault::Unsupported("transfer"));
                        }
                        Ok(None)
                    }
                    BlockEnd::None => Ok(None),
                }
            }
            EmissionLayout::If {
                header,
                yes,
                no,
                test,
                shape,
            } => {
                if self.control(*header)?.is_some() {
                    return Err(Fault::Unsupported("returning header"));
                }
                if matches!(shape, IfShape::Empty) {
                    return Ok(None);
                }
                let condition = self.predicate(*test)?;
                let inverted =
                    matches!(shape, IfShape::Inverted | IfShape::Guard { inverted: true });
                if matches!(shape, IfShape::Inverted) && !condition {
                    return Ok(None);
                }
                self.control(if condition != inverted { *yes } else { *no })
            }
            EmissionLayout::Condition {
                header,
                tests,
                yes,
                no,
                ..
            } => {
                if self.control(*header)?.is_some() {
                    return Err(Fault::Unsupported("returning condition header"));
                }
                for test in tests.iter() {
                    if self.predicate(*test)? {
                        return self.control(*yes);
                    }
                }
                match *no {
                    ConditionElse::Body(no) => self.control(no),
                    ConditionElse::None => Ok(None),
                    _ => Err(Fault::Unsupported("condition completion")),
                }
            }
            EmissionLayout::Try {
                body,
                catches,
                finally: None,
            } => match self.control(*body) {
                Err(Fault::Cast) => {
                    let ControlKind::Try {
                        bindings,
                        chain,
                        clauses,
                        ..
                    } = &control.kind
                    else {
                        return Err(Fault::Unsupported("try facts"));
                    };
                    if catches.len() != 1 {
                        return Err(Fault::Unsupported("catch count"));
                    }
                    let original =
                        &self.code.chains()[chain.0 as usize].clauses[clauses.start as usize];
                    let ty = original
                        .catch
                        .as_ref()
                        .and_then(|ty| ty.descriptor.as_ref().ok())
                        .ok_or(Fault::Unsupported("catch type"))?;
                    if self.code.types()[ty.0 as usize].units
                        != "Ljava/lang/ClassCastException;"
                            .encode_utf16()
                            .collect::<Vec<_>>()
                    {
                        return Err(Fault::Unsupported("catch model"));
                    }
                    if let Some(binding) = &bindings[0] {
                        self.locals[binding.local.0 as usize] =
                            Some(Value::Reference(Reference::Object(u32::MAX)));
                    }
                    self.control(catches[0])
                }
                result => result,
            },
            _ => Err(Fault::Unsupported("control")),
        }
    }
    pub fn run(&mut self) -> Result<Value, Fault> {
        if !self.code.emission().initial().is_empty() {
            return Err(Fault::Unsupported("entry copies"));
        }
        self.control(self.code.root().ok_or(Fault::Unsupported("method root"))?)?
            .ok_or(Fault::Unsupported("method exit"))
    }
}
