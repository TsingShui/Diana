//! Checked operand translation for the invoke operation family.

use super::*;

impl Builder<'_, '_> {
    pub(super) fn invoke(
        &mut self,
        instruction: &DecodedInstruction,
    ) -> Result<Translation, FlowError> {
        let opcode = instruction.spec().opcode;
        let pc = instruction.pc();
        let target = match (opcode, self.first_reference(instruction)) {
            (0xfc | 0xfd, Some(OperandReference::CallSite(value))) => {
                OperandReference::CallSite(value)
            }
            (_, Some(OperandReference::Method(value))) => OperandReference::Method(value),
            _ => return Err(internal("invoke has no checked target reference")),
        };
        let kind = self.invoke_kind(opcode);
        let prototype = if kind == InvokeKind::Polymorphic {
            match instruction
                .references()
                .find(|reference| reference.slot == crate::raw::ReferenceSlot::Second)
                .map(|reference| reference.target)
            {
                Some(OperandReference::Proto(id)) => Some(id),
                _ => return Err(internal("polymorphic call has no checked prototype")),
            }
        } else {
            None
        };
        let layout = self
            .decoded
            .invoke_layout_at(pc)
            .ok_or(internal("invoke has no P1 layout state"))?;
        let (kind, reads) = match layout {
            Ok(layout) => {
                let reads = self.invoke_reads(instruction, layout)?;
                (
                    OpKind::Invoke {
                        kind,
                        target,
                        prototype,
                        result: layout.result(),
                    },
                    reads,
                )
            }
            Err(_) => (
                OpKind::UnresolvedInvoke {
                    kind,
                    target,
                    prototype,
                    result: self
                        .decoded
                        .invoke_result_at(pc)
                        .and_then(Result::ok)
                        .map(InvokeResult::from)
                        .unwrap_or(InvokeResult::Unavailable),
                },
                self.empty_span(),
            ),
        };
        Ok(Translation {
            kind,
            reads,
            writes: self.empty_span(),
            effects: Effects::CALL
                .with(Effects::MAY_THROW)
                .with(Effects::HEAP_READ)
                .with(Effects::HEAP_WRITE)
                .with(Effects::CLASS_INIT),
        })
    }
}
