//! Translates checked P1 operands once, then constructs normal/exception
//! completion from those ordered slots and effects. No decoding or independent
//! replay occurs here; only the ordinary Builder can publish the operation.

mod arithmetic;
mod heap;
mod invoke;
mod values;

use super::*;

struct Translation {
    kind: OpKind,
    reads: SlotSpan,
    writes: SlotSpan,
    effects: Effects,
}

impl Builder<'_, '_> {
    pub(super) fn operation(
        &mut self,
        atom: &Atom,
        instruction: &DecodedInstruction,
        block_index: usize,
    ) -> Result<Operation, FlowError> {
        let block = self.blocks[block_index].id;
        let id =
            OpId(u32::try_from(self.operations.len()).map_err(|_| FlowError::PlatformCapacity)?);
        let translation = self.translate_operation(instruction)?;
        self.complete_operation(atom, instruction, block, id, translation)
    }

    fn translate_operation(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        match opcode {
            0x00 => Ok(Translation {
                kind: OpKind::Nop,
                reads: self.empty_span(),
                writes: self.empty_span(),
                effects: Effects::default(),
            }),
            0x01..=0x09 => self.move_value(instruction),
            0x0a..=0x0c => self.unresolved_result(instruction),
            0x0d => self.move_exception(instruction),
            0x12..=0x19 => self.literal_constant(instruction),
            0x1a | 0x1b => self.string_constant(instruction),
            0x1c => self.class_constant(instruction),
            0x1d | 0x1e => self.monitor(instruction),
            0x1f => self.cast(instruction),
            0x20 => self.instance_of(instruction),
            0x21 => self.array_length(instruction),
            0x22 => self.new_instance(instruction),
            0x23 => self.new_array(instruction),
            0x24 | 0x25 => self.filled_array(instruction),
            0x26 => self.fill_array(instruction),
            0x2d..=0x31 => self.compare(instruction),
            0x44..=0x51 => self.array_access(instruction),
            0x52..=0x6d => self.field_access(instruction),
            0x6e..=0x72 | 0x74..=0x78 | 0xfa..=0xfd => self.invoke(instruction),
            0x7b..=0x8f => self.unary(instruction),
            0x90..=0xaf => self.binary_three(instruction),
            0xb0..=0xcf => self.binary_two(instruction),
            0xd0..=0xd7 => self.literal16(instruction),
            0xd8..=0xe2 => self.literal8(instruction),
            0xfe => self.handle_constant(instruction),
            0xff => self.method_type_constant(instruction),
            _ => Err(FlowError::Unsupported {
                pc: instruction.pc(),
                opcode,
                reason: "opcode has no P2 operation translation",
            }),
        }
    }

    fn complete_operation(
        &mut self,
        atom: &Atom,
        instruction: &DecodedInstruction,
        block: BlockId,
        id: OpId,
        translation: Translation,
    ) -> Result<Operation, FlowError> {
        let pc = instruction.pc();
        let opcode = instruction.spec().opcode;
        let Translation {
            kind,
            reads,
            mut writes,
            mut effects,
        } = translation;
        if instruction.spec().may_throw {
            effects = effects.with(Effects::MAY_THROW);
        }
        let result_pc = atom.result.map(|index| self.instructions[index].pc());
        if let Some(result_index) = atom.result {
            let result_instruction = self.instructions[result_index];
            let dest = self.register(result_instruction, 0)?;
            let width = match result_instruction.opcode() {
                0x0b => SlotWidth::Two,
                _ => SlotWidth::One,
            };
            writes = self.slots_for([RegisterSlot::new(dest, width)])?;
        }
        let commit = if atom.result.is_some() {
            CommitRule::AfterResult
        } else if writes.is_empty() {
            CommitRule::None
        } else {
            CommitRule::AtOperation
        };
        let commit_pc = match commit {
            CommitRule::None => None,
            CommitRule::AtOperation => Some(pc),
            CommitRule::AfterResult => result_pc,
        };
        let heap = if effects.contains(Effects::HEAP_WRITE)
            || effects.contains(Effects::ALLOCATION)
            || effects.contains(Effects::CLASS_INIT)
            || effects.contains(Effects::MONITOR)
        {
            HeapTransfer::EventOutcome
        } else {
            HeapTransfer::Preserve
        };
        let lock = match kind {
            OpKind::Monitor { enter: true } => LockTransfer::Acquire,
            OpKind::Monitor { enter: false } => LockTransfer::Release,
            _ => LockTransfer::Preserve,
        };
        let exceptional = if effects.may_throw() {
            Some(ExceptionCompletion {
                throw_pc: pc,
                selection_pc: if opcode == 0x1e {
                    pc.checked_advance(instruction.width())
                        .ok_or(FlowError::PlatformCapacity)?
                } else {
                    pc
                },
                frame: ExceptionFrame::PreserveBeforeOperation,
                heap,
                lock: if opcode == 0x1e {
                    LockTransfer::ReleaseOutcome
                } else {
                    LockTransfer::Preserve
                },
            })
        } else {
            None
        };
        Ok(Operation {
            id,
            block,
            pc,
            result_pc,
            kind,
            reads,
            writes,
            effects,
            normal: NormalCompletion {
                reads: ReadOrder::CaptureAllBeforeCommit,
                commit,
                commit_pc,
                heap,
                lock,
            },
            exceptional,
        })
    }
}
