//! Adapt checked JVM instructions and declarations to the shared diff index.

use std::collections::{BTreeSet, HashMap, hash_map::Entry};

use diana_decomp::input::jvm::ClassFile;
use diana_decomp::input::jvm::bytecode::{Code, ConstantOperand, Instruction, Operands};

use crate::analysis::jvm::{JvmRoots, Location};
use crate::analysis::jvm_code::display;
use crate::analysis::{query::QueryError, runner};

use super::blocks::Flow;
use super::body::{CAPACITY, uint, value};
use super::{
    Body, ExpansionBudget, Format, Index, InventoryIssue, JvmDeclaration, Method, Op, Symbols,
};

mod constants;
use constants::{ConstantReader, DynamicSites};

const CLASSES_PER_CLAIM: usize = 32;
const RESOURCE_LIMIT: &str = "expanded JVM operands exceed the comparison index capacity";

pub(in crate::analysis::diff) fn build<T: Send>(
    roots: &JvmRoots<'_>,
    jobs: usize,
    symbols: &Symbols,
    publish: fn(Method) -> T,
) -> Result<Index<T>, QueryError> {
    let mut results = Vec::new();
    runner::run::<_, _, _, QueryError>(
        roots.len().div_ceil(CLASSES_PER_CLAIM),
        jobs.max(1),
        || roots.reader(),
        |reader, claim, emit| {
            let mut index = Index::default();
            let first = claim * CLASSES_PER_CLAIM;
            let last = first.saturating_add(CLASSES_PER_CLAIM).min(roots.len());
            for class in first..last {
                let root = roots.get(class).ok_or(QueryError::WorkerLost)?;
                let location = root.location();
                let result = reader
                    .read(root)
                    .and_then(|class| scan_class(&class, location, symbols, &mut index, publish));
                if let Err(error) = result {
                    index.issues.push(InventoryIssue {
                        at: Some(location.to_string()),
                        reason: error.to_string(),
                    });
                }
            }
            emit((claim, index))
        },
        &mut |result| {
            results.push(result);
            Ok(())
        },
    )?;
    let mut index = super::merge_ordered(results)?;
    index
        .issues
        .extend(roots.issues().iter().map(|issue| InventoryIssue {
            at: Some(issue.context.clone()),
            reason: issue.reason.clone(),
        }));
    Ok(index)
}

fn scan_class<T>(
    class: &ClassFile<'_>,
    location: &Location,
    symbols: &Symbols,
    index: &mut Index<T>,
    publish: fn(Method) -> T,
) -> Result<(), QueryError> {
    let owner = format!("L{};", display(class.name()?)?);
    let mut constants = ConstantReader::new(class, symbols);
    let mut budget = ExpansionBudget {
        remaining: class
            .encoded_len()
            .saturating_mul(64)
            .saturating_add(1 << 20),
    };
    for (position, member) in class.methods()?.enumerate() {
        let at = format!("{location}:m{position}");
        let method = (|| {
            let member = member?;
            let mut method = Method {
                at: at.clone(),
                owner: owner.clone(),
                name: constants::member_name(member.name(class)?).map_err(|reason| {
                    QueryError::Local {
                        kind: diana_decomp::FailureKind::ResourceExhausted,
                        reason,
                    }
                })?,
                prototype: display(member.descriptor(class)?)?,
                access: u32::from(member.access_flags()),
                jvm: None,
                body: None,
                issues: Vec::new(),
            };
            let signature = class
                .signature(member.attributes())
                .map_err(QueryError::from)
                .and_then(|signature| signature.map(display).transpose());
            let exceptions = class
                .declared_exceptions(member)
                .map_err(QueryError::from)
                .and_then(|entries| {
                    entries
                        .map(|entry| Ok(format!("L{};", display(entry?)?)))
                        .collect::<Result<Vec<_>, _>>()
                });
            let declaration = JvmDeclaration {
                signature: metadata(signature, &mut method.issues),
                exceptions: metadata(exceptions, &mut method.issues),
            };
            if !declaration.is_empty() {
                method.jvm = Some(Box::new(declaration));
            }
            let declaration_only = member.access_flags() & (0x0100 | 0x0400) != 0;
            match class.code_attribute(member) {
                Ok(Some(attribute)) => {
                    if declaration_only {
                        method
                            .issues
                            .push("native or abstract JVM method has Code".into());
                    }
                    match Code::parse_at(attribute.payload, attribute.offset) {
                        Ok(code) => {
                            collect_body(code, symbols, &mut constants, &mut budget, &mut method)
                        }
                        Err(error) => method.issues.push(error.to_string()),
                    }
                }
                Ok(None) if !declaration_only => {
                    method
                        .issues
                        .push("concrete JVM method is missing Code".into());
                }
                Ok(None) => {}
                Err(error) => method.issues.push(error.to_string()),
            }
            Ok::<_, QueryError>(method)
        })();

        match method {
            Ok(method) => {
                if !budget.charge(method.retained_bytes()) {
                    index.issues.push(InventoryIssue {
                        at: Some(at),
                        reason: RESOURCE_LIMIT.to_owned(),
                    });
                    break;
                }
                index
                    .methods
                    .try_reserve(1)
                    .map_err(crate::analysis::input::InputError::Allocation)?;
                index.methods.push(publish(method));
            }
            Err(error) => index.issues.push(InventoryIssue {
                at: Some(at),
                reason: error.to_string(),
            }),
        }
    }
    Ok(())
}

