//! Establish JVM frame shapes and produce the ordinary register CFG.

mod ops;

use std::collections::{BTreeSet, VecDeque};

use smallvec::SmallVec;

use super::bytecode::{Code, Instruction, Operands};
use super::{Attribute, ClassFile};
use crate::engine::jvm::{
    metadata::{self, LocalCoordinates},
    SourceError, SourceIssue,
};
use crate::frontend::{ClassNode, DebugInfo, MethodNode};
use crate::ir::{
    ArgType, ExceptionHandler, IfOp, InsnArg, InsnNode, MethodContext, MethodDescriptor,
    RegisterArg, Splitter, CFG,
};
use crate::FailureKind;
use ops::{OpError, Registers, Stack};

// The register IR sorts instructions by offset. One JVM instruction can
// contribute register copies. Reserve ordered slots within each original PC
// and leave room for incoming local bindings before original byte zero.
const PC_STRIDE: u32 = 32;
const FRAME_WORK_LIMIT: usize = 2_000_000;
const FRAME_VALUE_LIMIT: usize = 262_144;
const HANDLER_EDGE_LIMIT: usize = 262_144;

fn ir_pc(offset: u16) -> u32 {
    (u32::from(offset) + 1) * PC_STRIDE
}

#[derive(Clone)]
struct Handler {
    start: u16,
    end: u16,
    target: u16,
    ty: Option<ArgType>,
}

impl Handler {
    fn raw(&self) -> ExceptionHandler {
        ExceptionHandler::new(
            ir_pc(self.start),
            ir_pc(self.end),
            ir_pc(self.target),
            self.ty.clone(),
        )
    }
}

struct BasicBlock {
    first: usize,
    end: usize,
    successors: SmallVec<[usize; 2]>,
}

