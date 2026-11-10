//! Target-index pre-filter over raw instruction bytes.
//!
//! Every reference instruction encodes its target index as a little-endian
//! operand at `pc + 2`, and instructions start on two-byte boundaries, so the
//! operand bytes always sit at an even offset inside the instruction stream. A
//! body that contains no such value cannot reference any target, and skipping
//! its decode costs one pass over its bytes instead of a full disassembly.
//!
//! The filter is conservative: a body that passes is decoded and matched
//! exactly, so a false positive only costs work, never a wrong row.

use diana_decomp::input::ProgramIndex;
use diana_decomp::{FieldId, MethodId, StringId, TypeId};

use super::{QueryError, Target, filter};

/// One bit per 16-bit operand value; `const-string/jumbo` is the only reference
/// instruction outside that range, and a target beyond it disables the filter.
const BITMAP_INDICES: usize = 1 << 16;

/// The resolved target indices, as a membership test over operand bytes.
/// `None` disables filtering (no resolved target, or an operand wider than 16 bits).
pub(super) struct Prefilter {
    bits: Option<Vec<u64>>,
}

impl Prefilter {
    pub(super) fn new(
        index: &ProgramIndex<'_>,
        targets: &[Target],
        typed_uses: bool,
    ) -> Result<Self, QueryError> {
        // Typed-use expansion matches a type through every member signature that
        // mentions it, so its operand set is far wider than one id space. Leave
        // that mode unfiltered.
        if typed_uses {
            return Ok(Self { bits: None });
        }
        let mut ids = Vec::new();
        for target in targets {
            match target {
                Target::String(predicate) => {
                    for id in 0..index.string_ids_len() {
                        let value = index.string(StringId(id))?;
                        if predicate.evaluate(value)?.is_some() {
                            ids.push(id);
                        }
                    }
                }
                Target::Type(predicate) => {
                    for id in 0..index.type_ids_len() {
                        let (descriptor, _) = index.type_descriptor(TypeId(id))?;
                        if predicate.evaluate(descriptor)?.is_some() {
                            ids.push(id);
                        }
                    }
                }
                Target::Method {
                    owner,
                    name,
                    prototype,
                    ..
                } => {
                    for id in 0..index.method_ids_len() {
                        let reference = index.method_reference(MethodId(id))?;
                        if filter::select(owner.as_ref(), reference.owner)?.is_none() {
                            continue;
                        }
                        if name.evaluate(reference.declaration.name)?.is_none() {
                            continue;
                        }
                        if let Some(signature) = prototype
                            && !reference.declaration.prototype.matches(signature)?
                        {
                            continue;
                        }
                        ids.push(id);
                    }
                }
                Target::Field {
                    owner,
                    name,
                    field_type,
                } => {
                    for id in 0..index.field_ids_len() {
                        let reference = index.field_reference(FieldId(id))?;
                        if filter::select(owner.as_ref(), reference.owner)?.is_none() {
                            continue;
                        }
                        if name.evaluate(reference.declaration.name)?.is_none() {
                            continue;
                        }
                        if let Some(field_type) = field_type
                            && !reference.declaration.field_type.matches(field_type)?
                        {
                            continue;
                        }
                        ids.push(id);
                    }
                }
                Target::Resource(_) | Target::Hierarchy { .. } => {}
            }
        }
        if ids.is_empty() {
            return Ok(Self { bits: None });
        }
        let mut bits = Vec::new();
        bits.try_reserve_exact(BITMAP_INDICES / 64)
            .map_err(|_| QueryError::from(allocation_error()))?;
        bits.resize(BITMAP_INDICES / 64, 0u64);
        for id in ids {
            let Ok(value) = u16::try_from(id) else {
                // A 32-bit operand cannot be answered by the 16-bit table.
                return Ok(Self { bits: None });
            };
            bits[usize::from(value) >> 6] |= 1u64 << (value & 63);
        }
        Ok(Self { bits: Some(bits) })
    }

    /// Whether `code` can contain any target operand; `true` when filtering is off.
    pub(super) fn might_reference(&self, code: &[u8]) -> bool {
        let Some(bits) = &self.bits else {
            return true;
        };
        // Operands start at `pc + 2` on two-byte boundaries, so only even offsets
        // inside the instruction stream can carry one.
        let mut index = 0usize;
        while index + 1 < code.len() {
            let value = u16::from_le_bytes([code[index], code[index + 1]]) as usize;
            if bits[value >> 6] >> (value & 63) & 1 != 0 {
                return true;
            }
            index += 2;
        }
        false
    }
}

fn allocation_error() -> diana_decomp::input::InputError {
    diana_decomp::input::InputError::Allocation {
        offset_byte: diana_decomp::ByteOffset(0),
    }
}
