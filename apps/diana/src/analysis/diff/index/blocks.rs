//! Partitions of the checked original instruction stream used for block
//! comparison. Payload partitions are data, never normal successors.
use super::body::records;
use super::compare::{clause, region};
use super::{Body, Format, Op};
use diana_decomp::input::formats::{FlowKind, OpcodeSpec};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Flow(u8);

impl Flow {
    const FALLTHROUGH: u8 = 1;
    const TERMINATES: u8 = 2;
    const THROWS: u8 = 4;
    const DATA: u8 = 8;

    pub(super) const PAYLOAD: Self = Self(Self::DATA);

    pub(super) fn instruction(spec: OpcodeSpec) -> Self {
        use FlowKind::*;
        let flow = match spec.flow {
            Next | FillArray => Self::FALLTHROUGH,
            Conditional | PackedSwitch | SparseSwitch => Self::FALLTHROUGH | Self::TERMINATES,
            Return | Throw | Goto => Self::TERMINATES,
        };
        Self(flow | if spec.may_throw { Self::THROWS } else { 0 })
    }

    pub(super) fn jvm(op: diana_decomp::input::jvm::bytecode::Instruction<'_>) -> Self {
        let flow = match op.wide_opcode().unwrap_or_else(|| op.opcode()) {
            0x99..=0xa6 | 0xc6..=0xc7 => Self::FALLTHROUGH | Self::TERMINATES,
            0xa7..=0xb1 | 0xbf | 0xc8..=0xc9 => Self::TERMINATES,
            _ => Self::FALLTHROUGH,
        };
        Self(flow | if op.may_throw() { Self::THROWS } else { 0 })
    }

    pub(super) fn may_throw(self) -> bool {
        self.has(Self::THROWS)
    }

    fn has(self, mask: u8) -> bool {
        self.0 & mask != 0
    }
}

impl Body {
    fn is_payload(&self, i: usize) -> bool {
        self.code.get(i).is_some_and(|op| op.flow.has(Flow::DATA))
    }

    /// True only for an in-method normal successor at the next instruction.
    pub fn falls_through(&self, i: usize) -> bool {
        self.complete_control
            && self
                .code
                .get(i)
                .is_some_and(|op| op.flow.has(Flow::FALLTHROUGH))
            && i + 1 < self.code.len()
            && !self.is_payload(i + 1)
    }

    /// Includes 0 and the exclusive end. Every payload is a singleton partition.
    /// An unfinished original control directory has no certified partition.
    pub fn block_boundaries(&self) -> Option<Vec<usize>> {
        if !self.complete_control {
            return None;
        }
        let mut leader = vec![false; self.code.len() + 1];
        leader[0] = true;
        leader[self.code.len()] = true;
        for (i, op) in self.code.iter().enumerate() {
            if op.flow.has(Flow::DATA) {
                leader[i] = true;
                leader[i + 1] = true;
                continue;
            }
            if op.flow.has(Flow::TERMINATES) {
                leader[i + 1] = true;
            }
            if !op.flow.has(Flow::TERMINATES) && !(self.format == Format::Dex && op.opcode == 0x26)
            {
                continue;
            }
            for (kind, bytes) in records(self.operand_without_registers(i)) {
                let target = match kind {
                    b'B' | b'D' => bytes,
                    b'K' if self.format.switch(op.opcode) => bytes.get(8..)?,
                    _ => continue,
                };
                let target = u32::from_le_bytes(target.try_into().ok()?) as usize;
                *leader.get_mut(target)? = true;
            }
        }
        let mut protection = self.protection.as_slice();
        while !protection.is_empty() {
            let (start, end, clauses) = region(&mut protection)?;
            let start = start as usize;
            let end = end as usize;
            *leader.get_mut(start)? = true;
            *leader.get_mut(end)? = true;
            // Checked protected ranges are disjoint, so this walk visits each
            // protected instruction at most once across the entire method.
            let protected = self.code.get(start..end)?;
            for (offset, Op { flow, .. }) in protected.iter().enumerate() {
                if flow.has(Flow::THROWS) {
                    leader[start + offset + 1] = true;
                }
            }
            for _ in 0..clauses {
                let (handler, _, _) = clause(&mut protection)?;
                *leader.get_mut(handler as usize)? = true;
            }
        }
        Some(
            leader
                .iter()
                .enumerate()
                .filter_map(|(i, &is_leader)| is_leader.then_some(i))
                .collect(),
        )
    }
}
