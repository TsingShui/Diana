//! Align original instructions and compare ordered DEX facts within a method pair.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use super::index::{Body, Method, Op};
use super::inline;
use super::matching::Matching;
use super::{Change, Fragment, MatchedMethod, MethodLocation, Site};

pub(super) fn method(
    old: &Method,
    new: &Method,
    symbols: &HashMap<String, String>,
    symbols_renamed: bool,
) -> MatchedMethod {
    let mut changes = Vec::new();
    if old.access != new.access {
        changes.push(attribute(
            "access_flags",
            format!("0x{:x}", old.access),
            format!("0x{:x}", new.access),
        ));
    }
    let prototype = remap_prototype(&old.prototype, symbols);
    if prototype != new.prototype {
        changes.push(attribute(
            "prototype",
            old.prototype.clone(),
            new.prototype.clone(),
        ));
    }
    let complete = old.issues.is_empty() && new.issues.is_empty();
    match (&old.body, &new.body) {
        (Some(a), Some(b)) => {
            compare_bodies(a, b, symbols, symbols_renamed, complete, &mut changes)
        }
        (None, Some(_)) if complete => changes.push(Change {
            kind: "method_body",
            old: Some(site("declaration only".into())),
            new: Some(site("DEX code".into())),
        }),
        (Some(_), None) if complete => changes.push(Change {
            kind: "method_body",
            old: Some(site("DEX code".into())),
            new: Some(site("declaration only".into())),
        }),
        _ => {}
    }
    let status = if !complete {
        "unknown"
    } else if changes.is_empty() {
        "unchanged"
    } else {
        "changed"
    };
    MatchedMethod {
        old: old.into(),
        new: new.into(),
        evidence: Vec::new(),
        status,
        changes,
    }
}

fn remap_prototype(prototype: &str, symbols: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(prototype.len());
    let mut tail = prototype;
    while let Some(start) = tail.find('L') {
        let Some(end) = tail[start..].find(';').map(|end| start + end + 1) else {
            break;
        };
        out.push_str(&tail[..start]);
        let descriptor = &tail[start..end];
        out.push_str(symbols.get(descriptor).map_or(descriptor, String::as_str));
        tail = &tail[end..];
    }
    out.push_str(tail);
    out
}

fn site(text: String) -> Site {
    Site { offset: None, text }
}

fn attribute(kind: &'static str, old: String, new: String) -> Change {
    Change {
        kind,
        old: Some(site(old)),
        new: Some(site(new)),
    }
}

fn instruction(body: &Body, i: usize) -> Site {
    Site {
        offset: Some(body.code[i].pc),
        text: body.describe_op(i),
    }
}

fn compare_bodies(
    old: &Body,
    new: &Body,
    symbols: &HashMap<String, String>,
    symbols_renamed: bool,
    complete: bool,
    changes: &mut Vec<Change>,
) {
    if old.code.len() == new.code.len()
        && old.frame[1] == new.frame[1]
        && old.operands == new.operands
        && old.protection == new.protection
        && old
            .code
            .iter()
            .zip(&new.code)
            .all(|(a, b)| a.opcode == b.opcode && a.end == b.end)
        && (!symbols_renamed || !old.has_owner_member_ref())
    {
        return;
    }

    if old.frame[1] != new.frame[1] {
        changes.push(attribute(
            "input_register_words",
            old.frame[1].to_string(),
            new.frame[1].to_string(),
        ));
    }
    let (aligned, relocated) = align_bodies(old, new, complete);
    let mut targets = vec![None; old.code.len() + 1];
    for &(i, j) in &aligned {
        targets[i] = Some(j);
    }
    if complete {
        targets[old.code.len()] = Some(new.code.len());
    }
    if complete {
        changes.extend(
            old.protection_changes(new, symbols, &aligned, &targets)
                .into_iter()
                .map(|change| Change {
                    kind: "exception_handlers",
                    old: change.old.map(|(offset, text)| Site { offset, text }),
                    new: change.new.map(|(offset, text)| Site { offset, text }),
                }),
        );
    }
    let mut new_aligned = relocated.then(|| vec![false; new.code.len()]);
    let mut cursor = (0, 0);
    for (i, j) in aligned {
        if complete {
            if let Some(ref mut covered) = new_aligned {
                covered[j] = true;
                removed_added(old, cursor.0..i, new, 0..0, changes);
            } else {
                removed_added(old, cursor.0..i, new, cursor.1..j, changes);
            }
        }
        if old.code[i].opcode != new.code[j].opcode
            || !old.operands_match(i, new, j, symbols, &targets)
        {
            changes.push(Change {
                kind: "instruction",
                old: Some(instruction(old, i)),
                new: Some(instruction(new, j)),
            });
        } else if relocated && old.falls_through(i) && targets[i + 1] != Some(j + 1) {
            changes.push(Change {
                kind: "fallthrough",
                old: Some(fallthrough(old, i)),
                new: Some(fallthrough(new, j)),
            });
        }
        cursor = (i + 1, j + 1);
    }
    if let Some(covered) = new_aligned {
        removed_added(old, cursor.0..old.code.len(), new, 0..0, changes);
        for j in (0..new.code.len()).filter(|&j| !covered[j]) {
            removed_added(old, 0..0, new, j..j + 1, changes);
        }
    } else if complete {
        removed_added(
            old,
            cursor.0..old.code.len(),
            new,
            cursor.1..new.code.len(),
            changes,
        );
    }
}

