//! Evidence for a straight-line callee body replacing a static call in its original caller.
use std::collections::HashMap;

use super::index::{Body, Format, Method};

mod jvm;

/// Context used to establish a register correspondence or corroborate the enclosing caller.
/// Method and fragment delivery still retain their complete original ranges.
const CONTEXT: usize = 32;

pub(super) struct InlineSite {
    pub start: usize,
    pub count: usize,
    pub corroborates_outer: bool,
}

pub(super) fn is_call(body: &Body, index: usize) -> bool {
    let opcode = match body.format {
        // Range invokes have already been normalized by the index.
        Format::Dex => 0x71,
        Format::Jvm => 0xb8,
    };
    body.code.get(index).is_some_and(|op| op.opcode == opcode)
}

fn straight(opcode: u8) -> bool {
    !matches!(opcode, 0x0d..=0x11 | 0x26..=0x2c | 0x32..=0x40)
}

pub(super) fn eligible(method: &Method) -> Option<usize> {
    let helper = method.body.as_ref()?;
    if helper.format == Format::Jvm {
        return jvm::eligible(method, helper).map(|site| site.count);
    }
    let (last, prefix) = helper.code.split_last()?;
    (helper.protection.is_empty()
        && matches!(last.opcode, 0x0e..=0x11)
        && prefix.len() >= 4
        && prefix.iter().all(|op| straight(op.opcode)))
    .then_some(prefix.len())
}

/// Each register word maps both ways, including overlapping wide operands.
#[derive(Default)]
struct Words {
    before: HashMap<u32, u32>,
    after: HashMap<u32, u32>,
}

impl Words {
    fn bind(&mut self, a: u32, b: u32) -> bool {
        *self.before.entry(a).or_insert(b) == b && *self.after.entry(b).or_insert(a) == a
    }

    fn caller_word(&self, word: u32) -> Option<u32> {
        // Incoming words use the same ordinal in both canonical frames.
        self.before
            .get(&word)
            .copied()
            .or_else(|| (word % 2 == 1).then_some(word))
    }

    fn match_outer_op(&mut self, caller: &Body, i: usize, target: &Body, j: usize) -> bool {
        if !same_operation(caller, i, target, j) {
            return false;
        }
        let mut right = target.register_words(j);
        for a in caller.register_words(i) {
            let Some(b) = right.next() else { return false };
            if (a % 2 == 1 || b % 2 == 1) && a != b || !self.bind(a, b) {
                return false;
            }
        }
        right.next().is_none()
    }
}

fn same_operation(a: &Body, i: usize, b: &Body, j: usize) -> bool {
    a.code[i].opcode == b.code[j].opcode
        && a.operand_without_registers(i) == b.operand_without_registers(j)
}

fn aligned_target(aligned: &[(usize, usize)], i: usize) -> Option<usize> {
    let row = aligned.binary_search_by_key(&i, |&(old, _)| old).ok()?;
    Some(aligned[row].1)
}

/// Candidate boundaries come from the adjacent aligned instructions. If the ordered
/// region is the only insertion, its independently checked positional boundary also
/// covers ties between a removed invoke and an invoke inside the helper.
pub(super) fn replaced_count(helper: &Method, count: usize) -> Option<usize> {
    let body = helper.body.as_ref()?;
    Some(match body.format {
        Format::Dex if body.code.get(count)?.opcode == 0x0e => 1,
        Format::Dex => 2,
        Format::Jvm => jvm::eligible(helper, body)?.descriptor.parameters.len() + 1,
    })
}

pub(super) enum Context {
    Dex(Vec<(usize, usize)>),
    Jvm(jvm::Outside),
}

impl Context {
    pub(super) fn new(old: &Body, new: &Body) -> Self {
        match old.format {
            Format::Dex => Self::Dex(super::alignment::align(&old.code, &new.code)),
            Format::Jvm => Self::Jvm(jvm::Outside::new(old, new)),
        }
    }
}
pub(super) fn find(
    caller: &Method,
    helper: &Method,
    target: &Method,
    invoke: usize,
    context: &Context,
) -> Option<InlineSite> {
    let source = caller.body.as_ref()?;
    let embedded = helper.body.as_ref()?;
    let updated = target.body.as_ref()?;
    match (source.format, embedded.format, updated.format, context) {
        (Format::Dex, Format::Dex, Format::Dex, Context::Dex(aligned)) => {
            dex_window(source, embedded, updated, invoke, aligned)
        }
        (Format::Jvm, Format::Jvm, Format::Jvm, Context::Jvm(outside)) => {
            jvm::find(caller, helper, target, invoke, outside)
        }
        _ => None,
    }
}