pub(crate) fn recover(
    class: &ClassFile<'_>,
    attribute: Attribute<'_>,
    owner: &ClassNode,
    method: &MethodNode,
    issues: &mut Vec<SourceIssue>,
) -> Result<CFG, SourceError> {
    let code = Code::parse_at(attribute.payload, attribute.offset)
        .map_err(|error| SourceError::Malformed(error.to_string()))?;
    let mut instructions = Vec::new();
    let mut tail = None;
    let mut dynamic = false;
    let ordinal = method.id as u16;
    for decoded in code.instructions() {
        match decoded {
            Ok(instruction) => {
                dynamic |= instruction.opcode() == 0xba;
                instructions.push(instruction);
            }
            Err(error) => {
                tail = Some(error.range.start);
                issues.push(issue(
                    ordinal,
                    Some(error.range.start),
                    Some(u32::from(error.range.end)),
                    error.kind(),
                    error,
                ));
                break;
            }
        }
    }
    if let Some(missing) = code.missing_bytes() {
        let start = *tail.get_or_insert(missing.start);
        if start >= missing.start && !issues.iter().any(|issue| issue.offset_byte == Some(start)) {
            issues.push(issue(
                ordinal,
                Some(missing.start),
                Some(u32::from(missing.end)),
                FailureKind::Malformed,
                "Code attribute is truncated",
            ));
        }
    }
    let present_end = code.bytes().len() as u16;
    let mut positions = vec![None; usize::from(present_end) + 1];
    for (index, instruction) in instructions.iter().enumerate() {
        positions[usize::from(instruction.offset())] = Some(index);
    }
    positions[usize::from(present_end)] = Some(instructions.len());
    let index_at = |pc: u16| positions.get(usize::from(pc)).copied().flatten();
    let valid_boundary = |pc: u16| index_at(pc).is_some();

    let mut handlers = Vec::new();
    match code.exception_handlers() {
        Ok(rows) => {
            if let Some(error) = rows.issue() {
                issues.push(issue(ordinal, None, None, FailureKind::Malformed, error));
            }
            for row in rows {
                let ty = row
                    .catch_type
                    .map(|index| crate::engine::jvm::class_type(class.class_name(index)?))
                    .transpose();
                if row.range_problem(code).is_some()
                    || !valid_boundary(row.start)
                    || !valid_boundary(row.end)
                    || !index_at(row.target).is_some_and(|position| position < instructions.len())
                {
                    issues.push(issue(
                        ordinal,
                        Some(row.start),
                        Some(u32::from(row.end)),
                        FailureKind::Malformed,
                        format!(
                            "invalid JVM exception range or handler target at byte {}",
                            row.target
                        ),
                    ));
                } else {
                    match ty {
                        Ok(ty) => handlers.push(Handler {
                            start: row.start,
                            end: row.end,
                            target: row.target,
                            ty,
                        }),
                        Err(error) => issues.push(issue(
                            ordinal,
                            Some(row.start),
                            Some(u32::from(row.end)),
                            error.kind(),
                            error,
                        )),
                    }
                }
            }
        }
        Err(error) => issues.push(issue(ordinal, None, None, FailureKind::Malformed, error)),
    }

    let handlers = partition_handlers(handlers, &instructions)?;
    let writes = (dynamic && tail.is_none() && !method.info.is_constructor())
        .then(|| ops::written_parameter_slots(&instructions).ok())
        .flatten();
    let (entry, inputs, input_words) =
        entry_frame(owner, method, writes.as_ref()).map_err(op_error)?;
    let registers =
        Registers::new(code.max_locals, code.max_stack, input_words).map_err(op_error)?;
    let debug_info = metadata::debug_info(
        class,
        code,
        method,
        &inputs,
        |slot, end| {
            if !valid_boundary(0) || !valid_boundary(end) {
                return None;
            }
            Some(LocalCoordinates {
                register: registers.local(slot).ok()?,
                start: ir_pc(0),
                end: ir_pc(end),
            })
        },
        issues,
    );
    let context = MethodContext::new(
        owner.class_type().clone(),
        method.info.name.clone(),
        MethodDescriptor {
            parameters: method.info.param_types.clone(),
            return_type: method.info.return_type.clone(),
        },
        method.is_static(),
    );
    let mut output = Vec::new();
    for (index, (slot, ty)) in inputs.into_iter().enumerate() {
        let param = registers.input_register(slot).map_err(op_error)?;
        let local = registers.local(slot).map_err(op_error)?;
        let mut bind = InsnNode::mov(RegisterArg::new(local, ty.clone()), InsnArg::reg(param, ty));
        // The incoming bindings share the non-original prelude. Their ordering
        // remains before PC zero even for a wide prototype.
        bind.offset = index.min((PC_STRIDE - 1) as usize) as u32;
        output.push(bind);
    }
    if instructions.is_empty() {
        let mut gap = InsnNode::unavailable("JVM Code has no readable instructions");
        gap.offset = ir_pc(0);
        output.push(gap);
        return Ok(split(context, registers, output, &handlers, debug_info));
    }
    let mut leaders = vec![false; instructions.len()];
    leaders[0] = true;
    let mut invalid_control = vec![false; instructions.len()];
    let mut targets = Vec::with_capacity(instructions.len());
    for (index, &instruction) in instructions.iter().enumerate() {
        let mut edges = SmallVec::<[usize; 2]>::new();
        if let Some(targets) = branch_targets(instruction) {
            if let Some(leader) = leaders.get_mut(index + 1) {
                *leader = true;
            }
            let mut add = |target| {
                if let Some(target) = instruction
                    .target(target)
                    .ok()
                    .and_then(index_at)
                    .filter(|&position| position < instructions.len())
                {
                    leaders[target] = true;
                    edges.push(target);
                } else {
                    invalid_control[index] = true;
                }
            };
            match targets {
                Branch::Direct(target) => add(target),
                Branch::Switch(switch) => {
                    add(switch.default);
                    for (_, target) in switch.cases() {
                        add(target);
                    }
                }
            }
        }
        targets.push(edges);
    }
    for handler in &handlers {
        for pc in [handler.start, handler.end, handler.target] {
            if let Some(leader) = index_at(pc).and_then(|position| leaders.get_mut(position)) {
                *leader = true;
            }
        }
    }
    for (index, &instruction) in instructions.iter().enumerate() {
        if is_terminal(instruction) {
            if let Some(leader) = leaders.get_mut(index + 1) {
                *leader = true;
            }
        }
    }
    let mut block_for = vec![0; instructions.len()];
    let mut blocks = Vec::new();
    let mut first = 0;
    for end in (1..instructions.len())
        .filter(|&position| leaders[position])
        .chain([instructions.len()])
    {
        block_for[first..end].fill(blocks.len());
        blocks.push(BasicBlock {
            first,
            end,
            successors: std::mem::take(&mut targets[end - 1]),
        });
        first = end;
    }
    for block in &mut blocks {
        let last = instructions[block.end - 1];
        if has_fallthrough(last) && block.end < block_for.len() {
            block.successors.push(block.end);
        }
        // Taken edges use instruction positions, including edges from a
        // switch that point back to the dispatch block.
        for successor in &mut block.successors {
            *successor = block_for[*successor];
        }
    }
    let mut catches = vec![None::<ArgType>; blocks.len()];
    for handler in &handlers {
        let position = index_at(handler.target).expect("checked JVM handler target");
        let ty = handler.ty.clone().unwrap_or_else(throwable);
        let catch = &mut catches[block_for[position]];
        *catch = Some(match catch.take() {
            Some(previous) if previous != ty => throwable(),
            _ => ty,
        });
    }
    let mut frames = if blocks.len() == 1 && blocks[0].successors.is_empty() && handlers.is_empty()
    {
        vec![Some(entry)]
    } else {
        frame_entries(
            class,
            &instructions,
            &blocks,
            &block_for,
            &handlers,
            &catches,
            &index_at,
            &invalid_control,
            registers,
            entry,
            ordinal,
            issues,
        )
    };
    for (index, block) in blocks.iter().enumerate() {
        let Some(mut frame) = frames[index].take() else {
            continue;
        };
        for position in block.first..block.end {
            let instruction = instructions[position];
            let offset = instruction.offset();
            let first = output.len();
            if position == block.first {
                if let Some(ty) = &catches[index] {
                    output.push(InsnNode::move_exception(RegisterArg::new(
                        registers.stack(0),
                        ty.clone(),
                    )));
                }
            }
            let start = output.len();
            let result = if invalid_control[position] {
                Err(OpError::Operand(
                    super::bytecode::OperandProblem::BranchOutsideCode,
                ))
            } else {
                ops::apply(class, instruction, &mut frame, registers, Some(&mut output))
            };
            if let Err(error) = result {
                let unavailable = output.get(start..).and_then(|nodes| {
                    nodes
                        .iter()
                        .find_map(|node| node.payload.unavailable.as_deref())
                });
                let reason =
                    unavailable.map_or_else(|| op_gap_reason(instruction, &error), str::to_owned);
                if unavailable.is_none() {
                    output.truncate(start);
                    if invalid_control[position]
                        || !emit_unavailable_control(
                            instruction,
                            method.return_type(),
                            registers.stack(code.max_stack),
                            &reason,
                            &mut output,
                        )
                    {
                        output.push(InsnNode::unavailable(reason.clone()));
                    }
                }
                let end = u32::from(offset) + instruction.bytes().len() as u32;
                issues.push(issue(
                    ordinal,
                    Some(offset),
                    Some(end),
                    error.kind(),
                    reason,
                ));
                if !error.retains_stack() {
                    frame.invalidate();
                }
            }
            if output.len() == first {
                output.push(InsnNode::nop());
            }
            for (delta, node) in output[first..].iter_mut().enumerate() {
                node.offset = ir_pc(offset) + delta as u32;
                if let Some(target) = &mut node.payload.target {
                    *target = ir_pc(*target as u16) as i32;
                }
                if let Some(cases) = node.payload.switch_cases.as_deref_mut() {
                    for (_, target) in cases {
                        *target = ir_pc(*target as u16) as i32;
                    }
                }
                if let Some(target) = &mut node.payload.switch_default {
                    *target = ir_pc(*target as u16) as i32;
                }
            }
        }
    }
    if let Some(offset) = tail {
        let mut gap =
            InsnNode::unavailable(format!("JVM byte {offset}: instruction layout unavailable"));
        gap.offset = ir_pc(offset);
        output.push(gap);
    }
    Ok(split(context, registers, output, &handlers, debug_info))
}