fn fallthrough(body: &Body, index: usize) -> Site {
    Site {
        offset: Some(body.code[index].pc),
        text: if body.falls_through(index) {
            format!("fallthrough @0x{:x}", body.code[index + 1].pc)
        } else {
            "no fallthrough".into()
        },
    }
}

fn removed_added(
    old: &Body,
    removed: Range<usize>,
    new: &Body,
    added: Range<usize>,
    out: &mut Vec<Change>,
) {
    out.extend(removed.map(|i| Change {
        kind: "instruction_removed",
        old: Some(instruction(old, i)),
        new: None,
    }));
    out.extend(added.map(|j| Change {
        kind: "instruction_added",
        old: None,
        new: Some(instruction(new, j)),
    }));
}

type AlignedInstructions = Vec<(usize, usize)>;

fn align_bodies(old: &Body, new: &Body, complete: bool) -> (AlignedInstructions, bool) {
    if complete
        && (old.code.len() != new.code.len()
            || old
                .code
                .iter()
                .zip(&new.code)
                .any(|(a, b)| shape(a) != shape(b)))
        && let Some(relocated) = relocated_blocks(old, new)
    {
        return (relocated, true);
    }
    (align(&old.code, &new.code), false)
}

fn relocated_blocks(old: &Body, new: &Body) -> Option<AlignedInstructions> {
    let old_starts = old.block_boundaries()?;
    let new_starts = new.block_boundaries()?;
    let before: Vec<_> = old_starts.windows(2).map(|r| r[0]..r[1]).collect();
    let after: Vec<_> = new_starts.windows(2).map(|r| r[0]..r[1]).collect();
    if before.len() < 3 || after.len() < 3 {
        return None;
    }
    let mut by_shape = HashMap::new();
    for (is_new, body, blocks) in [(false, old, &before), (true, new, &after)] {
        for (i, block) in blocks.iter().enumerate().skip(1) {
            let mut hash = DefaultHasher::new();
            for op in &body.code[block.clone()] {
                shape(op).hash(&mut hash);
            }
            let entry = by_shape
                .entry(hash.finish())
                .or_insert(([None; 2], [false; 2]));
            let side = usize::from(is_new);
            entry.1[side] |= entry.0[side].replace(i).is_some();
        }
    }

    let mut old_blocks = vec![None; before.len()];
    let mut new_blocks = vec![None; after.len()];
    let mut queue = Vec::new();
    let same_block = |i: usize, j: usize| {
        let a = &old.code[before[i].clone()];
        let b = &new.code[after[j].clone()];
        a.len() == b.len() && a.iter().zip(b).all(|(a, b)| shape(a) == shape(b))
    };
    if same_block(0, 0) {
        queue.push((0, 0));
    }
    for (slots, duplicates) in by_shape.into_values() {
        if let ([Some(i), Some(j)], [false, false]) = (slots, duplicates)
            && same_block(i, j)
        {
            queue.push((i, j));
        }
    }
    // Direct fallthrough can disambiguate identical blocks, such as the goto
    // following each already paired conditional. Never cross a claim made by
    // another distinct block to follow a changed edge.
    for &(i, j) in &queue {
        old_blocks[i] = Some(j);
        new_blocks[j] = Some(i);
    }
    let mut cursor = 0;
    while let Some(&(i, j)) = queue.get(cursor) {
        cursor += 1;
        if old_blocks.get(i + 1) == Some(&None)
            && new_blocks.get(j + 1) == Some(&None)
            && old.falls_through(before[i].end - 1)
            && new.falls_through(after[j].end - 1)
            && same_block(i + 1, j + 1)
        {
            old_blocks[i + 1] = Some(j + 1);
            new_blocks[j + 1] = Some(i + 1);
            queue.push((i + 1, j + 1));
        }
    }
    let mut ordinals = old_blocks.iter().copied().flatten();
    let mut previous = ordinals.next()?;
    if !ordinals.any(|j| {
        let inverted = j < previous;
        previous = j;
        inverted
    }) {
        return None;
    }
    let mut old_slots = vec![None; old.code.len()];
    let mut new_slots = vec![false; new.code.len()];
    for (i, j) in queue {
        for (a, b) in before[i].clone().zip(after[j].clone()) {
            old_slots[a] = Some(b);
            new_slots[b] = true;
        }
    }
    for (i, j) in align(&old.code, &new.code) {
        if old_slots[i].is_none() && !new_slots[j] {
            old_slots[i] = Some(j);
            new_slots[j] = true;
        }
    }
    Some(
        old_slots
            .into_iter()
            .enumerate()
            .filter_map(|(i, j)| j.map(|j| (i, j)))
            .collect(),
    )
}