fn metadata<T>(result: Result<T, QueryError>, issues: &mut Vec<String>) -> Option<T> {
    match result {
        Ok(metadata) => Some(metadata),
        Err(error) => {
            issues.push(error.to_string());
            None
        }
    }
}

struct Layout<'a> {
    code: Code<'a>,
    instructions: Vec<Instruction<'a>>,
}

impl Layout<'_> {
    fn boundary(&self, pc: u16) -> Result<u32, String> {
        self.instructions
            .binary_search_by_key(&pc, |instruction| instruction.offset())
            .map(|index| index as u32)
            .map_err(|_| format!("no JVM instruction boundary at byte {pc}"))
    }

    fn end_boundary(&self, pc: u16) -> Result<u32, String> {
        if usize::from(pc) == self.code.bytes().len() && self.code.missing_bytes().is_none() {
            Ok(self.instructions.len() as u32)
        } else {
            self.boundary(pc)
        }
    }

    fn target(&self, instruction: Instruction<'_>, relative: i32) -> Result<u32, String> {
        let pc = instruction
            .target(relative)
            .map_err(|error| format!("{error:?}"))?;
        self.boundary(pc)
    }

    fn write_switch(
        &self,
        body: &mut Body,
        instruction: Instruction<'_>,
        switch: diana_decomp::input::jvm::bytecode::Switch<'_>,
    ) -> Result<(), String> {
        value(
            &mut body.operands,
            b'B',
            &self.target(instruction, switch.default)?.to_le_bytes(),
        )?;
        for (key, relative) in switch.cases() {
            let mut encoded = [0; 12];
            encoded[..8].copy_from_slice(&i64::from(key).to_le_bytes());
            encoded[8..].copy_from_slice(&self.target(instruction, relative)?.to_le_bytes());
            value(&mut body.operands, b'K', &encoded)?;
        }
        Ok(())
    }
}