/// The JVM searches the original table in row order, including when ranges
/// overlap. The shared region analysis consumes disjoint protected intervals
/// with ordered handler lists.
fn partition_handlers(
    handlers: Vec<Handler>,
    instructions: &[Instruction<'_>],
) -> Result<Vec<Handler>, SourceError> {
    if handlers.is_empty() {
        return Ok(handlers);
    }
    let throwing: Vec<_> = instructions
        .iter()
        .copied()
        .filter(|instruction| instruction.may_throw())
        .map(Instruction::offset)
        .collect();
    let mut endpoints = Vec::with_capacity(handlers.len() * 2);
    for (index, handler) in handlers.iter().enumerate() {
        endpoints.push((handler.start, true, index));
        endpoints.push((handler.end, false, index));
    }
    endpoints.sort_unstable();
    let mut active = BTreeSet::new();
    let mut output = Vec::<Handler>::new();
    let mut previous_stack = 0..0;
    let mut work = 0usize;
    let mut edges = 0usize;
    let mut previous = endpoints[0].0;
    for group in endpoints.chunk_by(|left, right| left.0 == right.0) {
        let boundary = group[0].0;
        let throwing_sites = throwing.partition_point(|&site| site < boundary)
            - throwing.partition_point(|&site| site < previous);
        // A handler may cover its own initial astore. Only instructions that
        // can throw distinguish the ordered handler stacks of two intervals.
        if throwing_sites != 0 {
            let mut types = BTreeSet::new();
            let mut chain = SmallVec::<[&Handler; 4]>::new();
            for &index in &active {
                work += 1;
                if work > FRAME_WORK_LIMIT {
                    return Err(handler_capacity());
                }
                let handler: &Handler = &handlers[index];
                if !types.insert(handler.ty.as_ref()) {
                    continue;
                }
                edges += throwing_sites;
                if edges > HANDLER_EDGE_LIMIT {
                    return Err(handler_capacity());
                }
                chain.push(handler);
                if handler.ty.is_none() {
                    break;
                }
            }
            let last = &mut output[previous_stack.clone()];
            if last.len() == chain.len()
                && last
                    .iter()
                    .zip(&chain)
                    .all(|(last, current)| last.target == current.target && last.ty == current.ty)
            {
                for handler in last {
                    handler.end = boundary;
                }
            } else {
                let first = output.len();
                output.extend(chain.into_iter().map(|handler| Handler {
                    start: previous,
                    end: boundary,
                    target: handler.target,
                    ty: handler.ty.clone(),
                }));
                previous_stack = first..output.len();
            }
        }
        for &(_, begins, index) in group {
            if begins {
                active.insert(index);
            } else {
                active.remove(&index);
            }
        }
        previous = boundary;
    }
    Ok(output)
}

fn handler_capacity() -> SourceError {
    SourceError::Operation {
        kind: FailureKind::ResourceExhausted,
        reason: "JVM exception ranges exceed protected interval capacity".to_owned(),
    }
}

fn split(
    context: MethodContext,
    registers: Registers,
    output: Vec<InsnNode>,
    handlers: &[Handler],
    debug_info: Option<DebugInfo>,
) -> CFG {
    let mut cfg = Splitter::new("JVM method")
        .method(context)
        .instructions(output)
        .handlers(handlers.iter().map(Handler::raw).collect())
        .registers(registers.count())
        .ins(registers.input())
        .build();
    cfg.debug_info = debug_info;
    cfg
}

fn entry_frame(
    owner: &ClassNode,
    method: &MethodNode,
    writes: Option<&[bool; 256]>,
) -> Result<(Stack, Vec<(u16, ArgType)>, u16), OpError> {
    let mut frame = Stack::default();
    let mut physical_slot = 0u16;
    let mut bindings = Vec::new();
    let receiver = (!method.is_static()).then_some(owner.class_type());
    let receiver_count = usize::from(receiver.is_some());
    for (index, ty) in receiver.into_iter().chain(method.param_types()).enumerate() {
        let words = if ty.is_wide() { 2 } else { 1 };
        let start = usize::from(physical_slot);
        let parameter = writes
            .and_then(|writes| writes.get(start..start + usize::from(words)))
            .filter(|words| !words.contains(&true))
            .and_then(|_| index.checked_sub(receiver_count))
            .and_then(|index| u16::try_from(index).ok());
        frame.seed_local(physical_slot, ty.clone(), parameter)?;
        bindings.push((physical_slot, ty.clone()));
        physical_slot = physical_slot.checked_add(words).ok_or(OpError::Local)?;
    }
    Ok((frame, bindings, physical_slot))
}

enum Branch<'a> {
    Direct(i32),
    Switch(super::bytecode::Switch<'a>),
}

fn branch_targets(instruction: Instruction<'_>) -> Option<Branch<'_>> {
    match instruction.operands() {
        Operands::Branch(target) => Some(Branch::Direct(target)),
        Operands::Switch(switch) => Some(Branch::Switch(switch)),
        _ => None,
    }
}

fn is_terminal(instruction: Instruction<'_>) -> bool {
    matches!(
        instruction.wide_opcode().unwrap_or(instruction.opcode()),
        0xa9 | 0xac..=0xb1 | 0xbf
    )
}

fn has_fallthrough(instruction: Instruction<'_>) -> bool {
    !is_terminal(instruction) && !matches!(instruction.opcode(), 0xa7..=0xab | 0xc8..=0xc9)
}

fn throwable() -> ArgType {
    ArgType::object("java/lang/Throwable")
}

#[allow(clippy::too_many_arguments)]
fn frame_entries(
    class: &ClassFile<'_>,
    instructions: &[Instruction<'_>],
    blocks: &[BasicBlock],
    block_for: &[usize],
    handlers: &[Handler],
    catches: &[Option<ArgType>],
    index_at: &impl Fn(u16) -> Option<usize>,
    invalid_control: &[bool],
    registers: Registers,
    entry: Stack,
    method: u16,
    issues: &mut Vec<SourceIssue>,
) -> Vec<Option<Stack>> {
    let mut frames = vec![None; blocks.len()];
    let mut in_queue = vec![false; blocks.len()];
    let mut queue = VecDeque::new();
    let mut conflicted = vec![false; blocks.len()];
    let mut frame_values = entry.frame_values();
    frames[0] = Some(entry);
    queue.push_back(0);
    in_queue[0] = true;
    let mut work = 0usize;
    while let Some(index) = queue.pop_front() {
        in_queue[index] = false;
        let Some(mut frame) = frames[index].clone() else {
            continue;
        };
        let block = &blocks[index];
        let mut propagate = |destination: usize, incoming: &Stack| {
            let previous = frames[destination].as_ref().map_or(0, Stack::frame_values);
            let remaining = FRAME_VALUE_LIMIT.saturating_sub(frame_values - previous);
            let (mut changed, conflict) = if let Some(current) = &mut frames[destination] {
                match current.join(incoming) {
                    Ok(changed) => (changed, None),
                    Err(error) => {
                        current.invalidate();
                        (true, Some(error))
                    }
                }
            } else if incoming.frame_values() > remaining {
                let mut unknown = Stack::default();
                unknown.invalidate();
                frames[destination] = Some(unknown);
                (true, Some(OpError::Allocation))
            } else {
                frames[destination] = Some(incoming.clone());
                (true, None)
            };
            let current = frames[destination].as_mut().expect("propagated JVM stack");
            let excess = current.frame_values() > remaining;
            let conflict = conflict.or_else(|| excess.then_some(OpError::Allocation));
            if excess {
                current.invalidate();
                changed = true;
            }
            frame_values = frame_values - previous + current.frame_values();
            if let Some(error) = conflict {
                if !conflicted[destination] {
                    let pc = instructions[blocks[destination].first].offset();
                    let reason = if error.kind() == FailureKind::ResourceExhausted {
                        "JVM method exceeds the live frame capacity".to_owned()
                    } else {
                        error.to_string()
                    };
                    issues.push(issue(method, Some(pc), None, error.kind(), reason));
                    conflicted[destination] = true;
                }
            }
            if changed && !in_queue[destination] {
                in_queue[destination] = true;
                queue.push_back(destination);
            }
        };
        for position in block.first..block.end {
            let instruction = instructions[position];
            let offset = instruction.offset();
            work = work.saturating_add(1);
            if !handlers.is_empty() && instruction.may_throw() {
                work = work.saturating_add(handlers.len());
                for handler in handlers
                    .iter()
                    .filter(|handler| handler.start <= offset && offset < handler.end)
                {
                    let Some(target) = index_at(handler.target).map(|position| block_for[position])
                    else {
                        continue;
                    };
                    let Some(catch) = &catches[target] else {
                        continue;
                    };
                    let mut caught = frame.clone();
                    if caught.push_exception(catch.clone(), registers).is_ok() {
                        propagate(target, &caught);
                    }
                }
            }
            if work > FRAME_WORK_LIMIT {
                for (target, pending) in in_queue.iter().enumerate() {
                    if *pending {
                        if let Some(frame) = &mut frames[target] {
                            frame.invalidate();
                        }
                    }
                }
                let pc = instructions[block.first].offset();
                issues.push(issue(
                    method,
                    Some(pc),
                    None,
                    FailureKind::ResourceExhausted,
                    "JVM frame propagation exceeded the method work budget",
                ));
                return frames;
            }
            if invalid_control[position]
                || ops::apply(class, instruction, &mut frame, registers, None)
                    .is_err_and(|error| !error.retains_stack())
            {
                frame.invalidate();
            }
        }
        for &successor in &block.successors {
            propagate(successor, &frame);
        }
    }
    frames
}

/// The control action stays visible when its condition or value was lost.
fn emit_unavailable_control(
    instruction: Instruction<'_>,
    return_type: &ArgType,
    scratch: u32,
    reason: &str,
    output: &mut Vec<InsnNode>,
) -> bool {
    let mut unavailable = |ty: ArgType| {
        let mut node = InsnNode::unavailable(reason);
        node.set_result(RegisterArg::new(scratch, ty.clone()));
        output.push(node);
        InsnArg::reg(scratch, ty)
    };
    let control = match instruction.opcode() {
        0x99..=0xa6 | 0xc6..=0xc7 => {
            let Some(target) = instruction
                .relative_branch()
                .and_then(|delta| instruction.target(delta).ok())
            else {
                return false;
            };
            let condition = unavailable(ArgType::BOOLEAN);
            InsnNode::if_cmp(
                IfOp::Ne,
                condition,
                InsnArg::lit(0, ArgType::BOOLEAN),
                target.into(),
            )
        }
        0xa7 | 0xc8 => {
            let Some(target) = instruction
                .relative_branch()
                .and_then(|delta| instruction.target(delta).ok())
            else {
                return false;
            };
            InsnNode::goto(target.into())
        }
        0xaa | 0xab => {
            let Some(switch) = instruction.switch() else {
                return false;
            };
            let Some(default) = instruction.target(switch.default).ok() else {
                return false;
            };
            let cases = switch
                .cases()
                .map(|(key, delta)| {
                    instruction
                        .target(delta)
                        .ok()
                        .map(|target| (key, i32::from(target)))
                })
                .collect::<Option<Vec<_>>>();
            let Some(cases) = cases else { return false };
            let mut node = InsnNode::switch(unavailable(ArgType::INT), cases);
            node.payload.switch_default = Some(default.into());
            node
        }
        0xac..=0xb0 if return_type != &ArgType::VOID => {
            InsnNode::return_value(unavailable(return_type.clone()))
        }
        0xb1 => InsnNode::return_void(),
        0xbf => InsnNode::throw(unavailable(throwable())),
        _ => return false,
    };
    output.push(control);
    true
}

fn op_gap_reason(instruction: Instruction<'_>, error: &OpError) -> String {
    let offset = instruction.offset();
    let mnemonic = instruction
        .modified_mnemonic()
        .unwrap_or(instruction.mnemonic());
    match instruction.constant_index() {
        Some(index) => format!("JVM byte {offset}: {mnemonic} #{index}: {error}"),
        None => format!("JVM byte {offset}: {mnemonic}: {error}"),
    }
}

fn issue(
    method: u16,
    offset_byte: Option<u16>,
    end_offset_byte: Option<u32>,
    kind: FailureKind,
    reason: impl std::fmt::Display,
) -> SourceIssue {
    SourceIssue {
        method: Some(method),
        field: None,
        offset_byte,
        end_offset_byte,
        stage: "JVM decoding",
        kind,
        reason: reason.to_string(),
    }
}

fn op_error(error: OpError) -> SourceError {
    SourceError::Operation {
        kind: error.kind(),
        reason: error.to_string(),
    }
}
