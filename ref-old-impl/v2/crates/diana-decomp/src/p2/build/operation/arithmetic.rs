//! Checked operand translation for the arithmetic operation family.

use super::*;

impl Builder<'_, '_> {
    pub(super) fn compare(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let (dest, left, right) = self.three_regs(instruction)?;
        let op = match opcode {
            0x2d => CompareOp::CmplFloat,
            0x2e => CompareOp::CmpgFloat,
            0x2f => CompareOp::CmplDouble,
            0x30 => CompareOp::CmpgDouble,
            _ => CompareOp::CmpLong,
        };
        let left_width = match op {
            CompareOp::CmpLong | CompareOp::CmplDouble | CompareOp::CmpgDouble => SlotWidth::Two,
            _ => SlotWidth::One,
        };
        let reads = self.slots_for([
            RegisterSlot::new(left, left_width),
            RegisterSlot::new(right, left_width),
        ])?;
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::Compare { op },
            reads,
            writes,
            effects: Effects::default(),
        })
    }

    pub(super) fn unary(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let (dest, source) = self.two_regs(instruction)?;
        let op = self.unary_op(opcode)?;
        let reads = self.slots_for([RegisterSlot::new(source, op.source_width())])?;
        let writes = self.slots_for([RegisterSlot::new(dest, op.dest_width())])?;
        Ok(Translation {
            kind: OpKind::Unary { op },
            reads,
            writes,
            effects: Effects::default(),
        })
    }

    pub(super) fn binary_three(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let (dest, left, right) = self.three_regs(instruction)?;
        let op = binary_op((opcode - 0x90) % 32)?;
        let width = op.width();
        let reads = self.slots_for([
            RegisterSlot::new(left, op.left_width()),
            RegisterSlot::new(right, op.right_width()),
        ])?;
        let writes = self.slots_for([RegisterSlot::new(dest, width)])?;
        Ok(Translation {
            kind: OpKind::Binary {
                op,
                form: BinaryForm::ThreeAddress,
                literal: None,
            },
            reads,
            writes,
            effects: self.binary_effects(op),
        })
    }

    pub(super) fn binary_two(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let (dest, right) = self.two_regs(instruction)?;
        let op = binary_op((opcode - 0xb0) % 32)?;
        let width = op.width();
        let reads = self.slots_for([
            RegisterSlot::new(dest, op.left_width()),
            RegisterSlot::new(right, op.right_width()),
        ])?;
        let writes = self.slots_for([RegisterSlot::new(dest, width)])?;
        Ok(Translation {
            kind: OpKind::Binary {
                op,
                form: BinaryForm::TwoAddress,
                literal: None,
            },
            reads,
            writes,
            effects: self.binary_effects(op),
        })
    }

    pub(super) fn literal16(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let (dest, source) = self.two_regs(instruction)?;
        let op = lit16_op((opcode - 0xd0) % 8)?;
        let literal = self.signed_literal(instruction)?;
        let reads = self.slots_for([RegisterSlot::new(source, SlotWidth::One)])?;
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::Binary {
                op,
                form: BinaryForm::Literal,
                literal: Some(literal),
            },
            reads,
            writes,
            effects: self.binary_effects(op),
        })
    }

    pub(super) fn literal8(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let (dest, source) = self.two_regs(instruction)?;
        let op = lit8_op((opcode - 0xd8) % 11)?;
        let literal = self.signed_literal(instruction)?;
        let reads = self.slots_for([RegisterSlot::new(source, SlotWidth::One)])?;
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::Binary {
                op,
                form: BinaryForm::Literal,
                literal: Some(literal),
            },
            reads,
            writes,
            effects: self.binary_effects(op),
        })
    }
}