fn collect_body<'a>(
    code: Code<'a>,
    symbols: &Symbols,
    constants: &mut ConstantReader<'_, 'a>,
    budget: &mut ExpansionBudget,
    method: &mut Method,
) {
    let mut body = Body {
        format: Format::Jvm,
        jvm_major: constants.class().major_version(),
        frame: [code.max_locals, 0, code.max_stack],
        code_units: code
            .missing_bytes()
            .map_or(code.bytes().len() as u32, |tail| u32::from(tail.end)),
        names: symbols.names(),
        complete_control: true,
        ..Body::default()
    };
    let mut layout = Layout {
        code,
        instructions: Vec::with_capacity(code.bytes().len().min(24)),
    };
    for instruction in code.instructions() {
        match instruction {
            Ok(instruction) => layout.instructions.push(instruction),
            Err(error) => {
                body.complete_control = false;
                method.issues.push(error.to_string());
                break;
            }
        }
    }
    if let Some(tail) = code.missing_bytes() {
        body.complete_control = false;
        method.issues.push(format!(
            "missing JVM Code bytes {}..{}",
            tail.start, tail.end
        ));
    }
    if let Err(error) = body.code.try_reserve_exact(layout.instructions.len()) {
        method
            .issues
            .push(format!("JVM Code index allocation failed: {error}"));
        return;
    }
    let mut dynamic = DynamicSites::default();
    for &instruction in &layout.instructions {
        let (opcode, local) = operation(instruction);
        let from = body.operands.len();
        let registers = body.raw_registers.len();
        let calls = body.calls.len();
        let anchors = body.anchors.len();
        uint(&mut body.operands, u32::from(local.is_some()));
        if let Some(local) = local {
            uint(&mut body.operands, u32::from(local));
            body.raw_registers.push(local);
        }
        if matches!(opcode, 0xa8 | 0xa9) {
            body.complete_control = false;
            method.issues.push(format!(
                "@{}: legacy JVM subroutine transfer has no complete comparison graph",
                instruction.offset()
            ));
        }
        let operands = instruction.operands();
        let result = if let Some(problem) = instruction.operand_problem() {
            Err(format!("{problem:?}"))
        } else {
            match operands {
                Operands::Branch(relative) => layout
                    .target(instruction, relative)
                    .and_then(|target| value(&mut body.operands, b'B', &target.to_le_bytes())),
                Operands::Switch(switch) => layout.write_switch(&mut body, instruction, switch),
                Operands::Immediate(number) => {
                    value(&mut body.operands, b'I', &number.to_le_bytes())
                }
                Operands::Increment {
                    value: increment, ..
                } => value(
                    &mut body.operands,
                    b'I',
                    &i32::from(increment).to_le_bytes(),
                ),
                Operands::ArrayType(ty) => value(&mut body.operands, b'E', &[ty]),
                Operands::Constant { index, extra } => {
                    let result =
                        constants.instruction(&mut body, &mut dynamic, budget, instruction, index);
                    result.and_then(|()| match extra {
                        Some(
                            ConstantOperand::InterfaceSlots(slots)
                            | ConstantOperand::Dimensions(slots),
                        ) => value(&mut body.operands, b'E', &[slots]),
                        None => Ok(()),
                    })
                }
                Operands::Local(_) | Operands::None => Ok(()),
            }
        };
        let known_operands = result.is_ok();
        if let Err(error) = result {
            if matches!(operands, Operands::Branch(_) | Operands::Switch(_)) {
                body.complete_control = false;
            }
            body.operands.truncate(from);
            body.raw_registers.truncate(registers);
            body.calls.truncate(calls);
            body.anchors.truncate(anchors);
            uint(&mut body.operands, 0);
            method
                .issues
                .push(format!("@{}: {error}", instruction.offset()));
            if matches!(error.as_str(), CAPACITY | RESOURCE_LIMIT) {
                body.complete_control = false;
                break;
            }
        }
        body.code.push(Op {
            pc: u32::from(instruction.offset()),
            opcode,
            shape: u16::from(opcode) << 8 | u16::from(local.is_some()),
            end: body.operands.len() as u32,
            register_end: body.raw_registers.len() as u32,
            flow: Flow::jvm(instruction),
            known_operands,
        });
    }

    if body.code.len() == layout.instructions.len()
        && let Err(error) = protection(&layout, &mut body, constants.class(), &mut method.issues)
    {
        body.complete_control = false;
        method.issues.push(error);
    }
    body.anchors.sort_unstable();
    body.anchors.dedup();
    body.calls.sort_unstable();
    body.calls.dedup();
    method.body = Some(body);
}