/// Keep unchanged prefixes and suffixes. In the middle, use unique shape
/// matches in increasing order, then align the intervening local edits.
pub(super) fn align(old: &[Op], new: &[Op]) -> Vec<(usize, usize)> {
    let prefix = old
        .iter()
        .zip(new)
        .take_while(|(a, b)| shape(a) == shape(b))
        .count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| shape(a) == shape(b))
        .count();
    let mut result: Vec<_> = (0..prefix).map(|i| (i, i)).collect();
    let ends = (old.len() - suffix, new.len() - suffix);
    let mut cursor = (prefix, prefix);
    let anchors = unique_in_order(&old[prefix..ends.0], &new[prefix..ends.1]);
    for (i, j) in anchors
        .into_iter()
        .map(|(i, j)| (prefix + i, prefix + j))
        .chain(std::iter::once(ends))
    {
        local_alignment(old, cursor.0..i, new, cursor.1..j, &mut result);
        if (i, j) != ends {
            result.push((i, j));
        }
        cursor = (i + 1, j + 1);
    }
    result.extend((ends.0..old.len()).zip(ends.1..new.len()));
    result
}

fn shape(op: &Op) -> (u8, u16) {
    (op.opcode, op.shape)
}

fn unique_in_order(old: &[Op], new: &[Op]) -> Vec<(usize, usize)> {
    if old.is_empty() || new.is_empty() {
        return Vec::new();
    }
    let mut lookup = HashMap::<_, (Option<usize>, Option<usize>)>::new();
    for (i, value) in new.iter().enumerate() {
        lookup
            .entry(shape(value))
            .and_modify(|slot| slot.0 = None)
            .or_insert((Some(i), None));
    }
    for (i, value) in old.iter().enumerate() {
        if let Some(slot) = lookup.get_mut(&shape(value)) {
            if slot.1.is_some() {
                slot.0 = None;
            } else {
                slot.1 = Some(i);
            }
        }
    }
    let points: Vec<_> = old
        .iter()
        .enumerate()
        .filter_map(|(i, value)| {
            let &(j, only) = lookup.get(&shape(value))?;
            j.filter(|_| only == Some(i)).map(|j| (i, j))
        })
        .collect();
    let mut tails = Vec::<usize>::new();
    let mut previous = Vec::with_capacity(points.len());
    for (index, &(_, j)) in points.iter().enumerate() {
        let length = tails.partition_point(|&p| points[p].1 < j);
        previous.push(length.checked_sub(1).map(|i| tails[i]));
        if length == tails.len() {
            tails.push(index);
        } else {
            tails[length] = index;
        }
    }
    let mut out = Vec::with_capacity(tails.len());
    let mut index = tails.last().copied();
    while let Some(point) = index {
        out.push(points[point]);
        index = previous[point];
    }
    out.reverse();
    out
}

