//! Read-only adjacency in the selected source schedule, not graph discovery.
//! Whole operation portions can be separate blocks while remaining neighboring
//! events. Cross only direct sibling blocks with one copy-free original link.

use super::super::build::{bug, filled};
use super::super::*;

pub(super) struct Adjacency {
    pub incoming: Vec<u8>,
    pub sibling: Vec<Option<ControlId>>,
    next: Vec<Option<usize>>,
    previous: Vec<Option<usize>>,
    quiet_empty: Vec<bool>,
}
impl Adjacency {
    pub fn construct(code: &MethodCode) -> Result<Self, ExpressionError> {
        let mut result = Self {
            incoming: filled(code.blocks.len(), 0u8)?,
            sibling: filled(code.controls.len(), None)?,
            next: filled(code.blocks.len(), None)?,
            previous: filled(code.blocks.len(), None)?,
            quiet_empty: filled(code.blocks.len(), true)?,
        };
        for operation in &code.operations {
            let phases = &code.emission.operations[operation.source.0 as usize];
            if operation.effects != crate::p2::Effects::default()
                || operation.exceptional.is_some()
                || !phases.before.is_empty()
                || !phases.snapshots.is_empty()
                || !phases.after.is_empty()
            {
                result.quiet_empty[operation.node.0 as usize] = false;
            }
        }
        for transfer in &code.transfers {
            let count = result
                .incoming
                .get_mut(transfer.destination.0 as usize)
                .ok_or(bug("transfer names an unknown node"))?;
            *count = count.saturating_add(1).min(2);
        }
        for control in &code.controls {
            let emitted = &code.emission.controls[control.id.0 as usize];
            if matches!(
                emitted.layout,
                EmissionLayout::Sequence | EmissionLayout::Graph
            ) {
                for pair in emitted.children(control).windows(2) {
                    let slot = result
                        .sibling
                        .get_mut(pair[0].0 as usize)
                        .ok_or(bug("sequence names an unknown child"))?;
                    if slot.replace(pair[1]).is_some() {
                        return Err(bug("control has multiple sequence owners"));
                    }
                }
            }
        }
        for node in 0..code.blocks.len() {
            let Some(target) = result.successor(code, node) else {
                continue;
            };
            result.next[node] = Some(target);
            if result.previous[target].replace(node).is_some() {
                return Err(bug("linear source portion has two predecessors"));
            }
        }
        Ok(result)
    }
    fn successor(&self, code: &MethodCode, node: usize) -> Option<usize> {
        let block = code.emission.blocks.get(node)?;
        let BlockEnd::Transfers(links) = &block.end else {
            return None;
        };
        let [link] = links.as_slice() else {
            return None;
        };
        let original = code.transfers.get(link.0 as usize)?;
        let transfer = code.emission.transfers.get(link.0 as usize)?;
        if original.edge.is_some()
            || !matches!(transfer.spelling, TransferSpelling::Fallthrough)
            || !transfer.copies.is_empty()
        {
            return None;
        }
        let target = original.destination.0 as usize;
        if code.emission.blocks.get(target)?.show_label
            || self.incoming.get(target).copied()? != 1
            || code.blocks.get(node)?.original_block != code.blocks.get(target)?.original_block
        {
            return None;
        }
        let owner = code.labels.get(node)?.owner;
        let source = code.emission.controls.get(owner.0 as usize)?;
        if !matches!(source.layout, EmissionLayout::Block(n) if n.0 as usize == node)
            || source.continuation.is_some()
        {
            return None;
        }
        let next = self.sibling.get(owner.0 as usize).copied().flatten()?;
        let next = code.emission.controls.get(next.0 as usize)?;
        if !matches!(next.layout, EmissionLayout::Block(n) if n.0 as usize == target)
            || next.continuation.is_some()
        {
            return None;
        }
        Some(target)
    }
    pub fn empty_is_quiet(&self, node: usize) -> bool {
        self.quiet_empty.get(node).copied().unwrap_or(false)
    }
    pub fn next_node(&self, node: usize) -> Option<usize> {
        self.next.get(node).copied().flatten()
    }
    pub fn next_action(
        &self,
        code: &MethodCode,
        node: usize,
        index: usize,
    ) -> Option<(usize, usize, BodyAction)> {
        let actions = &code.emission.blocks.get(node)?.actions;
        if let Some(&action) = actions.get(index.checked_add(1)?) {
            return Some((node, index + 1, action));
        }
        if index.checked_add(1)? != actions.len() {
            return None;
        }
        let target = self.next_node(node)?;
        Some((
            target,
            0,
            *code.emission.blocks.get(target)?.actions.first()?,
        ))
    }
    pub fn previous_action(
        &self,
        code: &MethodCode,
        node: usize,
        index: usize,
    ) -> Option<(usize, usize, BodyAction)> {
        if let Some(index) = index.checked_sub(1) {
            return Some((
                node,
                index,
                *code.emission.blocks.get(node)?.actions.get(index)?,
            ));
        }
        let source = self.previous.get(node).copied().flatten()?;
        let actions = &code.emission.blocks.get(source)?.actions;
        let index = actions.len().checked_sub(1)?;
        Some((source, index, actions[index]))
    }
}
