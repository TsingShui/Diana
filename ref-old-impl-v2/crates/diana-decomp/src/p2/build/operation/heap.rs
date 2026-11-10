//! Checked operand translation for the heap operation family.

use super::*;

impl Builder<'_, '_> {
    pub(super) fn monitor(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let reg = self.register(instruction, 0)?;
        let reads = self.slots_for([RegisterSlot::new(reg, SlotWidth::One)])?;
        let effects = Effects::MONITOR.with(Effects::MAY_THROW);
        Ok(Translation {
            kind: OpKind::Monitor {
                enter: opcode == 0x1d,
            },
            reads,
            writes: self.empty_span(),
            effects,
        })
    }

    pub(super) fn cast(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let reg = self.register(instruction, 0)?;
        let type_id = match self.first_reference(instruction) {
            Some(OperandReference::Type(value)) => value,
            _ => return Err(internal("check-cast has no type reference")),
        };
        let reads = self.slots_for([RegisterSlot::new(reg, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::Cast { type_id },
            reads,
            writes: self.empty_span(),
            effects: Effects::MAY_THROW,
        })
    }

    pub(super) fn instance_of(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let (dest, source) = self.two_regs(instruction)?;
        let type_id = match self.first_reference(instruction) {
            Some(OperandReference::Type(value)) => value,
            _ => return Err(internal("instance-of has no type reference")),
        };
        let reads = self.slots_for([RegisterSlot::new(source, SlotWidth::One)])?;
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::InstanceOf { type_id },
            reads,
            writes,
            effects: Effects::default(),
        })
    }

    pub(super) fn array_length(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let (dest, source) = self.two_regs(instruction)?;
        let reads = self.slots_for([RegisterSlot::new(source, SlotWidth::One)])?;
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::ArrayLength,
            reads,
            writes,
            effects: Effects::MAY_THROW.with(Effects::HEAP_READ),
        })
    }

    pub(super) fn new_instance(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let dest = self.register(instruction, 0)?;
        let type_id = match self.first_reference(instruction) {
            Some(OperandReference::Type(value)) => value,
            _ => return Err(internal("new-instance has no type reference")),
        };
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::NewInstance { type_id },
            reads: self.empty_span(),
            writes,
            effects: Effects::MAY_THROW
                .with(Effects::CLASS_INIT)
                .with(Effects::ALLOCATION),
        })
    }

    pub(super) fn new_array(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let (dest, size) = self.two_regs(instruction)?;
        let type_id = match self.first_reference(instruction) {
            Some(OperandReference::Type(value)) => value,
            _ => return Err(internal("new-array has no type reference")),
        };
        let reads = self.slots_for([RegisterSlot::new(size, SlotWidth::One)])?;
        let writes = self.slots_for([RegisterSlot::new(dest, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::NewArray { type_id },
            reads,
            writes,
            effects: Effects::MAY_THROW.with(Effects::ALLOCATION),
        })
    }

    pub(super) fn filled_array(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let type_id = match self.first_reference(instruction) {
            Some(OperandReference::Type(value)) => value,
            _ => return Err(internal("filled-new-array has no type reference")),
        };
        let reads = self.all_register_slots(instruction, SlotWidth::One)?;
        Ok(Translation {
            kind: OpKind::FilledNewArray { type_id },
            reads,
            writes: self.empty_span(),
            effects: Effects::MAY_THROW
                .with(Effects::ALLOCATION)
                .with(Effects::HEAP_WRITE),
        })
    }

    pub(super) fn fill_array(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let array = self.register(instruction, 0)?;
        let payload = instruction
            .payload_target()
            .ok_or(internal("fill-array-data has no checked payload"))?;
        let reads = self.slots_for([RegisterSlot::new(array, SlotWidth::One)])?;
        Ok(Translation {
            kind: OpKind::FillArrayData { payload },
            reads,
            writes: self.empty_span(),
            effects: Effects::MAY_THROW.with(Effects::HEAP_WRITE),
        })
    }

    pub(super) fn array_access(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let (dest, array, index) = self.three_regs(instruction)?;
        let get = matches!(opcode, 0x44..=0x4a);
        let kind = self.array_kind(opcode);
        let width = kind.width();
        let reads = if get {
            self.slots_for([
                RegisterSlot::new(array, SlotWidth::One),
                RegisterSlot::new(index, SlotWidth::One),
            ])?
        } else {
            self.slots_for([
                RegisterSlot::new(dest, width),
                RegisterSlot::new(array, SlotWidth::One),
                RegisterSlot::new(index, SlotWidth::One),
            ])?
        };
        let writes = if get {
            self.slots_for([RegisterSlot::new(dest, width)])?
        } else {
            self.empty_span()
        };
        Ok(Translation {
            kind: OpKind::Array {
                access: if get {
                    ArrayAccess::Get
                } else {
                    ArrayAccess::Put
                },
                kind,
            },
            reads,
            writes,
            effects: Effects::MAY_THROW.with(if get {
                Effects::HEAP_READ
            } else {
                Effects::HEAP_WRITE
            }),
        })
    }

    pub(super) fn field_access(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let static_field = opcode >= 0x60;
        let (get, kind) = if static_field {
            (opcode < 0x67, self.field_kind((opcode - 0x60) % 7))
        } else {
            (opcode < 0x59, self.field_kind((opcode - 0x52) % 7))
        };
        let field = match self.first_reference(instruction) {
            Some(OperandReference::Field(value)) => value,
            _ => return Err(internal("field operation has no field reference")),
        };
        let dest = self.register(instruction, 0)?;
        let width = kind.width();
        let mut reads = Vec::new();
        if !static_field {
            push(
                &mut reads,
                RegisterSlot::new(self.register(instruction, 1)?, SlotWidth::One),
            )
            .map_err(request_error)?;
        }
        if !get {
            crate::substrate::owned::reserve(&mut reads, 1).map_err(request_error)?;
            reads.insert(0, RegisterSlot::new(dest, width));
        }
        let reads = self.slots_vec(reads)?;
        let writes = if get {
            self.slots_for([RegisterSlot::new(dest, width)])?
        } else {
            self.empty_span()
        };
        let mut effects = Effects::MAY_THROW.with(if get {
            Effects::HEAP_READ
        } else {
            Effects::HEAP_WRITE
        });
        if static_field {
            effects = effects.with(Effects::CLASS_INIT);
        }
        Ok(Translation {
            kind: OpKind::Field {
                access: if get {
                    FieldAccess::Get
                } else {
                    FieldAccess::Put
                },
                static_field,
                field,
                kind,
            },
            reads,
            writes,
            effects,
        })
    }
}
