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
    /// A selected read has no executed definition; this is a correspondence
    /// failure, distinct from an unimplemented operation in the test model.
    MissingValue(StorageId),
    #[allow(dead_code)]
    Runtime,
    #[allow(dead_code)]
    Allocation,
    #[allow(dead_code)]
    NullLock,
    Cast,
    NullArray,
    // Only the virtual-call preparation fixture constructs this modeled fault.
    #[allow(dead_code)]
    NullReceiver,
    #[allow(dead_code)]
    Helper(usize),
    Unsupported(&'static str),
}

pub struct Executor<'a, F> {
    code: &'a MethodCode,
    locals: Vec<Option<Value>>,
    calls: F,
    jump: Option<LabelId>,
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
            jump: None,
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
    pub fn expression(&mut self, id: ExprId) -> Result<Value, Fault> {
        let expression = &self.code.expressions()[id.0 as usize];
        if let Some(evaluation) = expression.evaluation {
            let source = &self.code.operations()[evaluation.operation.0 as usize];
            let statement = source
                .statement
                .ok_or(Fault::Unsupported("nested statement"))?;
            let value = self.expression(evaluation.expression)?;
            let value = match self.code.statement(statement).unwrap().kind {
                StatementKind::CheckCast { reference, .. } => self.cast(value, reference)?,
                _ => value,
            };
            if let EvaluationResult::Store(local) = evaluation.result {
                self.locals[local.0 as usize] = Some(value);
            }
            return Ok(value);
        }
        match &expression.kind {
            ExprKind::Read { storage, .. } | ExprKind::Snapshot { storage } => {
                self.locals[storage.0 as usize].ok_or(Fault::MissingValue(*storage))
            }
            ExprKind::Literal { bits, .. } if bits.width == diana_decomp::SlotWidth::One => {
                Ok(Value::Int((bits.value as u32) as i32))
            }
            ExprKind::Binary {
                op: diana_decomp::p2::BinaryOp::AddInt,
                left,
                right,
            } => {
                let left = self.expression(*left)?.int();
                Ok(Value::Int(
                    left.wrapping_add(self.expression(*right)?.int()),
                ))
            }
            ExprKind::ArrayLength(array) => match self.expression(*array)? {
                Value::Reference(Reference::Array { length, .. }) => Ok(Value::Int(length)),
                Value::Reference(Reference::Null) => Err(Fault::NullArray),
                _ => Err(Fault::Unsupported("array length operand")),
            },
            ExprKind::String(reference) => {
                let Ok(ReferenceDeclaration::String { units }) =
                    &self.code.references()[reference.0 as usize].declaration
                else {
                    return Err(Fault::Unsupported("string input"));
                };
                (self.calls)(
                    "$string",
                    &units
                        .iter()
                        .map(|&unit| Value::Int(i32::from(unit)))
                        .collect::<Vec<_>>(),
                )
            }
            ExprKind::Call(call) => {
                let Arguments::Known(arguments) = &call.arguments else {
                    return Err(Fault::Unsupported("helper arguments"));
                };
                let mut values = Vec::new();
                if let Some(receiver) = call.receiver {
                    values.push(self.expression(receiver)?);
                }
                let explicit = arguments
                    .iter()
                    .map(|&argument| self.expression(argument))
                    .collect::<Result<Vec<_>, _>>()?;
                values.extend(explicit);
                let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { name, .. })) =
                    &self.code.references()[call.target.0 as usize].declaration
                else {
                    return Err(Fault::Unsupported("helper target"));
                };
                (self.calls)(&String::from_utf16(&name.units).unwrap(), &values)
            }
            ExprKind::New {
                constructor,
                arguments,
                ..
            } => {
                let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { name, .. })) =
                    &self.code.references()[constructor.0 as usize].declaration
                else {
                    return Err(Fault::Unsupported("constructor declaration"));
                };
                if name.units != "<init>".encode_utf16().collect::<Vec<_>>() {
                    return Err(Fault::Unsupported("constructor name"));
                }
                let allocated = (self.calls)("$allocate", &[])?;
                let values = arguments
                    .iter()
                    .map(|&argument| self.expression(argument))
                    .collect::<Result<Vec<_>, _>>()?;
                (self.calls)("$construct", &values)?;
                Ok(allocated)
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
        let (op, left, right) = match condition {
            Condition::Pair {
                op, left, right, ..
            } => (*op, self.expression(*left)?, self.expression(*right)?),
            Condition::Zero { op, value, .. } => {
                let left = self.expression(*value)?;
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
            BranchOp::Lt | BranchOp::Ltz => left.int() < right.int(),
            BranchOp::Ge | BranchOp::Gez => left.int() >= right.int(),
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
        let statement = &self.code.statement(id).unwrap();
        self.copies(&phases.before)?;
        // This subset admits snapshots only before calls with plain operands.
        // Acting operands require capture/continuation support not modeled here.
        if !phases.snapshots.is_empty() {
            let StatementKind::Evaluate { expression } = statement.kind else {
                return Err(Fault::Unsupported("snapshot statement"));
            };
            let ExprKind::Call(call) = &self.code.expressions()[expression.0 as usize].kind else {
                return Err(Fault::Unsupported("snapshot effect"));
            };
            let Arguments::Known(arguments) = &call.arguments else {
                return Err(Fault::Unsupported("snapshot arguments"));
            };
            if call
                .receiver
                .into_iter()
                .chain(arguments.iter().copied())
                .any(|id| {
                    let expression = &self.code.expressions()[id.0 as usize];
                    expression.evaluation.is_some()
                        || !matches!(
                            expression.kind,
                            ExprKind::Read { .. } | ExprKind::Literal { .. }
                        )
                })
            {
                return Err(Fault::Unsupported("acting snapshot operand"));
            }
            self.copies(&phases.snapshots)?;
        }
        let value = match statement.kind {
            StatementKind::Move { expression, .. } | StatementKind::Evaluate { expression } => {
                let value = self.expression(expression)?;
                if let Some(check) = self.code.emission().definition_check(id) {
                    self.cast(value, check.reference)?
                } else {
                    value
                }
            }
            StatementKind::CheckCast { value, reference } => {
                let value = self.expression(value)?;
                self.cast(value, reference)?;
                Value::Unit
            }
            StatementKind::CaptureException => Value::Reference(Reference::Object(u32::MAX)),
            StatementKind::Construction(group) => {
                let construction = &self.code.constructions()[group.0 as usize];
                let value = self.expression(construction.expression)?;
                for publication in &construction.publications {
                    self.locals[publication.destination.0 as usize] = Some(value);
                }
                value
            }
            _ => return Err(Fault::Unsupported("statement")),
        };
        for output in &statement.outputs {
            match output.materialization() {
                Materialization::Store(local)
                | Materialization::ExceptionCapture(local)
                | Materialization::Deferred { local, .. } => {
                    self.locals[local.0 as usize] = Some(value)
                }
                _ => return Err(Fault::Unsupported("materialization")),
            }
        }
        self.copies(&phases.after)?;
        Ok(())
    }
    fn copies(&mut self, ids: &[CopyPlanId]) -> Result<(), Fault> {
        for &id in ids {
            let copy = &self.code.copies()[id.0 as usize];
            if copy
                .assignments
                .iter()
                .any(|assignment| assignment.issue().is_some())
            {
                return Err(Fault::Unsupported("unavailable copy"));
            }
            for step in &copy.steps {
                let (destination, value) = match *step {
                    CopyStatement::Save {
                        destination,
                        source,
                    } => (
                        destination,
                        self.locals[source.0 as usize].ok_or(Fault::Unsupported("copy source"))?,
                    ),
                    CopyStatement::Assign {
                        destination,
                        expression,
                        ..
                    } => (destination, self.expression(expression)?),
                };
                self.locals[destination.0 as usize] = Some(value);
            }
        }
        Ok(())
    }
    fn contains(&self, root: ControlId, target: ControlId) -> bool {
        root == target
            || self.code.controls()[root.0 as usize]
                .children()
                .iter()
                .any(|&child| self.contains(child, target))
    }
    fn transfer(&mut self, id: TransferId) -> Result<Option<Value>, Fault> {
        let transfer = &self.code.emission().transfers()[id.0 as usize];
        self.copies(&transfer.copies)?;
        match transfer.spelling {
            TransferSpelling::Fallthrough => {}
            TransferSpelling::Goto(label) => self.jump = Some(label),
            _ => return Err(Fault::Unsupported("transfer")),
        }
        Ok(None)
    }
    fn control(&mut self, id: ControlId) -> Result<Option<Value>, Fault> {
        let control = &self.code.controls()[id.0 as usize];
        if self.code.control_omitted(id) {
            return Ok(None);
        }
        if let Some(label) = self.jump {
            let target = self.code.labels()[label.0 as usize].owner;
            if id == target {
                self.jump = None;
            } else if !self.contains(id, target) {
                return Ok(None);
            } else if !matches!(
                control.layout,
                EmissionLayout::Sequence | EmissionLayout::Labelled | EmissionLayout::Graph
            ) {
                return Err(Fault::Unsupported("jump into conditional scope"));
            }
        }
        let result = match &control.layout {
            EmissionLayout::Sequence | EmissionLayout::Labelled | EmissionLayout::Graph => {
                let children = control.children();
                let mut index = 0;
                while index < children.len() {
                    if let Some(value) = self.control(children[index])? {
                        return Ok(Some(value));
                    }
                    if let Some(label) = self.jump {
                        let target = self.code.labels()[label.0 as usize].owner;
                        let Some(next) = children
                            .iter()
                            .position(|&child| self.contains(child, target))
                        else {
                            return Ok(None);
                        };
                        if next <= index {
                            return Err(Fault::Unsupported("nonforward test jump"));
                        }
                        index = next;
                    } else {
                        index += 1;
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
                        BodyAction::Receive { handler } => {
                            self.copies(&self.code.handlers()[handler as usize].receives)?
                        }
                        BodyAction::Release { operation, .. }
                            if self.code.origin_only_operation(operation) => {}
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
                            TerminalKind::Return(Some(value)) => Ok(Some(self.expression(value)?)),
                            TerminalKind::Return(None) => Ok(Some(Value::Unit)),
                            TerminalKind::Goto(transfer) => self.transfer(transfer),
                            TerminalKind::If {
                                when_true,
                                when_false,
                                ..
                            } => {
                                if self.code.emission().if_display(*block) == IfDisplay::Omitted {
                                    return Ok(None);
                                }
                                let test = PredicateEmission {
                                    terminal: *block,
                                    when_true,
                                    when_false,
                                    negate: false,
                                };
                                let transfer = if self.predicate(test)? {
                                    when_true
                                } else {
                                    when_false
                                };
                                self.transfer(transfer)
                            }
                            _ => Err(Fault::Unsupported("terminal")),
                        }
                    }
                    BlockEnd::Transfers(transfers) => {
                        for transfer in transfers {
                            let transfer = &self.code.emission().transfers()[transfer.0 as usize];
                            self.copies(&transfer.copies)?;
                            match transfer.spelling {
                                TransferSpelling::Fallthrough => {}
                                TransferSpelling::Goto(label) => {
                                    self.jump = Some(label);
                                    break;
                                }
                                _ => return Err(Fault::Unsupported("transfer")),
                            }
                        }
                        Ok(None)
                    }
                    BlockEnd::None => Ok(None),
                    BlockEnd::Annotated { .. } => {
                        Err(Fault::Unsupported("protocol correspondence"))
                    }
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
                Err(fault @ (Fault::Cast | Fault::Runtime)) => {
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
                    let expected = if fault == Fault::Cast {
                        "Ljava/lang/ClassCastException;"
                    } else {
                        "Ljava/lang/RuntimeException;"
                    };
                    if self.code.types()[ty.0 as usize].units
                        != expected.encode_utf16().collect::<Vec<_>>()
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
            EmissionLayout::Synchronized {
                implicit: true,
                body,
                acquire,
                ..
            } => {
                let operation = &self.code.operations()[acquire.0 as usize];
                let statement = self
                    .code
                    .statement(
                        operation
                            .statement
                            .ok_or(Fault::Unsupported("acquire body"))?,
                    )
                    .unwrap();
                let StatementKind::Monitor { enter: true, lock } = statement.kind else {
                    return Err(Fault::Unsupported("acquire semantics"));
                };
                let value = self.expression(lock)?;
                (self.calls)("$acquire", &[value])?;
                let result = self.control(*body);
                (self.calls)("$release", &[value])?;
                result
            }
            _ => Err(Fault::Unsupported("control")),
        };
        if matches!(result, Ok(None)) && self.jump.is_none() {
            match control.continuation {
                None => {}
                Some(ControlCompletion::Goto(label)) => self.jump = Some(label),
                _ => return Err(Fault::Unsupported("control completion")),
            }
        }
        result
    }
    pub fn run(&mut self) -> Result<Value, Fault> {
        if !self.code.emission().initial().is_empty() {
            return Err(Fault::Unsupported("entry copies"));
        }
        self.control(self.code.root().ok_or(Fault::Unsupported("method root"))?)?
            .ok_or(Fault::Unsupported("method exit"))
    }
}
