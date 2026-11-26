//! Canonical DEX operands. PCs stay alongside the comparable stream so a diff
//! can navigate either original even when instruction encoding widths changed.
use std::collections::HashMap;

use diana_decomp::input::RawMethodView;
use diana_decomp::input::formats::{LayoutKind, RegisterShape, RegisterWidth, opcode_mnemonic};
use diana_decomp::p1::{DecodedInstruction, DecodedItem, DecodedMethod, DecodedOutcome};
use diana_decomp::raw::RawLiteral;
use diana_decomp::{CodeUnitOffset, TypeId};

use super::{Body, Method, Op, reference};

// Bound expanded operands as well as the source code: a tiny const-string site
// can reference a multi-megabyte string many times. A stopped method retains
// its completed prefix and a local diagnostic.
pub(super) const MAX_BYTES: usize = 64 * 1024 * 1024;
const CAPACITY: &str = "expanded method operands exceed the index capacity";
const PACKED: u8 = 0x3e;
const SPARSE: u8 = 0x3f;
const ARRAY: u8 = 0x40;

// Each operation starts with a counted list of canonical register words.
// The following typed records are length-prefixed, including arbitrary strings
// and payloads; DEX text can therefore never introduce another operand.
fn uint(out: &mut Vec<u8>, mut n: u32) {
    while n >= 128 {
        out.push(n as u8 | 128);
        n >>= 7;
    }
    out.push(n as u8);
}
fn value(out: &mut Vec<u8>, tag: u8, bytes: &[u8]) -> Result<(), String> {
    if out.len().saturating_add(bytes.len()).saturating_add(6) > MAX_BYTES {
        return Err(CAPACITY.into());
    }
    out.push(tag);
    uint(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
    Ok(())
}

pub(super) fn read_uint(input: &mut &[u8]) -> Option<u32> {
    let mut n = 0;
    for shift in (0..=28).step_by(7) {
        let (&head, tail) = input.split_first()?;
        *input = tail;
        n |= u32::from(head & 127) << shift;
        if head < 128 {
            return Some(n);
        }
    }
    None
}

pub(super) fn records(mut input: &[u8]) -> impl Iterator<Item = (u8, &[u8])> {
    std::iter::from_fn(move || {
        let (&tag, mut rest) = input.split_first()?;
        let len = read_uint(&mut rest)? as usize;
        let (bytes, tail) = rest.split_at_checked(len)?;
        input = tail;
        Some((tag, bytes))
    })
}

impl Body {
    /// Canonical register words in operand order, including the high halves
    /// of wide operands. Odd identities name incoming parameter words by ordinal.
    pub fn register_words(&self, i: usize) -> impl Iterator<Item = u32> + '_ {
        let mut input = self.operand(i);
        let count = read_uint(&mut input).unwrap_or(0);
        (0..count).map_while(move |_| read_uint(&mut input))
    }

    pub fn registers(&self, i: usize) -> &[u16] {
        let from = i
            .checked_sub(1)
            .map_or(0, |prev| self.code[prev].register_end as usize);
        &self.raw_registers[from..self.code[i].register_end as usize]
    }

    pub fn operand_without_registers(&self, i: usize) -> &[u8] {
        let mut input = self.operand(i);
        for _ in 0..read_uint(&mut input).unwrap_or(0) {
            read_uint(&mut input);
        }
        input
    }

    pub fn callee(&self, i: usize) -> Option<&str> {
        records(self.operand_without_registers(i))
            .find(|(tag, _)| *tag == b'm')
            .and_then(|(_, bytes)| self.symbol(bytes))
    }

    pub fn describe_op(&self, i: usize) -> String {
        use std::fmt::Write;
        let opcode = self.code[i].opcode;
        let mnemonic = match opcode {
            PACKED => "packed-switch-payload",
            SPARSE => "sparse-switch-payload",
            ARRAY => "array-data-payload",
            _ => opcode_mnemonic(opcode).unwrap_or("unknown"),
        };
        let mut out = mnemonic.to_owned();
        for reg in self.registers(i) {
            let _ = write!(out, " v{reg}");
        }
        for (tag, data) in records(self.operand_without_registers(i)) {
            match tag {
                b'm' | b's' | b't' | b'f' | b'p' | b'h' | b'c' => {
                    out.push(' ');
                    out.push_str(self.symbol(data).unwrap_or("<unavailable>"));
                }
                b'L' => {
                    let mut bits = [0u8; 8];
                    if data.len() <= bits.len() {
                        bits[..data.len()].copy_from_slice(data);
                        let _ = write!(out, " #0x{:x}", u64::from_le_bytes(bits));
                    }
                }
                b'B' | b'D' => {
                    if let Ok(word) = data.try_into() {
                        let target = u32::from_le_bytes(word) as usize;
                        if let Some(op) = self.code.get(target) {
                            let _ = write!(out, " @0x{:x}", op.pc);
                        }
                    }
                }
                b'K' if matches!(opcode, 0x2b | 0x2c) && data.len() == 12 => {
                    let key = i64::from_le_bytes(data[..8].try_into().expect("eight-byte key"));
                    let target =
                        u32::from_le_bytes(data[8..].try_into().expect("four-byte target"));
                    if let Some(op) = self.code.get(target as usize) {
                        let _ = write!(out, " case {key} -> @0x{:x}", op.pc);
                    }
                }
                b'W' => {
                    if let Ok(width) = data.try_into() {
                        let _ = write!(out, " element-width={}", u32::from_le_bytes(width));
                    }
                }
                b'A' | b'K' => {
                    let _ = write!(out, " {} bytes [", data.len());
                    for b in data {
                        let _ = write!(out, "{b:02x}");
                    }
                    out.push(']');
                }
                _ => {
                    out.push(' ');
                    out.push_str(std::str::from_utf8(data).unwrap_or("<unavailable>"));
                }
            }
        }
        out
    }
}

