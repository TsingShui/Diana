//! Frozen adjacency for occurrence-sensitive flow. The builder appends edges
//! in evaluation order; freezing sorts and deduplicates each public edge row.

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex, OnceLock},
};

use super::{
    index::{FlowBlocks, PointNumbers},
    SemanticFlowEdgeKind, SemanticFlowPoint,
};

pub(super) type FlowEdge = (SemanticFlowPoint, SemanticFlowPoint, SemanticFlowEdgeKind);

#[derive(Debug, Clone)]
struct Rows<T> {
    offsets: Vec<usize>,
    items: Vec<T>,
}

impl<T: Copy + Ord> Rows<T> {
    fn counted(mut counts: Vec<usize>, initial: T) -> Self {
        let mut total = 0;
        for count in &mut counts {
            let degree = *count;
            *count = total;
            total += degree;
        }
        Self {
            offsets: counts,
            items: vec![initial; total],
        }
    }

    fn add(&mut self, row: usize, item: T) {
        let next = &mut self.offsets[row];
        self.items[*next] = item;
        *next += 1;
    }

    fn finish(&mut self) {
        // The insertion cursors now hold row ends. Recover each start and
        // compact distinct entries in place, retaining the public row order.
        let count = self.offsets.len() - 1;
        let mut start = 0;
        let mut write = 0;
        for cursor in &mut self.offsets[..count] {
            let end = *cursor;
            *cursor = write;
            if end == start + 1 {
                self.items[write] = self.items[start];
                write += 1;
            } else if end > start {
                self.items[start..end].sort_unstable();
                for read in start..end {
                    if write == *cursor || self.items[read] != self.items[write - 1] {
                        self.items[write] = self.items[read];
                        write += 1;
                    }
                }
            }
            start = end;
        }
        self.offsets[count] = write;
        self.items.truncate(write);
    }

    fn get(&self, index: usize) -> &[T] {
        &self.items[self.offsets[index]..self.offsets[index + 1]]
    }
}

#[derive(Debug)]
pub(in crate::ir::analysis) struct GraphIndex {
    pub(super) points: Arc<PointNumbers>,
    successors: Rows<usize>,
    predecessors: Rows<(usize, SemanticFlowEdgeKind)>,
    successor_points: OnceLock<Box<[SemanticFlowPoint]>>,
    pub(super) entries: Vec<usize>,
    blocks: OnceLock<FlowBlocks>,
    pub(super) cleanup: bool,
    mandatory_paths: OnceLock<Mutex<MandatoryPaths>>,
}

impl Default for GraphIndex {
    fn default() -> Self {
        Self::new(&[], &BTreeSet::new())
    }
}

impl GraphIndex {
    pub(super) fn new(edges: &[FlowEdge], entries: &BTreeSet<SemanticFlowPoint>) -> Self {
        let observations = edges.len().saturating_mul(2).saturating_add(entries.len());
        let points = Arc::new(PointNumbers::from_occurrences(
            edges
                .iter()
                .flat_map(|&(source, target, _)| [source, target])
                .chain(entries.iter().copied()),
            observations,
        ));
        let count = points.len();
        let mut outgoing = vec![0; count + 1];
        let mut incoming = vec![0; count + 1];
        let mut cleanup = false;
        for &(source, target, kind) in edges {
            outgoing[points.get(source).expect("indexed source")] += 1;
            incoming[points.get(target).expect("indexed target")] += 1;
            cleanup |= matches!(
                kind,
                SemanticFlowEdgeKind::FinallyEnter(_)
                    | SemanticFlowEdgeKind::FinallyResume(_)
                    | SemanticFlowEdgeKind::FinallyOverride
            );
        }
        let mut successors = Rows::counted(outgoing, 0);
        let mut predecessors = Rows::counted(incoming, (0, SemanticFlowEdgeKind::Normal));
        for &(source, target, kind) in edges {
            let source = points.get(source).expect("indexed source");
            let target = points.get(target).expect("indexed target");
            successors.add(source, target);
            predecessors.add(target, (source, kind));
        }
        successors.finish();
        predecessors.finish();
        let entries = entries
            .iter()
            .map(|&entry| points.get(entry).expect("indexed entry"))
            .collect();
        Self {
            points,
            successors,
            predecessors,
            successor_points: OnceLock::new(),
            entries,
            blocks: OnceLock::new(),
            cleanup,
            mandatory_paths: OnceLock::new(),
        }
    }