fn dex_window(
    caller: &Body,
    helper: &Body,
    target: &Body,
    invoke: usize,
    aligned: &[(usize, usize)],
) -> Option<InlineSite> {
    let count = helper.code.len().checked_sub(1)?;
    let ret = helper.code[count].opcode;
    if !is_call(caller, invoke)
        || !(0x0e..=0x11).contains(&ret)
        || !helper.protection.is_empty()
        || count < 4
        || caller.frame[1] != target.frame[1]
        || caller.registers(invoke).len() != usize::from(helper.frame[1])
    {
        return None;
    }
    let successor = if ret == 0x0e {
        invoke + 1
    } else {
        if caller.code.get(invoke + 1)?.opcode != ret - 5 {
            return None;
        }
        invoke + 2
    };
    let before = invoke
        .checked_sub(1)
        .map_or(Some(0), |i| aligned_target(aligned, i)?.checked_add(1));
    let after = if successor == caller.code.len() {
        Some(target.code.len())
    } else {
        aligned_target(aligned, successor)
    };
    let positional = (target.code.len().checked_sub(count)
        == caller.code.len().checked_sub(successor - invoke))
    .then_some(invoke);
    let starts = [before, after.and_then(|n| n.checked_sub(count)), positional];
    let mut result = None;
    for (idx, candidate) in starts.iter().enumerate() {
        let Some(start) = *candidate else { continue };
        if starts[..idx].contains(candidate) {
            continue;
        }
        let Some(end) = start
            .checked_add(count)
            .filter(|&end| end <= target.code.len())
        else {
            continue;
        };
        let prefix_lines_up = before == Some(start) || invoke == start;
        let suffix_lines_up =
            after == Some(end) || caller.code.len() - successor == target.code.len() - end;
        if !prefix_lines_up || !suffix_lines_up {
            continue;
        }
        if let Some(site) = window(caller, helper, target, invoke, successor, start, end) {
            if result.is_some() {
                return None;
            }
            result = Some(site);
        }
    }
    result
}

fn window(
    caller: &Body,
    helper: &Body,
    target: &Body,
    invoke: usize,
    successor: usize,
    start: usize,
    end: usize,
) -> Option<InlineSite> {
    let mut prior = 0;
    while prior < CONTEXT.min(invoke).min(start) {
        let (a, b) = (invoke - prior - 1, start - prior - 1);
        if !straight(caller.code[a].opcode) || !same_operation(caller, a, target, b) {
            break;
        }
        prior += 1;
    }
    if prior == 0 && (invoke != 0 || start != 0) {
        return None;
    }
    let mut outer = Words::default();
    let mut named_before = false;
    for d in (1..=prior).rev() {
        let (i, j) = (invoke - d, start - d);
        if !outer.match_outer_op(caller, i, target, j) {
            return None;
        }
        named_before |= outer_call(caller, i, helper);
    }
    let inputs: Vec<_> = caller.register_words(invoke).collect();
    // Local arguments need a checked prefix from method entry. Matching a use of
    // an otherwise unknown local does not establish what entered that register.
    if inputs.iter().any(|w| w.is_multiple_of(2)) && (prior != invoke || prior != start) {
        return None;
    }
    let mut inner = Words::default();
    for (i, j) in (start..end).enumerate() {
        if !straight(helper.code[i].opcode) || !same_operation(helper, i, target, j) {
            return None;
        }
        let mut actual = target.register_words(j);
        for word in helper.register_words(i) {
            let translated = actual.next()?;
            if word % 2 == 1
                && outer.caller_word(*inputs.get((word / 2) as usize)?) != Some(translated)
            {
                return None;
            }
            if !inner.bind(word, translated) {
                return None;
            }
        }
        if actual.next().is_some() {
            return None;
        }
    }
    let count = end - start;
    let mut used = helper.code[count].opcode == 0x0e;
    if !used {
        let mut destination = caller.register_words(invoke + 1);
        for word in helper.register_words(count) {
            if !outer.bind(destination.next()?, *inner.before.get(&word)?) {
                return None;
            }
        }
        if destination.next().is_some() {
            return None;
        }
    }
    let mut named_after = false;
    for (i, j) in (successor..caller.code.len())
        .zip(end..target.code.len())
        .take(CONTEXT)
    {
        if !outer.match_outer_op(caller, i, target, j) {
            if i == successor || !used {
                return None;
            }
            break;
        }
        named_after |= outer_call(caller, i, helper);
        if !used {
            let dest = caller.registers(invoke + 1).first().copied()?;
            let regs = caller.registers(i);
            let uses = if reads_first(caller.code[i].opcode) {
                regs
            } else {
                regs.get(1..).unwrap_or_default()
            };
            used = uses.contains(&dest);
            // A write to either result word before the observed use loses it.
            if !used
                && caller
                    .register_words(invoke + 1)
                    .any(|r| caller.register_words(i).any(|w| w == r))
            {
                return None;
            }
        }
        if used && named_after || !straight(caller.code[i].opcode) {
            break;
        }
    }
    used.then_some(InlineSite {
        start,
        count,
        corroborates_outer: named_before && named_after,
    })
}

fn outer_call(caller: &Body, index: usize, helper: &Body) -> bool {
    caller
        .callee(index)
        .is_some_and(|name| !helper.call_names().any(|callee| callee == name))
}

// In the remaining instruction families, the first register is a destination.
// A repeated destination/source still contributes a read in its source slot.
fn reads_first(op: u8) -> bool {
    matches!(op, 0x0f..=0x11 | 0x1d..=0x1f | 0x24 | 0x26..=0x27 | 0x2b..=0x2c
        | 0x32..=0x3d | 0x4b..=0x51 | 0x59..=0x5f | 0x67..=0x72 | 0xb0..=0xcf | 0xfa | 0xfc)
}
