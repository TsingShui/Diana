//! Checked operand translation for the values operation family.

use super::*;

impl Builder<'_, '_> {
    pub(super) fn move_value(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let kind = match opcode {
            0x01..=0x03 => MoveKind::Word,
            0x04..=0x06 => MoveKind::Wide,
            _ => MoveKind::Object,
        };
        let (dest, source) = self.two_regs(instruction)?;
        let width = match kind {
            MoveKind::Wide => SlotWidth::Two,
            _ => SlotWidth::One,
        };
        let reads = self.slots_for([RegisterSlot::new(source, width)])?;
        let writes = self.slots_for([RegisterSlot::new(dest, width)])?;
        Ok(Translation {
            kind: OpKind::Move { kind },
            reads,
            writes,
            effects: Effects::default(),
        })
    }

    pub(super) fn unresolved_result(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let pc = instruction.pc();
        if self.decoded.result_issue_at(pc).is_none() {
            return Err(internal("unmerged move-result lost its P1 issue"));
        }
        let width = if opcode == 0x0b {
            SlotWidth::Two
        } else {
            SlotWidth::One
        };
        let dest = self.register(instruction, 0)?;
        let writes = self.slots_for([RegisterSlot::new(dest, width)])?;
        Ok(Translation {
            kind: OpKind::UnresolvedResult { width },
            reads: self.empty_span(),
            writes,
            effects: Effects::default(),
        })
    }

    pub(super) fn move_exception(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let dest = self.register(instruction, 0)?;
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::MoveException,
            reads: self.empty_span(),
            writes,
            effects: Effects::default(),
        })
    }

    pub(super) fn literal_constant(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let dest = self.register(instruction, 0)?;
        let width = if opcode >= 0x16 {
            SlotWidth::Two
        } else {
            SlotWidth::One
        };
        let bits = self.literal_bits(instruction)?;
        let writes = self.slots_for([RegisterSlot::new(dest, width)])?;
        Ok(Translation {
            kind: OpKind::Const { bits, width },
            reads: self.empty_span(),
            writes,
            effects: Effects::default(),
        })
    }

    pub(super) fn string_constant(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let dest = self.register(instruction, 0)?;
        let value = match self.first_reference(instruction) {
            Some(OperandReference::String(value)) => value,
            _ => return Err(internal("const-string has no string reference")),
        };
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::ConstString { value },
            reads: self.empty_span(),
            writes,
            effects: Effects::MAY_THROW.with(Effects::CLASS_INIT),
        })
    }

    pub(super) fn class_constant(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let dest = self.register(instruction, 0)?;
        let type_id = match self.first_reference(instruction) {
            Some(OperandReference::Type(value)) => value,
            _ => return Err(internal("const-class has no type reference")),
        };
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::ConstClass { type_id },
            reads: self.empty_span(),
            writes,
            effects: Effects::MAY_THROW,
        })
    }

    pub(super) fn handle_constant(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let dest = self.register(instruction, 0)?;
        let handle = match self.first_reference(instruction) {
            Some(OperandReference::MethodHandle(value)) => value,
            _ => {
                return Err(internal(
                    "const-method-handle has no method-handle reference",
                ));
            }
        };
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::ConstMethodHandle { handle },
            reads: self.empty_span(),
            writes,
            effects: Effects::MAY_THROW,
        })
    }

    pub(super) fn method_type_constant(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let dest = self.register(instruction, 0)?;
        let proto = match self.first_reference(instruction) {
            Some(OperandReference::Proto(value)) => value,
            _ => return Err(internal("const-method-type has no proto reference")),
        };
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::ConstMethodType { proto },
            reads: self.empty_span(),
            writes,
            effects: Effects::MAY_THROW,
        })
    }
}