fn local_alignment(
    old: &[Op],
    a: Range<usize>,
    new: &[Op],
    b: Range<usize>,
    out: &mut Vec<(usize, usize)>,
) {
    if a.is_empty() || b.is_empty() {
        return;
    }
    let (m, n) = (a.len(), b.len());
    // A repeated long run has no defensible per-item shift without additional
    // evidence. Its common prefix/suffix is already established by align().
    if m.saturating_mul(n) > 4096 {
        if m == n {
            out.extend(a.zip(b));
        }
        return;
    }
    let stride = n + 1;
    let mut distance = vec![0u16; (m + 1) * stride];
    for i in (0..=m).rev() {
        distance[i * stride + n] = (m - i) as u16;
    }
    for j in (0..n).rev() {
        distance[m * stride + j] = (n - j) as u16;
    }
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            let diag = distance[(i + 1) * stride + j + 1]
                + u16::from(shape(&old[a.start + i]) != shape(&new[b.start + j]));
            distance[i * stride + j] = diag
                .min(1 + distance[(i + 1) * stride + j])
                .min(1 + distance[i * stride + j + 1]);
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < m && j < n {
        let same = shape(&old[a.start + i]) == shape(&new[b.start + j]);
        let best = distance[i * stride + j];
        if best == distance[(i + 1) * stride + j + 1] + u16::from(!same) {
            out.push((a.start + i, b.start + j));
            i += 1;
            j += 1;
        } else if best == 1 + distance[(i + 1) * stride + j] {
            i += 1;
        } else {
            j += 1;
        }
    }
}

pub(super) fn fragments(old: &[Method], new: &[Method], matched: &Matching) -> Vec<Fragment> {
    let mut unknown = HashMap::new();
    for (id, method) in old.iter().enumerate() {
        let target = if matched.old_to_new[id].is_none() && method.issues.is_empty() {
            method.body.as_ref().and_then(inline::eligible).map(|_| id)
        } else {
            None
        };
        unknown
            .entry(method.identity())
            .and_modify(|id| *id = None)
            .or_insert(target);
    }
    if unknown.values().all(Option::is_none) {
        return Vec::new();
    }
    let mut fragments = Vec::new();
    for pair in &matched.pairs {
        let before = &old[pair.old];
        let after = &new[pair.new];
        let (Some(a), Some(b)) = (&before.body, &after.body) else {
            continue;
        };
        if a.calls.is_empty() || !before.issues.is_empty() || !after.issues.is_empty() {
            continue;
        }
        let mut aligned = None;
        for i in 0..a.code.len() {
            if !inline::is_call(a.code[i].opcode) {
                continue;
            }
            let Some(id) = a
                .callee(i)
                .and_then(|call| unknown.get(call))
                .copied()
                .flatten()
            else {
                continue;
            };
            let Some(helper) = &old[id].body else {
                continue;
            };
            let targets = aligned.get_or_insert_with(|| align(&a.code, &b.code));
            let Some(site) = inline::find(a, helper, b, i, targets) else {
                continue;
            };
            fragments.push(Fragment {
                old: MethodLocation::from(&old[id]),
                new: MethodLocation::from(after),
                old_offset: helper.code[0].pc,
                new_offset: b.code[site.start].pc,
                instruction_count: site.count,
            });
        }
    }
    fragments.sort_unstable_by(|a, b| {
        (&a.old.at, &a.new.at, a.new_offset).cmp(&(&b.old.at, &b.new.at, b.new_offset))
    });
    fragments.dedup_by(|a, b| {
        a.old.at == b.old.at && a.new.at == b.new.at && a.new_offset == b.new_offset
    });
    fragments
}