fn normalized_opcode(opcode: u8) -> u8 {
    match opcode {
        0x01..=0x03 => 0x01,
        0x04..=0x06 => 0x04,
        0x07..=0x09 => 0x07,
        0x12..=0x15 => 0x14,
        0x16..=0x19 => 0x18,
        0x1b => 0x1a,
        0x25 => 0x24,
        0x29..=0x2a => 0x28,
        0x74..=0x78 => opcode - 6,
        0xfb | 0xfd => opcode - 1,
        _ => opcode,
    }
}

struct Words {
    parameter_base: u32,
    frame: u32,
    low: [u16; 32],
    high: HashMap<u32, u32>,
    next: u32,
}
impl Words {
    fn canonical(&mut self, reg: u32) -> Result<u32, String> {
        if reg >= self.frame {
            return Err(format!("register v{reg} outside the method frame"));
        }
        if reg >= self.parameter_base {
            return Ok((reg - self.parameter_base) * 2 + 1);
        }
        if let Some(slot) = self.low.get_mut(reg as usize) {
            if *slot == u16::MAX {
                // At most 65535 words in a legal frame, with ordinal <65535.
                *slot = self.next as u16;
                self.next += 1;
            }
            return Ok(u32::from(*slot) * 2);
        }
        let ordinal = self.high.entry(reg).or_insert_with(|| {
            let ordinal = self.next;
            self.next += 1;
            ordinal
        });
        Ok(*ordinal * 2)
    }

    fn write(
        &mut self,
        insn: &DecodedInstruction,
        body: &mut Body,
        shape: &mut u8,
    ) -> Result<(), String> {
        let operands = insn.operands().registers();
        let widths = match insn.spec().registers {
            RegisterShape::Fixed(w) => w,
            _ => [None; 3],
        };
        let wide = |i: usize| matches!(widths.get(i), Some(Some(RegisterWidth::Two)));
        let count = operands.len() + (0..operands.len()).filter(|&i| wide(i)).count();
        uint(&mut body.operands, count as u32);
        // Fixed-operand arity and the alias pattern among its first three
        // words fit directly in a byte. Larger invoke shapes are only a
        // candidate hint; exact arguments remain in the operand stream.
        *shape = (count as u8).rotate_left(3);
        let mut previous = [None; 3];
        for (i, reg) in operands.iter().enumerate() {
            let raw = u16::try_from(reg.0).map_err(|_| "register exceeds DEX frame")?;
            body.raw_registers.push(raw);
            // Relative alias relationships of fixed-width operands are useful for
            // candidate matching and do not depend on allocation or parameter IDs.
            if i < previous.len() {
                if let Some(first) = previous[..i].iter().position(|r| *r == Some(raw)) {
                    *shape ^= 1 << (i + first - 1);
                }
                previous[i] = Some(raw);
            }
            uint(&mut body.operands, self.canonical(reg.0)?);
            if wide(i) {
                uint(&mut body.operands, self.canonical(reg.0 + 1)?);
            }
        }
        Ok(())
    }
}

struct Evidence<'i, 'a> {
    items: &'i [DecodedItem],
    raw: &'i RawMethodView<'a>,
    decoded: Option<&'i DecodedMethod<'a>>,
    symbols: &'i super::Symbols,
}

impl Evidence<'_, '_> {
    fn target(&self, pc: CodeUnitOffset) -> Result<u32, String> {
        if pc == CodeUnitOffset(self.raw.code_units().0) {
            return Ok(self.items.len() as u32);
        }
        self.items
            .binary_search_by_key(&pc, DecodedItem::pc)
            .map(|n| n as u32)
            .map_err(|_| format!("no instruction boundary at {pc:?}"))
    }