// Encoding width and embedded local forms do not change the operation.
fn operation(instruction: Instruction<'_>) -> (u8, Option<u16>) {
    let opcode = instruction
        .wide_opcode()
        .unwrap_or_else(|| instruction.opcode());
    match opcode {
        0x1a..=0x2d => (
            0x15 + (opcode - 0x1a) / 4,
            Some(u16::from((opcode - 0x1a) % 4)),
        ),
        0x3b..=0x4e => (
            0x36 + (opcode - 0x3b) / 4,
            Some(u16::from((opcode - 0x3b) % 4)),
        ),
        0x13 => (0x12, None),
        0xc8 => (0xa7, None),
        0xc9 => (0xa8, None),
        _ => (
            opcode,
            match instruction.operands() {
                Operands::Local(local) | Operands::Increment { local, .. } => Some(local),
                _ => None,
            },
        ),
    }
}

struct Handler {
    target: u32,
    ty: Option<u16>,
}

fn protection(
    layout: &Layout<'_>,
    body: &mut Body,
    class: &ClassFile<'_>,
    issues: &mut Vec<String>,
) -> Result<(), String> {
    let rows = layout
        .code
        .exception_handlers()
        .map_err(|error| error.to_string())?;
    if let Some(error) = rows.issue() {
        body.complete_control = false;
        issues.push(error.to_string());
    }
    if rows.len() == 0 {
        return Ok(());
    }
    let mut handlers = Vec::with_capacity(rows.len());
    let mut endpoints = Vec::with_capacity(rows.len() * 2);
    let mut names = HashMap::new();
    for (ordinal, row) in rows.enumerate() {
        let row_result = (|| {
            if let Some(problem) = row.range_problem(layout.code) {
                return Err(format!("{problem:?}"));
            }
            let start = layout.boundary(row.start)?;
            let end = layout.end_boundary(row.end)?;
            if let Some(index) = row.catch_type
                && let Entry::Vacant(entry) = names.entry(index)
            {
                let name = class.class_name(index).map_err(|error| error.to_string())?;
                if name.bytes().starts_with(b"[") {
                    return Err("exception type must name a class".to_owned());
                }
                let name = display(name).map_err(|error| error.to_string())?;
                entry.insert(format!("L{name};"));
            }
            Ok((
                start,
                end,
                Handler {
                    target: layout.boundary(row.target)?,
                    ty: row.catch_type,
                },
            ))
        })();
        match row_result {
            Ok((start, end, handler)) => {
                let index = handlers.len();
                endpoints.push((start, true, index));
                endpoints.push((end, false, index));
                handlers.push(handler);
            }
            Err(error) => {
                body.complete_control = false;
                issues.push(format!(
                    "JVM exception row {ordinal} [{}..{}) -> {}: {error}",
                    row.start, row.end, row.target
                ));
            }
        }
    }
    endpoints.sort_unstable();
    let mut active = BTreeSet::<usize>::new();
    let mut start = 0;
    // A JVM handler table may overlap. The same comparison consumes disjoint
    // intervals by recording the original row order covering each interval.
    for group in endpoints.chunk_by(|a, b| a.0 == b.0) {
        let end = group[0].0;
        if start < end && !active.is_empty() {
            for node in [start, end, active.len() as u32] {
                uint(&mut body.protection, node);
            }
            for &handler in &active {
                let handler = &handlers[handler];
                uint(&mut body.protection, handler.target);
                match handler.ty {
                    Some(ty) => {
                        body.has_symbol_dependency = true;
                        value(&mut body.protection, b't', names[&ty].as_bytes())?;
                    }
                    None => value(&mut body.protection, b'*', &[])?,
                }
            }
        }
        for &(_, begins, index) in group {
            if begins {
                active.insert(index);
            } else {
                active.remove(&index);
            }
        }
        start = end;
    }
    Ok(())
}