    pub(super) fn must_reach(&self, source: usize, target: usize) -> bool {
        self.mandatory_paths
            .get_or_init(Mutex::default)
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .search(self, source, target)
    }

    pub(in crate::ir::analysis) fn len(&self) -> usize {
        self.points.len()
    }

    pub(in crate::ir::analysis) fn point(&self, node: usize) -> SemanticFlowPoint {
        self.points.point(node)
    }

    pub(in crate::ir::analysis) fn id(&self, point: SemanticFlowPoint) -> Option<usize> {
        self.points.get(point)
    }

    pub(super) fn blocks(&self) -> &FlowBlocks {
        self.blocks.get_or_init(|| FlowBlocks::new(self))
    }

    pub(super) fn successor_count(&self) -> usize {
        self.successors.items.len()
    }

    #[cfg(test)]
    pub(super) fn has_projected_successors(&self) -> bool {
        self.successor_points.get().is_some()
    }

    pub(super) fn successors(&self, node: usize) -> &[usize] {
        self.successors.get(node)
    }

    pub(in crate::ir::analysis) fn predecessors(
        &self,
        node: usize,
    ) -> impl ExactSizeIterator<Item = usize> + '_ {
        self.predecessors
            .get(node)
            .iter()
            .map(|&(source, _)| source)
    }

    pub(super) fn successor_points(&self, point: SemanticFlowPoint) -> &[SemanticFlowPoint] {
        let Some(row) = self.id(point) else {
            return &[];
        };
        let start = self.successors.offsets[row];
        let end = self.successors.offsets[row + 1];
        if start == end {
            return &[];
        }
        let points = self.successor_points.get_or_init(|| {
            self.successors
                .items
                .iter()
                .map(|&node| self.point(node))
                .collect()
        });
        &points[start..end]
    }

    pub(super) fn incoming_points(
        &self,
        point: SemanticFlowPoint,
    ) -> impl Iterator<Item = (SemanticFlowPoint, SemanticFlowEdgeKind)> + '_ {
        self.id(point)
            .into_iter()
            .flat_map(|node| self.predecessors.get(node))
            .map(|&(node, kind)| (self.point(node), kind))
    }
}

/// Depth-first search of paths that avoid the target. A terminal or an edge
/// back to the active path disproves mandatory reachability. Consecutive
/// searches retain only scratch space indexed by this immutable graph.
#[derive(Debug, Default)]
struct MandatoryPaths {
    colors: Vec<u32>,
    stack: Vec<(usize, usize)>,
    generation: u32,
}

impl MandatoryPaths {
    fn search(&mut self, edges: &GraphIndex, source: usize, target: usize) -> bool {
        self.colors.resize(edges.points.len(), 0);
        if self.generation >= u32::MAX - 2 {
            self.colors.fill(0);
            self.generation = 0;
        }
        self.generation += 2;
        let active = self.generation;
        let complete = active + 1;
        self.stack.clear();
        self.stack.push((source, 0));
        self.colors[source] = active;
        while let Some((point, cursor)) = self.stack.last_mut() {
            let successors = edges.successors.get(*point);
            let Some(&next) = successors.get(*cursor) else {
                if successors.is_empty() {
                    return false;
                }
                self.colors[*point] = complete;
                self.stack.pop();
                continue;
            };
            *cursor += 1;
            if next == target || self.colors[next] == complete {
                continue;
            }
            if self.colors[next] == active {
                return false;
            }
            self.colors[next] = active;
            self.stack.push((next, 0));
        }
        true
    }
}
