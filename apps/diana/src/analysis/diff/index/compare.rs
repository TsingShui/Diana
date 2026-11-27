//! Compare original control targets in the coordinate system of an aligned
//! method pair. The method-end sentinel is a valid exception-range boundary.
use super::body::{read_uint, records};
use super::{Body, Format};
use std::collections::HashMap;
use std::fmt::Write;

struct Alignment<'a> {
    old: &'a Body,
    new: &'a Body,
    targets: &'a [Option<usize>],
}

impl Alignment<'_> {
    fn target(&self, a: u32, b: u32) -> bool {
        if a as usize == self.old.code.len() {
            return b as usize == self.new.code.len();
        }
        self.targets.get(a as usize).copied().flatten() == Some(b as usize)
    }

    fn encoded_target(&self, a: &[u8], b: &[u8]) -> bool {
        let (Ok(a), Ok(b)) = (a.try_into(), b.try_into()) else {
            return false;
        };
        self.target(u32::from_le_bytes(a), u32::from_le_bytes(b))
    }
}

impl Body {
    pub fn operands_match(
        &self,
        i: usize,
        other: &Body,
        j: usize,
        symbols: &HashMap<String, String>,
        targets: &[Option<usize>],
    ) -> bool {
        if self.format == Format::Jvm
            && !(self.code[i].known_operands && other.code[j].known_operands)
        {
            return true;
        }
        let alignment = Alignment {
            old: self,
            new: other,
            targets,
        };
        let old = self.operand(i);
        let new = other.operand(j);
        let old_values = self.operand_without_registers(i);
        let new_values = other.operand_without_registers(j);
        if old[..old.len() - old_values.len()] != new[..new.len() - new_values.len()] {
            return false;
        }
        let mut left = records(old_values);
        let mut right = records(new_values);
        loop {
            let (a, b) = match (left.next(), right.next()) {
                (None, None) => return true,
                (Some(a), Some(b)) if a.0 == b.0 => (a, b),
                _ => return false,
            };
            let equal = match a.0 {
                b'B' | b'D' => alignment.encoded_target(a.1, b.1),
                b'K' if self.format.switch(self.code[i].opcode) => {
                    a.1.len() == 12
                        && b.1.len() == 12
                        && a.1[..8] == b.1[..8]
                        && alignment.encoded_target(&a.1[8..], &b.1[8..])
                }
                b'm' | b't' | b'f' => {
                    let old = self.symbol(a.1);
                    let new = other.symbol(b.1);
                    match (old, new) {
                        (Some(old), Some(new)) => {
                            symbols.get(old).map_or(old, String::as_str) == new
                        }
                        _ => false,
                    }
                }
                _ => a.1 == b.1,
            };
            if !equal {
                return false;
            }
        }
    }

    pub fn describe_protection(&self) -> String {
        let mut result = String::new();
        let mut ranges = self.protection.as_slice();
        let pc = |index: u32| {
            self.code
                .get(index as usize)
                .map_or(self.code_units, |op| op.pc)
        };
        while !ranges.is_empty() {
            let Some((start, end, count)) = region(&mut ranges) else {
                break;
            };
            if !result.is_empty() {
                result.push_str(", ");
            }
            let _ = write!(result, "try [0x{:x},0x{:x})", pc(start), pc(end));
            for _ in 0..count {
                let Some((target, tag, ty)) = clause(&mut ranges) else {
                    return result;
                };
                let name = if tag == b'*' {
                    "catch-all"
                } else {
                    std::str::from_utf8(ty).unwrap_or("<unknown>")
                };
                let _ = write!(result, " {name} -> @0x{:x}", pc(target));
            }
        }
        result
    }
}

pub(super) fn region(input: &mut &[u8]) -> Option<(u32, u32, u32)> {
    Some((read_uint(input)?, read_uint(input)?, read_uint(input)?))
}
pub(super) fn clause<'a>(input: &mut &'a [u8]) -> Option<(u32, u8, &'a [u8])> {
    let target = read_uint(input)?;
    let (&tag, tail) = input.split_first()?;
    *input = tail;
    let len = read_uint(input)? as usize;
    let (ty, tail) = input.split_at_checked(len)?;
    *input = tail;
    Some((target, tag, ty))
}
