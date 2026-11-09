//! One source-member index over the installed contraction forest. Children
//! determine static membership; their order does not determine execution.

use super::super::build::*;
use super::super::*;

enum Work {
    Enter(RegionId),
    End(RegionId, usize),
}

pub(super) fn construct(f: &mut RegionFacts) -> Result<(), RegionError> {
    let root = f.root.ok_or(bug("membership lost the installed root"))?;
    let mut nodes = Vec::new();
    crate::substrate::owned::reserve(&mut nodes, f.nodes.len())?;
    let mut positions = filled(f.nodes.len(), usize::MAX)?;
    let mut work = Vec::new();
    push(&mut work, Work::Enter(root))?;
    while let Some(task) = work.pop() {
        match task {
            Work::Enter(id) => {
                let region = f
                    .regions
                    .get(id.0 as usize)
                    .ok_or(bug("membership left its region arena"))?;
                push(&mut work, Work::End(id, nodes.len()))?;
                if let RegionKind::Block(node) = region.kind {
                    let position = positions
                        .get_mut(node.0 as usize)
                        .ok_or(bug("membership names an unknown leaf"))?;
                    if *position != usize::MAX {
                        return Err(bug("membership leaf has two static owners"));
                    }
                    *position = nodes.len();
                    nodes.push(node);
                }
                for &child in region.children.iter().rev() {
                    push(&mut work, Work::Enter(child))?;
                }
            }
            Work::End(id, start) => {
                let region = &mut f.regions[id.0 as usize];
                if nodes.len() - start != region.member_count {
                    return Err(bug("fragment children lost original membership"));
                }
                region.member_range = Some(start..nodes.len());
            }
        }
    }
    if nodes.len() != f.nodes.len() {
        return Err(bug("membership lost an original leaf"));
    }
    for region in &mut f.regions {
        if region.member_count == 0 {
            region.member_range.get_or_insert(0..0);
        }
    }
    f.member_nodes = nodes;
    f.member_positions = positions;
    Ok(())
}