    fn write_insn(
        &self,
        insn: &DecodedInstruction,
        body: &mut Body,
        cache: &mut reference::Cache,
        issues: &mut Vec<String>,
    ) -> Result<(), String> {
        if let Some(literal) = insn.operands().literal() {
            match literal {
                RawLiteral::Bits32(n) => value(&mut body.operands, b'L', &n.to_le_bytes())?,
                RawLiteral::Bits64(n) => value(&mut body.operands, b'L', &n.to_le_bytes())?,
                RawLiteral::Signed8(n) => {
                    value(&mut body.operands, b'L', &i32::from(n).to_le_bytes())?
                }
                RawLiteral::Signed16(n) => {
                    value(&mut body.operands, b'L', &i32::from(n).to_le_bytes())?
                }
            }
        }
        for (tag, target) in [(b'B', insn.branch_target()), (b'D', insn.payload_target())] {
            if let Some(target) = target {
                value(&mut body.operands, tag, &self.target(target)?.to_le_bytes())?;
                if matches!(insn.opcode(), 0x2b | 0x2c)
                    && let Some(decoded) = self.decoded
                    && let Some(cases) =
                        decoded.switch_cases_at(target).map_err(|e| e.to_string())?
                {
                    for case in cases {
                        let pc = insn
                            .pc()
                            .0
                            .checked_add_signed(case.offset.0)
                            .ok_or("switch target overflow")?;
                        let mut bytes = [0; 12];
                        bytes[..8].copy_from_slice(&case.key.to_le_bytes());
                        bytes[8..].copy_from_slice(&self.target(CodeUnitOffset(pc))?.to_le_bytes());
                        value(&mut body.operands, b'K', &bytes)?;
                    }
                }
            }
        }
        for site in insn.references() {
            // A P1 target-read failure is already in DecodedMethod::issues().
            // Preserve its unresolved operand without publishing the issue twice.
            let Some(Ok(fact)) = self
                .decoded
                .and_then(|d| d.reference_fact(insn.pc(), site.slot))
            else {
                value(
                    &mut body.operands,
                    b'?',
                    format!("{:?}", site.target).as_bytes(),
                )?;
                continue;
            };
            let symbol = match reference::resolve(cache, self.symbols, site.target, fact) {
                Ok(symbol) => symbol,
                Err(reason) => {
                    if reason == super::ROOT_CAPACITY {
                        return Err(reason);
                    }
                    issues.push(format!("@{}: {reason}", insn.pc().0));
                    value(
                        &mut body.operands,
                        b'?',
                        format!("{:?}", site.target).as_bytes(),
                    )?;
                    continue;
                }
            };
            value(&mut body.operands, symbol.tag, &symbol.id.to_le_bytes())?;
            body.has_symbol_dependency |= symbol.tag != b's';
            if symbol.tag == b'm' {
                body.calls.push(symbol.id);
            }
            if let Some(anchor) = symbol.anchor {
                body.anchors.push(anchor);
            }
        }
        Ok(())
    }

    fn protection(&self, decoded: &DecodedMethod<'_>, body: &mut Body) -> Result<(), String> {
        let mut bounds = decoded.try_boundaries();
        while let (Some(start), Some(end)) = (bounds.next(), bounds.next()) {
            let clauses = decoded
                .catch_clauses_at(CodeUnitOffset(start))
                .map_err(|e| e.to_string())?;
            for n in [
                self.target(CodeUnitOffset(start))?,
                self.target(CodeUnitOffset(end))?,
                clauses.len() as u32,
            ] {
                uint(&mut body.protection, n);
            }
            for (ty, pc) in clauses {
                uint(&mut body.protection, self.target(CodeUnitOffset(pc))?);
                match ty {
                    Some(id) => {
                        body.has_symbol_dependency = true;
                        let catch = decoded.catch_type(TypeId(id)).ok_or("missing catch type")?;
                        let descriptor =
                            reference::text(catch.descriptor().map_err(|e| e.to_string())?)?;
                        value(&mut body.protection, b't', descriptor.as_bytes())?;
                    }
                    None => value(&mut body.protection, b'*', &[])?,
                }
            }
        }
        Ok(())
    }
}

