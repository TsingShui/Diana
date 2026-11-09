//! Test-owned execution of the fixture's final control/expression subset.
//! It does not use original CFG edges to repair missing final transfers.
use diana_decomp::p2::BinaryOp;
use diana_decomp::p3::EntryRole;
use diana_decomp::p5::{LocalKind, Materialization};
use diana_decomp::p7::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Value {
    Int(i32),
    Ref(u32),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    Thrown(u32),
    Unsupported,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    Enter(u32),
    Exit(u32),
    Field(&'static str),
    Size(&'static str),
    Read(i32),
    Note(u32),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
    Next,
    Return(Value),
}
#[derive(Default)]
pub struct Model {
    pub parameters: Vec<Value>,
    pub queue_a: u32,
    pub queue_s: u32,
    pub size_fault: Option<u32>,
    pub note_fault: bool,
    pub other_read_fault: bool,
    pub effects: Vec<Effect>,
    pub held: Vec<u32>,
    locals: HashMap<StorageId, Value>,
    pending: Option<u32>,
}
impl Model {
    pub fn new(parameters: Vec<Value>) -> Self {
        Self {
            parameters,
            ..Self::default()
        }
    }
    pub fn counts() -> Self {
        Self {
            queue_a: 10,
            queue_s: 11,
            ..Self::default()
        }
    }
    pub fn execute(&mut self, code: &MethodCode) -> Result<Value, Failure> {
        for local in code.locals() {
            if let LocalKind::Parameter { role, .. } = local.original.kind() {
                let value = match role {
                    EntryRole::This => Value::Ref(1),
                    EntryRole::Parameter(index) => *self
                        .parameters
                        .get(index as usize)
                        .ok_or(Failure::Unsupported)?,
                };
                self.locals.insert(local.id, value);
            }
        }
        match self.control(code, code.root().ok_or(Failure::Unsupported)?)? {
            Outcome::Return(value) => Ok(value),
            Outcome::Next => Err(Failure::Unsupported),
        }
    }
    fn enter(&mut self, lock: u32) -> Result<(), Failure> {
        self.effects.push(Effect::Enter(lock));
        if lock == 0 {
            return Err(Failure::Thrown(99));
        }
        self.held.push(lock);
        Ok(())
    }
    fn release(&mut self, lock: u32) -> Result<(), Failure> {
        self.effects.push(Effect::Exit(lock));
        if self.held.last().copied() != Some(lock) {
            return Err(Failure::Thrown(98));
        }
        self.held.pop();
        Ok(())
    }
    fn reference(value: Value) -> Result<u32, Failure> {
        match value {
            Value::Ref(v) => Ok(v),
            _ => Err(Failure::Unsupported),
        }
    }
    fn integer(value: Value) -> Result<i32, Failure> {
        match value {
            Value::Int(v) => Ok(v),
            _ => Err(Failure::Unsupported),
        }
    }
    fn expression(&mut self, code: &MethodCode, id: ExprId) -> Result<Value, Failure> {
        if let Some(composed) = code.emission().composed_use(id) {
            return self.expression(code, composed.expression);
        }
        match &code.expressions()[id.0 as usize].kind {
            ExprKind::Read { storage, .. } | ExprKind::Snapshot { storage } => self
                .locals
                .get(storage)
                .copied()
                .ok_or(Failure::Unsupported),
            ExprKind::Literal { bits, view } => {
                Ok(if matches!(view, diana_decomp::p4::Interpretation::Null) {
                    Value::Ref(0)
                } else {
                    Value::Int(bits.value as i32)
                })
            }
            ExprKind::Binary {
                op: BinaryOp::AddInt,
                left,
                right,
            } => {
                let left = Self::integer(self.expression(code, *left)?)?;
                let right = Self::integer(self.expression(code, *right)?)?;
                Ok(Value::Int(left.wrapping_add(right)))
            }
            ExprKind::Field {
                reference,
                receiver: Some(receiver),
            } => {
                let object = Self::reference(self.expression(code, *receiver)?)?;
                if object != 1 {
                    return Err(Failure::Unsupported);
                }
                let Ok(ReferenceDeclaration::Member(MemberDeclaration::Field { name, .. })) =
                    &code.references()[reference.0 as usize].declaration
                else {
                    return Err(Failure::Unsupported);
                };
                match String::from_utf16(&name.units)
                    .map_err(|_| Failure::Unsupported)?
                    .as_str()
                {
                    "runningAsyncCalls" => {
                        self.effects.push(Effect::Field("A"));
                        Ok(Value::Ref(self.queue_a))
                    }
                    "runningSyncCalls" => {
                        self.effects.push(Effect::Field("S"));
                        Ok(Value::Ref(self.queue_s))
                    }
                    _ => Err(Failure::Unsupported),
                }
            }
            ExprKind::Call(call) => {
                let receiver = call
                    .receiver
                    .map(|id| self.expression(code, id))
                    .transpose()?;
                let Arguments::Known(arguments) = &call.arguments else {
                    return Err(Failure::Unsupported);
                };
                let values = arguments
                    .iter()
                    .map(|id| self.expression(code, *id))
                    .collect::<Result<Vec<_>, _>>()?;
                let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method {
                    owner, name, ..
                })) = &code.references()[call.target.0 as usize].declaration
                else {
                    return Err(Failure::Unsupported);
                };
                let owner = String::from_utf16(&code.types()[owner.0 as usize].units)
                    .map_err(|_| Failure::Unsupported)?;
                let name = String::from_utf16(&name.units).map_err(|_| Failure::Unsupported)?;
                match (owner.as_str(), name.as_str(), receiver, values.as_slice()) {
                    ("Ljava/util/ArrayDeque;", "size", Some(Value::Ref(0)), []) => {
                        Err(Failure::Thrown(99))
                    }
                    ("Ljava/util/ArrayDeque;", "size", Some(Value::Ref(queue)), []) => {
                        let (event, result) = match queue {
                            10 => ("A", 2),
                            11 => ("S", 3),
                            _ => return Err(Failure::Unsupported),
                        };
                        self.effects.push(Effect::Size(event));
                        if self.size_fault == Some(queue) {
                            Err(Failure::Thrown(queue + 100))
                        } else {
                            Ok(Value::Int(result))
                        }
                    }
                    ("Lfixture/MonitorSurface;", "read", None, [Value::Int(mode)]) => {
                        self.effects.push(Effect::Read(*mode));
                        if *mode == 1 {
                            Err(Failure::Thrown(100))
                        } else {
                            Ok(Value::Int(mode + 7))
                        }
                    }
                    (
                        "Lfixture/MonitorBoundaries;",
                        "read",
                        None,
                        [Value::Int(stage), Value::Int(mode)],
                    ) => {
                        self.effects.push(Effect::Read(*stage));
                        if self.other_read_fault {
                            Err(Failure::Thrown(200))
                        } else if mode & (1 << stage) != 0 {
                            Err(Failure::Thrown(100 + *stage as u32))
                        } else {
                            Ok(Value::Int(stage + 2))
                        }
                    }
                    (
                        "Lfixture/MonitorSurface;" | "Lfixture/MonitorBoundaries;",
                        "note",
                        None,
                        [Value::Ref(error)],
                    ) => {
                        self.effects.push(Effect::Note(*error));
                        if self.note_fault {
                            Err(Failure::Thrown(201))
                        } else {
                            Ok(Value::Int(0))
                        }
                    }
                    _ => Err(Failure::Unsupported),
                }
            }
            _ => Err(Failure::Unsupported),
        }
    }
    fn copies(&mut self, code: &MethodCode, ids: &[CopyPlanId]) -> Result<(), Failure> {
        for id in ids {
            if code.copies()[id.0 as usize]
                .assignments
                .iter()
                .any(|a| a.issue().is_some() || a.destination().is_err() || a.source().is_err())
            {
                return Err(Failure::Unsupported);
            }
            for step in &code.copies()[id.0 as usize].steps {
                match step {
                    CopyStatement::Save {
                        destination,
                        source,
                    } => {
                        let value = *self.locals.get(source).ok_or(Failure::Unsupported)?;
                        self.locals.insert(*destination, value);
                    }
                    CopyStatement::Assign {
                        destination,
                        expression,
                        ..
                    } => {
                        let value = self.expression(code, *expression)?;
                        self.locals.insert(*destination, value);
                    }
                }
            }
        }
        Ok(())
    }
    fn statement(&mut self, code: &MethodCode, id: StatementId) -> Result<(), Failure> {
        let statement = &code.statements()[id.0 as usize];
        let value = match statement.kind {
            StatementKind::Evaluate { expression } | StatementKind::Move { expression, .. } => {
                self.expression(code, expression)?
            }
            StatementKind::CaptureException => {
                Value::Ref(self.pending.ok_or(Failure::Unsupported)?)
            }
            StatementKind::Nop => return Ok(()),
            _ => return Err(Failure::Unsupported),
        };
        for output in &statement.outputs {
            match output.materialization() {
                Materialization::Store(local) | Materialization::ExceptionCapture(local) => {
                    self.locals.insert(StorageId(local.0), value);
                }
                Materialization::Literal(_) => {}
                _ => return Err(Failure::Unsupported),
            }
        }
        Ok(())
    }
    fn transfer(&mut self, code: &MethodCode, id: TransferId) -> Result<(), Failure> {
        let transfer = &code.emission().transfers()[id.0 as usize];
        self.copies(code, &transfer.copies)?;
        if !matches!(
            transfer.spelling,
            TransferSpelling::Fallthrough | TransferSpelling::Selected
        ) {
            return Err(Failure::Unsupported);
        }
        Ok(())
    }
    fn block(
        &mut self,
        code: &MethodCode,
        node: diana_decomp::p6::NodeId,
    ) -> Result<Outcome, Failure> {
        let block = &code.emission().blocks()[node.0 as usize];
        for action in &block.actions {
            match *action {
                BodyAction::Statement {
                    operation,
                    statement,
                } if !code.emission().origin_only_operation(operation)
                    && !code.emission().composed_operation(operation) =>
                {
                    let phases = &code.emission().operations()[operation.0 as usize];
                    self.copies(code, &phases.before)?;
                    if !phases.snapshots.is_empty() {
                        return Err(Failure::Unsupported);
                    }
                    self.statement(code, statement)?;
                    self.copies(code, &phases.after)?;
                }
                BodyAction::Release {
                    operation,
                    statement,
                    ..
                } if !code.emission().origin_only_operation(operation) => {
                    let StatementKind::Monitor { lock, .. } =
                        code.statements()[statement.0 as usize].kind
                    else {
                        return Err(Failure::Unsupported);
                    };
                    let lock = Self::reference(self.expression(code, lock)?)?;
                    self.release(lock)?;
                }
                BodyAction::Receive { handler } => {
                    self.copies(code, &code.handlers()[handler as usize].receives)?
                }
                BodyAction::Statement { .. } | BodyAction::Release { .. } => {}
                _ => return Err(Failure::Unsupported),
            }
        }
        match &block.end {
            BlockEnd::None => Ok(Outcome::Next),
            BlockEnd::Transfers(transfers) => {
                for &id in transfers {
                    self.transfer(code, id)?;
                }
                Ok(Outcome::Next)
            }
            BlockEnd::Terminal(id) => {
                if code.emission().origin_only_terminal(*id) {
                    return Ok(Outcome::Next);
                }
                let phases = &code.emission().terminals()[id.0 as usize];
                self.copies(code, &phases.before)?;
                if !phases.snapshots.is_empty() {
                    return Err(Failure::Unsupported);
                }
                match code.terminals()[id.0 as usize].kind {
                    TerminalKind::Return(Some(expression)) => {
                        Ok(Outcome::Return(self.expression(code, expression)?))
                    }
                    TerminalKind::Goto(id) => {
                        self.transfer(code, id)?;
                        Ok(Outcome::Next)
                    }
                    TerminalKind::Throw(expression) => {
                        let error = Self::reference(self.expression(code, expression)?)?;
                        Err(Failure::Thrown(if error == 0 { 99 } else { error }))
                    }
                    _ => Err(Failure::Unsupported),
                }
            }
        }
    }
    fn control(&mut self, code: &MethodCode, id: ControlId) -> Result<Outcome, Failure> {
        let source = &code.controls()[id.0 as usize];
        let emitted = &code.emission().controls()[id.0 as usize];
        if emitted.omitted {
            return Ok(Outcome::Next);
        }
        let result = match &emitted.layout {
            EmissionLayout::Sequence => {
                let mut outcome = Outcome::Next;
                for &child in emitted.children(source) {
                    outcome = self.control(code, child)?;
                    if !matches!(outcome, Outcome::Next) {
                        break;
                    }
                }
                Ok(outcome)
            }
            EmissionLayout::Block(node) => self.block(code, *node),
            EmissionLayout::Synchronized { body, implicit, .. } => {
                let ControlKind::Synchronized { lock, .. } = source.kind else {
                    return Err(Failure::Unsupported);
                };
                let lock = Self::reference(self.expression(code, lock)?)?;
                self.enter(lock)?;
                let result = self.control(code, *body);
                if *implicit {
                    self.release(lock)?;
                }
                result
            }
            EmissionLayout::Try {
                body,
                catches,
                finally: None,
            } => {
                let result = self.control(code, *body);
                if let Err(Failure::Thrown(error)) = result {
                    let ControlKind::Try {
                        chain,
                        handler_clauses,
                        bindings,
                        ..
                    } = &source.kind
                    else {
                        return Err(Failure::Unsupported);
                    };
                    let mut outcome = result;
                    for (index, &handler) in catches.iter().enumerate() {
                        let range = &handler_clauses[index];
                        let matched = code.chains()[chain.0 as usize].clauses
                            [range.start as usize..range.end as usize]
                            .iter()
                            .any(|clause| {
                                clause.catch.as_ref().is_none_or(|catch| {
                                    catch.descriptor.as_ref().is_ok_and(|id| {
                                        error == 100
                                            && code.types()[id.0 as usize].units
                                                == "Ljava/lang/IllegalStateException;"
                                                    .encode_utf16()
                                                    .collect::<Vec<_>>()
                                    })
                                })
                            });
                        if !matched {
                            continue;
                        }
                        let previous = self.pending.replace(error);
                        if let Some(binding) = bindings[index] {
                            self.locals.insert(binding.local, Value::Ref(error));
                        }
                        outcome = self.control(code, handler);
                        self.pending = previous;
                        break;
                    }
                    outcome
                } else {
                    result
                }
            }
            _ => Err(Failure::Unsupported),
        }?;
        if matches!(result, Outcome::Next) && emitted.continuation.is_some() {
            return Err(Failure::Unsupported);
        }
        Ok(result)
    }
}