pub(super) fn collect(
    outcome: DecodedOutcome<'_>,
    method: &mut Method,
    cache: &mut reference::Cache,
    symbols: &super::Symbols,
) {
    if let Some(error) = &outcome.error {
        method.issues.push(error.to_string());
    }
    let Some(raw) = outcome.source() else {
        return;
    };
    let decoded = outcome.decoded.as_ref();
    let items = decoded.map_or(outcome.prefix.as_slice(), |d| d.items());
    let evidence = Evidence {
        raw,
        items,
        decoded,
        symbols,
    };
    let mut body = Body {
        frame: [raw.registers_size(), raw.ins_size(), raw.outs_size()],
        code_units: raw.code_units().0,
        code: Vec::with_capacity(items.len().min(MAX_BYTES / size_of::<Op>())),
        names: symbols.names(),
        operands: Vec::new(),
        raw_registers: Vec::new(),
        protection: Vec::new(),
        calls: Vec::new(),
        anchors: Vec::new(),
        has_symbol_dependency: false,
        complete_control: decoded.is_some(),
    };
    let mut words = Words {
        frame: u32::from(raw.registers_size()),
        parameter_base: u32::from(raw.registers_size() - raw.ins_size()),
        low: [u16::MAX; 32],
        high: HashMap::new(),
        next: 0,
    };
    let referenced_payloads: std::collections::HashSet<_> = items
        .iter()
        .filter_map(|item| {
            if let DecodedItem::Instruction(i) = item {
                i.payload_target()
            } else {
                None
            }
        })
        .collect();
    if let Some(d) = decoded {
        for (pc, e) in d.issues() {
            method.issues.push(format!("@{}: {e}", pc.0));
        }
        if let Err(error) = evidence.protection(d, &mut body) {
            body.complete_control = false;
            method.issues.push(error);
        }
    }
    for item in items {
        let mut shape = 0u8;
        let flow = match item {
            DecodedItem::Instruction(insn) => super::blocks::Flow::instruction(insn.spec()),
            DecodedItem::Payload(_) => super::blocks::Flow::PAYLOAD,
        };
        let start = body.operands.len();
        let regs = body.raw_registers.len();
        let result = (|| {
            if body.operands.len() > MAX_BYTES.saturating_sub(4096)
                || body.code.len() >= MAX_BYTES / size_of::<Op>()
                || body.raw_registers.len() > MAX_BYTES / 2
            {
                return Err(CAPACITY.to_owned());
            }
            let opcode = match item {
                DecodedItem::Instruction(insn) => {
                    words.write(insn, &mut body, &mut shape)?;
                    evidence.write_insn(insn, &mut body, cache, &mut method.issues)?;
                    normalized_opcode(insn.opcode())
                }
                DecodedItem::Payload(payload) => {
                    uint(&mut body.operands, 0);
                    match payload.kind() {
                        LayoutKind::ArrayData {
                            element_width,
                            element_count,
                        } => {
                            let width = element_width.0 as u32;
                            value(&mut body.operands, b'W', &width.to_le_bytes())?;
                            let len = (element_width.0)
                                .checked_mul(element_count as usize)
                                .ok_or("array payload too large")?;
                            let data = payload
                                .words(raw)
                                .map_err(|e| e.to_string())?
                                .get(8..)
                                .and_then(|data| data.get(..len))
                                .ok_or("truncated array payload")?;
                            value(&mut body.operands, b'A', data)?;
                            ARRAY
                        }
                        kind => {
                            // A referenced switch carries its checked keys and
                            // absolute targets at each use, including shared payloads.
                            if !referenced_payloads.contains(&payload.pc()) || decoded.is_none() {
                                value(
                                    &mut body.operands,
                                    b'K',
                                    payload.words(raw).map_err(|e| e.to_string())?,
                                )?;
                            }
                            if matches!(kind, LayoutKind::PackedSwitch { .. }) {
                                PACKED
                            } else {
                                SPARSE
                            }
                        }
                    }
                }
            };
            Ok(Op {
                flow,
                opcode,
                shape: (u16::from(opcode) << 8) | u16::from(shape),
                pc: item.pc().0,
                end: body.operands.len() as u32,
                register_end: body.raw_registers.len() as u32,
            })
        })();
        match result {
            Ok(op) => body.code.push(op),
            Err(error) => {
                body.complete_control = false;
                body.operands.truncate(start);
                body.raw_registers.truncate(regs);
                method.issues.push(format!("@{}: {error}", item.pc().0));
                if error == CAPACITY || error == super::ROOT_CAPACITY {
                    break;
                }
                // Preserve this site's location and the following independent facts.
                uint(&mut body.operands, 0);
                let opcode = match item {
                    DecodedItem::Instruction(i) => normalized_opcode(i.opcode()),
                    _ => 0,
                };
                body.code.push(Op {
                    flow,
                    pc: item.pc().0,
                    opcode,
                    shape: u16::from(opcode) << 8,
                    end: body.operands.len() as u32,
                    register_end: regs as u32,
                });
            }
        }
    }
    body.anchors.sort_unstable();
    body.anchors.dedup();
    body.calls.sort_unstable();
    body.calls.dedup();
    body.anchors.shrink_to_fit();
    body.calls.shrink_to_fit();
    body.operands.shrink_to_fit();
    body.raw_registers.shrink_to_fit();
    method.body = Some(body);
}
