//! Occurrence-sensitive control flow over structured Semantic IR.
//!
//! Every numbered semantic site owns a before/after program point. Structural
//! nodes connect those points without projecting cloned occurrences back onto
//! one original CFG block.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ops::Range;
use std::sync::{Arc, Mutex, OnceLock};

use crate::ir::{
    RegionId, SemanticExpression, SemanticLabel, SemanticLeave, SemanticLeaveKind,
    SemanticLoopControl, SemanticLoopKind, SemanticNode, SemanticPredicate, SemanticSiteId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticFlowPoint {
    Before(SemanticSiteId),
    After(SemanticSiteId),
    Synthetic(u64),
}

impl SemanticFlowPoint {
    pub fn before(site: SemanticSiteId) -> Self {
        Self::Before(site)
    }

    pub fn after(site: SemanticSiteId) -> Self {
        Self::After(site)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticFlowEdgeKind {
    Normal,
    Exceptional,
    LoopBack,
    FinallyEnter(u64),
    FinallyResume(u64),
    FinallyOverride,
}

#[derive(Debug, Clone, Default)]
pub struct SemanticFlowGraph {
    predecessors: BTreeMap<SemanticFlowPoint, Vec<(SemanticFlowPoint, SemanticFlowEdgeKind)>>,
    successors: BTreeMap<SemanticFlowPoint, Vec<SemanticFlowPoint>>,
    entries: BTreeSet<SemanticFlowPoint>,
    complete: bool,
    reachability: OnceLock<SemanticReachability>,
}

#[derive(Debug, Clone)]
pub struct SemanticReachability {
    components: BTreeMap<SemanticFlowPoint, usize>,
    store: ReachabilityStore,
}

#[derive(Debug, Clone)]
enum ReachabilityStore {
    Linear,
    Dense { row_words: usize, bits: Vec<u64> },
    Sparse(Arc<SparseReachability>),
}

#[derive(Debug)]
struct SparseReachability {
    successors: Vec<Vec<usize>>,
    predecessors: Vec<Vec<usize>>,
    forward: Mutex<ReachabilityCache>,
    backward: Mutex<ReachabilityCache>,
}

#[derive(Debug, Default)]
struct ReachabilityCache {
    order: VecDeque<usize>,
    rows: BTreeMap<usize, Arc<DefinitionSet>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DefinitionSet {
    Word(u64),
    Words(Box<[u64]>),
}

impl DefinitionSet {
    fn new(definitions: usize) -> Self {
        match definitions.div_ceil(u64::BITS as usize) {
            1 => Self::Word(0),
            words => Self::Words(vec![0; words].into_boxed_slice()),
        }
    }

    fn words(&self) -> &[u64] {
        match self {
            Self::Word(word) => std::slice::from_ref(word),
            Self::Words(words) => words,
        }
    }

    fn words_mut(&mut self) -> &mut [u64] {
        match self {
            Self::Word(word) => std::slice::from_mut(word),
            Self::Words(words) => words,
        }
    }

    fn insert(&mut self, definition: usize) {
        self.words_mut()[definition / u64::BITS as usize] |= 1 << (definition % u64::BITS as usize);
    }

    fn contains(&self, definition: usize) -> bool {
        self.words()
            .get(definition / u64::BITS as usize)
            .is_some_and(|word| word & (1 << (definition % u64::BITS as usize)) != 0)
    }

    fn union_with(&mut self, other: &Self) {
        for (word, other) in self.words_mut().iter_mut().zip(other.words()) {
            *word |= other;
        }
    }

    fn union_delta(&mut self, other: &Self, delta: &mut Self) -> bool {
        let mut changed = false;
        for ((word, other), delta) in self
            .words_mut()
            .iter_mut()
            .zip(other.words())
            .zip(delta.words_mut())
        {
            let added = *other & !*word;
            *word |= added;
            *delta |= added;
            changed |= added != 0;
        }
        changed
    }

    fn union_delta_without(&mut self, other: &Self, excluded: &Self, delta: &mut Self) -> bool {
        let mut changed = false;
        for (((word, other), excluded), delta) in self
            .words_mut()
            .iter_mut()
            .zip(other.words())
            .zip(excluded.words())
            .zip(delta.words_mut())
        {
            let added = (*other & !*excluded) & !*word;
            *word |= added;
            *delta |= added;
            changed |= added != 0;
        }
        changed
    }

    fn remove_with(&mut self, other: &Self) {
        for (word, other) in self.words_mut().iter_mut().zip(other.words()) {
            *word &= !other;
        }
    }

    fn clear(&mut self) {
        self.words_mut().fill(0);
    }

    fn mask_words(bits: Range<usize>) -> impl Iterator<Item = (usize, u64)> {
        let start = bits.start / 64;
        let end = bits.end.div_ceil(64);
        (start..end).map(move |index| {
            let first = bits.start.saturating_sub(index * 64);
            let last = (bits.end - index * 64).min(64);
            let mask = (u64::MAX << first) & (u64::MAX >> (64 - last));
            (index, mask)
        })
    }

    fn insert_range(&mut self, bits: Range<usize>) {
        let words = self.words_mut();
        for (index, mask) in Self::mask_words(bits) {
            words[index] |= mask;
        }
    }

    fn same_in_range(&self, other: &Self, bits: Range<usize>) -> bool {
        Self::mask_words(bits)
            .all(|(index, mask)| (self.words()[index] ^ other.words()[index]) & mask == 0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SemanticValueDefinition {
    point: SemanticFlowPoint,
    value: usize,
    ordinal: usize,
}

impl SemanticValueDefinition {
    pub fn new(point: SemanticFlowPoint, value: usize, ordinal: usize) -> Self {
        Self {
            point,
            value,
            ordinal,
        }
    }
}

// Number definitions of the same value consecutively. Its kill mask is a
// contiguous bit range, even when the caller supplied interleaved values.
#[derive(Debug, Clone, Default)]
struct DefinitionsByValue {
    offsets: Vec<usize>,
    ordinals: Vec<usize>,
}

impl DefinitionsByValue {
    fn new(definitions: &[SemanticValueDefinition], value_count: usize) -> Self {
        let mut offsets = vec![0usize; value_count + 1];
        for definition in definitions {
            offsets[definition.value + 1] += 1;
        }
        for value in 0..value_count {
            offsets[value + 1] += offsets[value];
        }
        Self {
            offsets,
            ordinals: vec![0; definitions.len()],
        }
    }

    fn range(&self, value: usize) -> Option<Range<usize>> {
        let &start = self.offsets.get(value)?;
        let &end = self.offsets.get(value.checked_add(1)?)?;
        Some(start..end)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SemanticReachingValues {
    reaching: BTreeMap<SemanticFlowPoint, DefinitionSet>,
    by_value: DefinitionsByValue,
}

impl SemanticReachingValues {
    pub fn at(&self, value: usize, point: SemanticFlowPoint) -> Option<Vec<usize>> {
        let reaching = self.reaching.get(&point)?;
        let range = self.by_value.range(value).unwrap_or(0..0);
        Some(
            range
                .filter(|index| reaching.contains(*index))
                .map(|index| self.by_value.ordinals[index])
                .collect(),
        )
    }

    pub fn unchanged(
        &self,
        value: usize,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
    ) -> Option<bool> {
        let source = self.reaching.get(&source)?;
        let target = self.reaching.get(&target)?;
        Some(source.same_in_range(target, self.by_value.range(value)?))
    }
}

impl SemanticReachability {
    fn analyze(graph: &SemanticFlowGraph) -> Self {
        const MAX_DENSE_BYTES: usize = 64 * 1024 * 1024;
        Self::analyze_with_dense_limit(graph, MAX_DENSE_BYTES)
    }

    fn analyze_with_dense_limit(graph: &SemanticFlowGraph, max_dense_bytes: usize) -> Self {
        if let Some(chain) = Self::linear_chain(graph) {
            return chain;
        }
        let points = graph.points();
        let mut visited = BTreeSet::new();
        let mut finish = Vec::with_capacity(points.len());
        for root in &points {
            if !visited.insert(*root) {
                continue;
            }
            let mut stack = vec![(*root, 0usize)];
            while let Some((point, successor_index)) = stack.last_mut() {
                let successors = graph
                    .successors
                    .get(point)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                if *successor_index < successors.len() {
                    let successor = successors[*successor_index];
                    *successor_index += 1;
                    if visited.insert(successor) {
                        stack.push((successor, 0));
                    }
                } else {
                    finish.push(*point);
                    stack.pop();
                }
            }
        }

        let mut components = BTreeMap::new();
        let mut component_count = 0usize;
        for root in finish.into_iter().rev() {
            if components.contains_key(&root) {
                continue;
            }
            let component = component_count;
            component_count += 1;
            components.insert(root, component);
            let mut stack = vec![root];
            while let Some(point) = stack.pop() {
                for (predecessor, _) in graph
                    .predecessors
                    .get(&point)
                    .into_iter()
                    .flat_map(Vec::as_slice)
                {
                    if !components.contains_key(predecessor) {
                        components.insert(*predecessor, component);
                        stack.push(*predecessor);
                    }
                }
            }
        }

        let mut successors = vec![BTreeSet::new(); component_count];
        // Kosaraju assigns each component before all its outgoing neighbors:
        // the reverse traversal starts at the greatest remaining finish time.
        for (point, point_successors) in &graph.successors {
            let source = components[point];
            for successor in point_successors {
                let target = components[successor];
                if source != target {
                    debug_assert!(source < target);
                    successors[source].insert(target);
                }
            }
        }

        let words_per_row = component_count.div_ceil(u64::BITS as usize);
        let dense_bytes = component_count
            .saturating_mul(words_per_row)
            .saturating_mul(std::mem::size_of::<u64>());
        let store = if dense_bytes <= max_dense_bytes {
            let mut bits = vec![0u64; component_count * words_per_row];
            for component in (0..component_count).rev() {
                let (earlier, later) = bits.split_at_mut((component + 1) * words_per_row);
                let row = &mut earlier[component * words_per_row..];
                row[component / 64] |= 1u64 << (component % 64);
                for &successor in &successors[component] {
                    let start = (successor - component - 1) * words_per_row;
                    for (word, other) in row.iter_mut().zip(&later[start..start + words_per_row]) {
                        *word |= *other;
                    }
                }
            }
            ReachabilityStore::Dense {
                row_words: words_per_row,
                bits,
            }
        } else {
            let successors = successors
                .into_iter()
                .map(|targets| targets.into_iter().collect::<Vec<_>>())
                .collect::<Vec<_>>();
            let mut predecessors = vec![Vec::new(); component_count];
            for (source, targets) in successors.iter().enumerate() {
                for target in targets {
                    predecessors[*target].push(source);
                }
            }
            ReachabilityStore::Sparse(Arc::new(SparseReachability {
                successors,
                predecessors,
                forward: Mutex::new(ReachabilityCache::default()),
                backward: Mutex::new(ReachabilityCache::default()),
            }))
        };
        Self { components, store }
    }

    // A straight-line method has one path. Its point order answers all
    // reachability queries without a component graph or a closure matrix.
    fn linear_chain(graph: &SemanticFlowGraph) -> Option<Self> {
        if graph.entries.len() != 1 || graph.successors.values().any(|targets| targets.len() != 1) {
            return None;
        }
        let mut point = *graph.entries.first()?;
        let mut components = BTreeMap::new();
        while components.insert(point, components.len()).is_none() {
            match graph.successors(point) {
                [next] => point = *next,
                [] if components.len() - 1 == graph.successors.len() => {
                    // Every edge, including any disconnected component's edge,
                    // contributes an entry to the single-successor map above.
                    return Some(Self {
                        components,
                        store: ReachabilityStore::Linear,
                    });
                }
                _ => return None,
            }
        }
        None
    }

    pub fn reaches(&self, source: SemanticFlowPoint, target: SemanticFlowPoint) -> bool {
        let (Some(source), Some(target)) =
            (self.components.get(&source), self.components.get(&target))
        else {
            return false;
        };
        match &self.store {
            ReachabilityStore::Linear => source <= target,
            ReachabilityStore::Dense { row_words, bits } => {
                bits[source * row_words + target / 64] & (1u64 << (target % 64)) != 0
            }
            ReachabilityStore::Sparse(reachable) => reachable.reaches(*source, *target),
        }
    }

    pub fn reaches_any_between(
        &self,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
        candidates: &BTreeSet<SemanticFlowPoint>,
    ) -> bool {
        match &self.store {
            ReachabilityStore::Dense { .. } | ReachabilityStore::Linear => {
                candidates.iter().any(|candidate| {
                    self.reaches(source, *candidate) && self.reaches(*candidate, target)
                })
            }
            ReachabilityStore::Sparse(reachable) => {
                let (Some(source), Some(target)) =
                    (self.components.get(&source), self.components.get(&target))
                else {
                    return false;
                };
                let from_source = reachable.forward_row(*source);
                let to_target = reachable.backward_row(*target);
                candidates.iter().any(|candidate| {
                    self.components.get(candidate).is_some_and(|candidate| {
                        from_source.contains(*candidate) && to_target.contains(*candidate)
                    })
                })
            }
        }
    }
}

impl SparseReachability {
    const MAX_CACHED_ROWS: usize = 32;

    fn reaches(&self, source: usize, target: usize) -> bool {
        if source == target {
            return true;
        }
        if source >= target {
            return false;
        }
        self.forward_row(source).contains(target)
    }

    fn forward_row(&self, source: usize) -> Arc<DefinitionSet> {
        Self::row(&self.forward, &self.successors, source)
    }

    fn backward_row(&self, target: usize) -> Arc<DefinitionSet> {
        Self::row(&self.backward, &self.predecessors, target)
    }

    fn row(
        cache: &Mutex<ReachabilityCache>,
        adjacency: &[Vec<usize>],
        source: usize,
    ) -> Arc<DefinitionSet> {
        if let Some(row) = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .rows
            .get(&source)
            .cloned()
        {
            return row;
        }

        let mut row = DefinitionSet::new(adjacency.len());
        let mut pending = vec![source];
        while let Some(component) = pending.pop() {
            if row.contains(component) {
                continue;
            }
            row.insert(component);
            pending.extend(adjacency[component].iter().copied());
        }
        let row = Arc::new(row);

        let mut cache = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = cache.rows.get(&source) {
            return existing.clone();
        }
        while cache.rows.len() >= Self::MAX_CACHED_ROWS {
            let Some(evicted) = cache.order.pop_front() else {
                break;
            };
            cache.rows.remove(&evicted);
        }
        cache.order.push_back(source);
        cache.rows.insert(source, row.clone());
        row
    }
}

impl SemanticFlowGraph {
    pub fn analyze(root: &SemanticNode) -> Self {
        SemanticFlowBuilder::new().build(root)
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn points(&self) -> BTreeSet<SemanticFlowPoint> {
        self.predecessors
            .keys()
            .chain(self.successors.keys())
            .chain(self.successors.values().flatten())
            .chain(
                self.predecessors
                    .values()
                    .flatten()
                    .map(|(predecessor, _)| predecessor),
            )
            .chain(&self.entries)
            .copied()
            .collect()
    }

    pub fn successors(&self, point: SemanticFlowPoint) -> &[SemanticFlowPoint] {
        self.successors
            .get(&point)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn predecessors(
        &self,
        point: SemanticFlowPoint,
    ) -> impl Iterator<Item = SemanticFlowPoint> + '_ {
        self.predecessors
            .get(&point)
            .into_iter()
            .flatten()
            .map(|(predecessor, _)| *predecessor)
    }

    pub fn reachability(&self) -> SemanticReachability {
        SemanticReachability::analyze(self)
    }

    pub(crate) fn indexed_reachability(&self) -> &SemanticReachability {
        self.reachability.get_or_init(|| self.reachability())
    }

    /// Returns whether at least one path connects `source` to `target`.
    pub fn reaches(&self, source: SemanticFlowPoint, target: SemanticFlowPoint) -> bool {
        if source == target {
            return true;
        }
        let mut visited = BTreeSet::new();
        let mut pending = vec![source];
        while let Some(point) = pending.pop() {
            if !visited.insert(point) {
                continue;
            }
            let Some(successors) = self.successors.get(&point) else {
                continue;
            };
            if successors.contains(&target) {
                return true;
            }
            pending.extend(successors.iter().copied());
        }
        false
    }

    /// Proves that every maximal path leaving `source` reaches `target`.
    ///
    /// A terminal path or a cycle that avoids `target` rejects the proof. This
    /// is intentionally stronger than ordinary reachability because sinking a
    /// throwing or observable expression must preserve execution count.
    pub fn must_reach(&self, source: SemanticFlowPoint, target: SemanticFlowPoint) -> bool {
        if source == target {
            return true;
        }
        let mut colors = BTreeMap::<SemanticFlowPoint, VisitColor>::new();
        let mut pending = vec![(source, false)];
        while let Some((point, leaving)) = pending.pop() {
            if point == target {
                continue;
            }
            if leaving {
                colors.insert(point, VisitColor::Complete);
                continue;
            }
            match colors.get(&point) {
                Some(VisitColor::Active) => return false,
                Some(VisitColor::Complete) => continue,
                None => {}
            }
            let Some(successors) = self.successors.get(&point) else {
                return false;
            };
            if successors.is_empty() {
                return false;
            }
            colors.insert(point, VisitColor::Active);
            pending.push((point, true));
            pending.extend(
                successors
                    .iter()
                    .rev()
                    .copied()
                    .map(|successor| (successor, false)),
            );
        }
        true
    }

    pub fn reaching_values(
        &self,
        definitions: &[SemanticValueDefinition],
        value_count: usize,
        targets: &BTreeSet<SemanticFlowPoint>,
    ) -> SemanticReachingValues {
        let definition_count = definitions.len();
        let mut by_value = DefinitionsByValue::new(definitions, value_count);
        let mut next = by_value.offsets[..value_count].to_vec();
        let mut generated = BTreeMap::<SemanticFlowPoint, DefinitionSet>::new();
        let mut killed = BTreeMap::<SemanticFlowPoint, DefinitionSet>::new();
        for definition in definitions {
            let value = definition.value;
            let index = next[value];
            next[value] += 1;
            by_value.ordinals[index] = definition.ordinal;
            generated
                .entry(definition.point)
                .or_insert_with(|| DefinitionSet::new(definition_count))
                .insert(index);
            killed
                .entry(definition.point)
                .or_insert_with(|| DefinitionSet::new(definition_count))
                .insert_range(by_value.offsets[value]..by_value.offsets[value + 1]);
        }
        if !self.has_cleanup_continuations() {
            return self.reaching_values_plain(definitions, targets, generated, killed, by_value);
        }
        self.reaching_values_with_cleanup(definitions, targets, generated, killed, by_value)
    }

    fn has_cleanup_continuations(&self) -> bool {
        self.predecessors.values().flatten().any(|(_, edge)| {
            matches!(
                edge,
                SemanticFlowEdgeKind::FinallyEnter(_)
                    | SemanticFlowEdgeKind::FinallyResume(_)
                    | SemanticFlowEdgeKind::FinallyOverride
            )
        })
    }

    fn reaching_values_plain(
        &self,
        definitions: &[SemanticValueDefinition],
        targets: &BTreeSet<SemanticFlowPoint>,
        generated: BTreeMap<SemanticFlowPoint, DefinitionSet>,
        killed: BTreeMap<SemanticFlowPoint, DefinitionSet>,
        by_value: DefinitionsByValue,
    ) -> SemanticReachingValues {
        let definition_count = definitions.len();
        let nodes = self.reaching_slice(targets);
        let points = nodes.iter().copied().collect::<Vec<_>>();
        let ids = points
            .iter()
            .copied()
            .enumerate()
            .map(|(index, point)| (point, index))
            .collect::<BTreeMap<_, _>>();
        let mut successors = vec![Vec::new(); points.len()];
        for (index, point) in points.iter().enumerate() {
            for successor in self
                .successors
                .get(point)
                .into_iter()
                .flat_map(Vec::as_slice)
            {
                if let Some(&successor) = ids.get(successor) {
                    successors[index].push(successor);
                }
            }
        }
        let entries = points
            .iter()
            .map(|point| self.entries.contains(point))
            .collect::<Vec<_>>();
        let mut facts = vec![DefinitionSet::new(definition_count); points.len()];
        let mut deltas = vec![DefinitionSet::new(definition_count); points.len()];
        let mut reachable = vec![false; points.len()];
        let mut pending = VecDeque::new();
        let mut queued = vec![false; points.len()];
        for (node, entry) in entries.iter().copied().enumerate() {
            if !entry {
                continue;
            }
            reachable[node] = true;
            if let Some(generated) = generated.get(&points[node]) {
                facts[node].union_delta(generated, &mut deltas[node]);
            }
            queued[node] = true;
            pending.push_back(node);
        }
        let mut delta = DefinitionSet::new(definition_count);
        while let Some(node) = pending.pop_front() {
            queued[node] = false;
            std::mem::swap(&mut delta, &mut deltas[node]);
            for successor in &successors[node] {
                let first_reach = !reachable[*successor];
                if first_reach {
                    reachable[*successor] = true;
                    if let Some(generated) = generated.get(&points[*successor]) {
                        facts[*successor].union_delta(generated, &mut deltas[*successor]);
                    }
                }
                let changed = match killed.get(&points[*successor]) {
                    Some(killed) => facts[*successor].union_delta_without(
                        &delta,
                        killed,
                        &mut deltas[*successor],
                    ),
                    None => facts[*successor].union_delta(&delta, &mut deltas[*successor]),
                };
                if (first_reach || changed) && !queued[*successor] {
                    queued[*successor] = true;
                    pending.push_back(*successor);
                }
            }
            delta.clear();
        }
        let mut reaching = points
            .into_iter()
            .enumerate()
            .filter(|&(index, point)| reachable[index] && targets.contains(&point))
            .map(|(index, point)| (point, facts[index].clone()))
            .collect::<BTreeMap<_, _>>();
        self.insert_unreachable_target_bottoms(&mut reaching, targets, definition_count);
        SemanticReachingValues { reaching, by_value }
    }

    fn reaching_values_with_cleanup(
        &self,
        definitions: &[SemanticValueDefinition],
        targets: &BTreeSet<SemanticFlowPoint>,
        generated: BTreeMap<SemanticFlowPoint, DefinitionSet>,
        killed: BTreeMap<SemanticFlowPoint, DefinitionSet>,
        by_value: DefinitionsByValue,
    ) -> SemanticReachingValues {
        let definition_count = definitions.len();
        let nodes = self.reaching_slice(targets);
        let mut outgoing =
            BTreeMap::<SemanticFlowPoint, Vec<(SemanticFlowPoint, SemanticFlowEdgeKind)>>::new();
        for (target, predecessors) in &self.predecessors {
            if !nodes.contains(target) {
                continue;
            }
            for (predecessor, edge) in predecessors {
                if nodes.contains(predecessor) {
                    outgoing
                        .entry(*predecessor)
                        .or_default()
                        .push((*target, *edge));
                }
            }
        }

        let mut states = Vec::<(SemanticFlowPoint, Vec<u64>)>::new();
        let mut state_ids = BTreeMap::<(SemanticFlowPoint, Vec<u64>), usize>::new();
        let mut successors = Vec::<Vec<usize>>::new();
        let mut predecessors = Vec::<Vec<usize>>::new();
        let mut entries = Vec::<bool>::new();
        let mut pending = VecDeque::new();
        for entry in self.entries.iter().filter(|entry| nodes.contains(entry)) {
            let id = states.len();
            let state = (*entry, Vec::new());
            if state_ids.insert(state.clone(), id).is_none() {
                states.push(state);
                successors.push(Vec::new());
                predecessors.push(Vec::new());
                entries.push(true);
                pending.push_back(id);
            }
        }
        while let Some(state_id) = pending.pop_front() {
            let (point, continuation) = states[state_id].clone();
            for (successor, edge) in outgoing.get(&point).into_iter().flat_map(Vec::as_slice) {
                let Some(next_continuation) = edge.transfer(&continuation) else {
                    continue;
                };
                let next_state = (*successor, next_continuation);
                let next_id = if let Some(id) = state_ids.get(&next_state) {
                    *id
                } else {
                    let id = states.len();
                    state_ids.insert(next_state.clone(), id);
                    states.push(next_state);
                    successors.push(Vec::new());
                    predecessors.push(Vec::new());
                    entries.push(false);
                    pending.push_back(id);
                    id
                };
                if !successors[state_id].contains(&next_id) {
                    successors[state_id].push(next_id);
                    predecessors[next_id].push(state_id);
                }
            }
        }

        let mut facts = vec![DefinitionSet::new(definition_count); states.len()];
        let mut initialized = vec![false; states.len()];
        let mut pending = (0..states.len()).collect::<VecDeque<_>>();
        let mut queued = vec![true; states.len()];
        while let Some(state_id) = pending.pop_front() {
            queued[state_id] = false;
            let mut next = DefinitionSet::new(definition_count);
            let mut reachable = entries[state_id];
            for predecessor in &predecessors[state_id] {
                if initialized[*predecessor] {
                    reachable = true;
                    next.union_with(&facts[*predecessor]);
                }
            }
            if !reachable {
                continue;
            }
            let point = states[state_id].0;
            if let Some(generated_at_point) = generated.get(&point) {
                next.remove_with(&killed[&point]);
                next.union_with(generated_at_point);
            }
            if initialized[state_id] && facts[state_id] == next {
                continue;
            }
            facts[state_id] = next;
            initialized[state_id] = true;
            for successor in &successors[state_id] {
                if !queued[*successor] {
                    queued[*successor] = true;
                    pending.push_back(*successor);
                }
            }
        }

        let mut reaching = BTreeMap::<SemanticFlowPoint, DefinitionSet>::new();
        for (state_id, (point, _)) in states.into_iter().enumerate() {
            if initialized[state_id] && targets.contains(&point) {
                reaching
                    .entry(point)
                    .or_insert_with(|| DefinitionSet::new(definition_count))
                    .union_with(&facts[state_id]);
            }
        }
        self.insert_unreachable_target_bottoms(&mut reaching, targets, definition_count);
        SemanticReachingValues { reaching, by_value }
    }

    fn insert_unreachable_target_bottoms(
        &self,
        reaching: &mut BTreeMap<SemanticFlowPoint, DefinitionSet>,
        targets: &BTreeSet<SemanticFlowPoint>,
        definition_count: usize,
    ) {
        for target in targets.iter().copied().filter(|target| {
            self.entries.contains(target)
                || self.predecessors.contains_key(target)
                || self.successors.contains_key(target)
        }) {
            reaching
                .entry(target)
                .or_insert_with(|| DefinitionSet::new(definition_count));
        }
    }

    fn reaching_slice(&self, targets: &BTreeSet<SemanticFlowPoint>) -> BTreeSet<SemanticFlowPoint> {
        let mut nodes = targets.clone();
        let mut ancestors = targets.iter().copied().collect::<VecDeque<_>>();
        while let Some(node) = ancestors.pop_front() {
            for (predecessor, _) in self
                .predecessors
                .get(&node)
                .into_iter()
                .flat_map(Vec::as_slice)
            {
                if nodes.insert(*predecessor) {
                    ancestors.push_back(*predecessor);
                }
            }
        }
        nodes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitColor {
    Active,
    Complete,
}

impl SemanticFlowEdgeKind {
    fn transfer(self, continuation: &[u64]) -> Option<Vec<u64>> {
        let mut result = continuation.to_vec();
        match self {
            Self::Normal | Self::Exceptional | Self::LoopBack => {}
            Self::FinallyEnter(token) => {
                if result.contains(&token) {
                    return None;
                }
                result.push(token);
            }
            Self::FinallyResume(token) => {
                if result.pop() != Some(token) {
                    return None;
                }
            }
            Self::FinallyOverride => {
                result.pop()?;
            }
        }
        Some(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Completion {
    Return,
    Throw,
    Break(RegionId),
    Continue(RegionId),
    BreakLabel(SemanticLabel),
    ContinueLabel(SemanticLabel),
    Open,
}

enum FinallyContinuation {
    Normal,
    Abrupt(Completion),
    Throwing,
}

#[derive(Default)]
struct FlowFragment {
    entry: Option<SemanticFlowPoint>,
    normal: bool,
    exits: BTreeSet<SemanticFlowPoint>,
    abrupt: BTreeMap<Completion, BTreeSet<SemanticFlowPoint>>,
    throwing: BTreeSet<SemanticFlowPoint>,
}

impl FlowFragment {
    fn identity() -> Self {
        Self {
            normal: true,
            ..Self::default()
        }
    }

    fn is_identity(&self) -> bool {
        self.entry.is_none()
            && self.normal
            && self.exits.is_empty()
            && self.abrupt.is_empty()
            && self.throwing.is_empty()
    }

    fn site(site: SemanticSiteId, may_throw: bool) -> Self {
        let before = SemanticFlowPoint::before(site);
        let after = SemanticFlowPoint::after(site);
        Self {
            entry: Some(before),
            normal: true,
            exits: BTreeSet::from([after]),
            abrupt: BTreeMap::new(),
            throwing: if may_throw {
                BTreeSet::from([before])
            } else {
                Default::default()
            },
        }
    }

    fn add_abrupt(&mut self, completion: Completion, point: SemanticFlowPoint) {
        self.abrupt.entry(completion).or_default().insert(point);
    }

    fn merge_abrupt(&mut self, other: BTreeMap<Completion, BTreeSet<SemanticFlowPoint>>) {
        for (completion, points) in other {
            self.abrupt.entry(completion).or_default().extend(points);
        }
    }

    fn take_abrupt(&mut self, completion: &Completion) -> BTreeSet<SemanticFlowPoint> {
        self.abrupt.remove(completion).unwrap_or_default()
    }
}

enum BuildTask<'a> {
    Visit(&'a SemanticNode),
    Combine(BuildFrame<'a>),
}

enum BuildFrame<'a> {
    Sequence(usize),
    If {
        condition: &'a crate::ir::SemanticOperand<SemanticPredicate>,
        has_else: bool,
    },
    Loop {
        control: SemanticLoopControl,
        kind: SemanticLoopKind,
        condition: &'a crate::ir::SemanticOperand<SemanticPredicate>,
    },
    For {
        control: SemanticLoopControl,
        init: &'a crate::ir::SemanticStatement,
        condition: &'a crate::ir::SemanticOperand<SemanticPredicate>,
        update: &'a crate::ir::SemanticStatement,
    },
    ForEach {
        control: SemanticLoopControl,
        iterable: &'a crate::ir::SemanticOperand<SemanticExpression>,
    },
    Switch {
        region: Option<RegionId>,
        selector: &'a crate::ir::SemanticOperand<SemanticExpression>,
        cases: usize,
        has_default: bool,
    },
    Try {
        catches: usize,
        has_finally: bool,
    },
    Synchronized {
        lock: &'a crate::ir::SemanticOperand<SemanticExpression>,
    },
    Label(SemanticLabel),
}

impl BuildFrame<'_> {
    fn child_count(&self) -> usize {
        match self {
            Self::Sequence(count) => *count,
            Self::If { has_else, .. } => 1 + usize::from(*has_else),
            Self::Loop { .. } => 2,
            Self::For { .. }
            | Self::ForEach { .. }
            | Self::Synchronized { .. }
            | Self::Label(_) => 1,
            Self::Switch { cases, .. } => *cases,
            Self::Try {
                catches,
                has_finally,
            } => 1 + catches + usize::from(*has_finally),
        }
    }
}

struct SemanticFlowBuilder {
    graph: SemanticFlowGraph,
    next_synthetic: u64,
    next_continuation: u64,
}

impl SemanticFlowBuilder {
    fn new() -> Self {
        Self {
            graph: SemanticFlowGraph {
                complete: true,
                ..SemanticFlowGraph::default()
            },
            next_synthetic: 0,
            next_continuation: 0,
        }
    }

    fn mark_incomplete(&mut self, _reason: &'static str) {
        self.graph.complete = false;
    }

    fn build(mut self, root: &SemanticNode) -> SemanticFlowGraph {
        let mut pending = vec![BuildTask::Visit(root)];
        let mut results = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                BuildTask::Visit(node) => self.schedule(node, &mut pending, &mut results),
                BuildTask::Combine(frame) => {
                    let count = frame.child_count();
                    if results.len() < count {
                        self.mark_incomplete("semantic flow combine stack underflow");
                        continue;
                    }
                    let children = results.split_off(results.len() - count);
                    results.push(self.combine(frame, children));
                }
            }
        }
        if let Some(result) = results.pop() {
            self.graph.entries.extend(result.entry);
        }
        self.graph
    }

    fn schedule<'a>(
        &mut self,
        node: &'a SemanticNode,
        pending: &mut Vec<BuildTask<'a>>,
        results: &mut Vec<FlowFragment>,
    ) {
        match node {
            SemanticNode::Empty => results.push(self.empty_node()),
            SemanticNode::BasicBlock(block) => {
                let fragments = block
                    .statements
                    .iter()
                    .map(|statement| self.site(statement.site, statement.effects().may_throw()))
                    .collect();
                results.push(self.sequence(fragments));
            }
            SemanticNode::Sequence(children) => {
                pending.push(BuildTask::Combine(BuildFrame::Sequence(children.len())));
                pending.extend(children.iter().rev().map(BuildTask::Visit));
            }
            SemanticNode::If {
                condition,
                then_node,
                else_node,
            } => {
                pending.push(BuildTask::Combine(BuildFrame::If {
                    condition,
                    has_else: else_node.is_some(),
                }));
                if let Some(else_node) = else_node {
                    pending.push(BuildTask::Visit(else_node));
                }
                pending.push(BuildTask::Visit(then_node));
            }
            SemanticNode::Loop {
                control,
                kind,
                test,
                body,
                ..
            } => {
                pending.push(BuildTask::Combine(BuildFrame::Loop {
                    control: *control,
                    kind: *kind,
                    condition: &test.condition,
                }));
                pending.push(BuildTask::Visit(body));
                pending.push(BuildTask::Visit(&test.setup));
            }
            SemanticNode::For {
                control,
                init,
                condition,
                update,
                body,
            } => {
                pending.push(BuildTask::Combine(BuildFrame::For {
                    control: *control,
                    init,
                    condition,
                    update,
                }));
                pending.push(BuildTask::Visit(body));
            }
            SemanticNode::ForEach {
                control,
                iterable,
                body,
                ..
            } => {
                pending.push(BuildTask::Combine(BuildFrame::ForEach {
                    control: *control,
                    iterable,
                }));
                pending.push(BuildTask::Visit(body));
            }
            SemanticNode::Switch {
                region,
                selector,
                cases,
            } => {
                pending.push(BuildTask::Combine(BuildFrame::Switch {
                    region: *region,
                    selector,
                    cases: cases.len(),
                    has_default: cases.iter().any(|case| case.is_default),
                }));
                pending.extend(cases.iter().rev().map(|case| BuildTask::Visit(&case.body)));
            }
            SemanticNode::Try {
                body,
                catches,
                finally,
                ..
            } => {
                pending.push(BuildTask::Combine(BuildFrame::Try {
                    catches: catches.len(),
                    has_finally: finally.is_some(),
                }));
                if let Some(finally) = finally {
                    pending.push(BuildTask::Visit(&finally.body));
                }
                pending.extend(
                    catches
                        .iter()
                        .rev()
                        .map(|catch| BuildTask::Visit(&catch.body)),
                );
                pending.push(BuildTask::Visit(body));
            }
            SemanticNode::Synchronized { lock, body, .. } => {
                pending.push(BuildTask::Combine(BuildFrame::Synchronized { lock }));
                pending.push(BuildTask::Visit(body));
            }
            SemanticNode::Label { label, body } => {
                pending.push(BuildTask::Combine(BuildFrame::Label(*label)));
                pending.push(BuildTask::Visit(body));
            }
            SemanticNode::Leave(leave) => results.push(self.leave(leave)),
        }
    }

    fn combine(&mut self, frame: BuildFrame<'_>, mut children: Vec<FlowFragment>) -> FlowFragment {
        match frame {
            BuildFrame::Sequence(_) => self.sequence(children),
            BuildFrame::If {
                condition,
                has_else,
            } => {
                let then_fragment = children.remove(0);
                let else_fragment = has_else.then(|| children.remove(0));
                self.branch(condition, then_fragment, else_fragment)
            }
            BuildFrame::Loop {
                control,
                kind,
                condition,
            } => {
                let setup = children.remove(0);
                let body = children.remove(0);
                self.loop_node(control, kind, condition, setup, body)
            }
            BuildFrame::For {
                control,
                init,
                condition,
                update,
            } => self.for_node(control, init, condition, update, children.remove(0)),
            BuildFrame::ForEach { control, iterable } => {
                self.for_each_node(control, iterable, children.remove(0))
            }
            BuildFrame::Switch {
                region,
                selector,
                cases,
                has_default,
            } => self.switch_node(
                region,
                selector,
                children.drain(..cases).collect(),
                has_default,
            ),
            BuildFrame::Try {
                catches,
                has_finally,
            } => self.try_node(catches, has_finally, children),
            BuildFrame::Synchronized { lock } => {
                let lock = self.site(lock.site, expression_may_throw(&lock.value));
                self.sequence(vec![lock, children.remove(0)])
            }
            BuildFrame::Label(label) => self.label(label, children.remove(0)),
        }
    }

    fn site(&mut self, site: Option<SemanticSiteId>, may_throw: bool) -> FlowFragment {
        let Some(site) = site else {
            self.mark_incomplete("semantic node has no numbered site");
            return FlowFragment::identity();
        };
        let fragment = FlowFragment::site(site, may_throw);
        self.add_edge(
            SemanticFlowPoint::before(site),
            SemanticFlowPoint::after(site),
            SemanticFlowEdgeKind::Normal,
        );
        fragment
    }

    fn leave(&mut self, leave: &SemanticLeave) -> FlowFragment {
        let mut fragment = self.site(
            leave.site,
            leave.value().is_some_and(expression_may_throw)
                || leave.condition.as_ref().is_some_and(predicate_may_throw),
        );
        let Some(point) = fragment.exits.iter().next().copied() else {
            self.mark_incomplete("semantic leave has no flow point");
            return fragment;
        };
        fragment.normal = false;
        fragment.exits.clear();
        fragment.add_abrupt(
            match &leave.kind {
                SemanticLeaveKind::Return(_) => Completion::Return,
                SemanticLeaveKind::Throw(_) => Completion::Throw,
                SemanticLeaveKind::Break => Completion::Break(leave.target),
                SemanticLeaveKind::Continue => Completion::Continue(leave.target),
                SemanticLeaveKind::BreakLabel(label) => Completion::BreakLabel(*label),
                SemanticLeaveKind::ContinueLabel(label) => Completion::ContinueLabel(*label),
                SemanticLeaveKind::FallThrough(_) | SemanticLeaveKind::Jump(_) => {
                    self.mark_incomplete("semantic tree contains an open transfer");
                    Completion::Open
                }
            },
            point,
        );
        fragment
    }

    fn sequence(&mut self, fragments: Vec<FlowFragment>) -> FlowFragment {
        let mut result = FlowFragment::identity();
        for fragment in fragments {
            if !result.normal {
                break;
            }
            if result.entry.is_none() {
                result.entry = fragment.entry;
            }
            if let Some(entry) = fragment.entry {
                self.connect(&result.exits, entry, SemanticFlowEdgeKind::Normal);
                result.exits = fragment.exits;
            }
            result.normal = fragment.normal;
            result.merge_abrupt(fragment.abrupt);
            result.throwing.extend(fragment.throwing);
        }
        result
    }

    fn branch(
        &mut self,
        condition: &crate::ir::SemanticOperand<SemanticPredicate>,
        then_fragment: FlowFragment,
        else_fragment: Option<FlowFragment>,
    ) -> FlowFragment {
        let mut condition_fragment =
            self.site(condition.site, predicate_may_throw(&condition.value));
        let sources = condition_fragment.exits.clone();
        let mut result = FlowFragment {
            entry: condition_fragment.entry,
            normal: false,
            exits: BTreeSet::new(),
            abrupt: std::mem::take(&mut condition_fragment.abrupt),
            throwing: condition_fragment.throwing,
        };
        self.add_alternative(&sources, then_fragment, &mut result);
        match else_fragment {
            Some(fragment) => self.add_alternative(&sources, fragment, &mut result),
            None => {
                result.normal = true;
                result.exits.extend(sources);
            }
        }
        result
    }

    fn loop_node(
        &mut self,
        control: SemanticLoopControl,
        kind: SemanticLoopKind,
        condition: &crate::ir::SemanticOperand<SemanticPredicate>,
        setup: FlowFragment,
        mut body: FlowFragment,
    ) -> FlowFragment {
        let condition_fragment = self.site(condition.site, predicate_may_throw(&condition.value));
        let break_points = body.take_abrupt(&break_completion(control));
        let continue_points = body.take_abrupt(&continue_completion(control));
        let condition_true = condition.value.constant_value() == Some(true);
        let condition_false = condition.value.constant_value() == Some(false);
        let mut result = FlowFragment::identity();

        match kind {
            SemanticLoopKind::PreTested => {
                let header = self.sequence(vec![setup, condition_fragment]);
                if !condition_false {
                    self.connect_to_fragment(&header.exits, &body, SemanticFlowEdgeKind::Normal);
                }
                result.entry = header.entry;
                result.throwing.extend(header.throwing);
                result.throwing.extend(body.throwing);
                result.merge_abrupt(header.abrupt);
                result.merge_abrupt(body.abrupt);
                let mut back = body.exits;
                back.extend(continue_points);
                if let Some(entry) = header.entry {
                    self.connect(&back, entry, SemanticFlowEdgeKind::LoopBack);
                }
                result.exits.extend(break_points);
                if !condition_true {
                    result.exits.extend(header.exits);
                }
            }
            SemanticLoopKind::PostTested => {
                let test = self.sequence(vec![setup, condition_fragment]);
                result.entry = body.entry.or(test.entry);
                result.throwing.extend(body.throwing);
                result.throwing.extend(test.throwing);
                result.merge_abrupt(body.abrupt);
                result.merge_abrupt(test.abrupt);
                let mut reaches_test = body.exits;
                reaches_test.extend(continue_points);
                if let Some(entry) = test.entry {
                    self.connect(&reaches_test, entry, SemanticFlowEdgeKind::Normal);
                }
                if !condition_false {
                    if let Some(entry) = result.entry {
                        self.connect(&test.exits, entry, SemanticFlowEdgeKind::LoopBack);
                    }
                }
                result.exits.extend(break_points);
                if !condition_true {
                    result.exits.extend(test.exits);
                }
            }
            SemanticLoopKind::Endless => {
                let iteration = self.sequence(vec![setup, body]);
                result.entry = iteration.entry;
                result.throwing = iteration.throwing;
                result.abrupt = iteration.abrupt;
                let mut back = iteration.exits;
                back.extend(continue_points);
                if let Some(entry) = iteration.entry {
                    self.connect(&back, entry, SemanticFlowEdgeKind::LoopBack);
                }
                result.exits = break_points;
            }
        }
        result.normal = !result.exits.is_empty();
        result
    }

    fn for_node(
        &mut self,
        control: SemanticLoopControl,
        init: &crate::ir::SemanticStatement,
        condition: &crate::ir::SemanticOperand<SemanticPredicate>,
        update: &crate::ir::SemanticStatement,
        mut body: FlowFragment,
    ) -> FlowFragment {
        let init = self.site(init.site, init.effects().may_throw());
        let condition_fragment = self.site(condition.site, predicate_may_throw(&condition.value));
        let update = self.site(update.site, update.effects().may_throw());
        let break_points = body.take_abrupt(&break_completion(control));
        let continue_points = body.take_abrupt(&continue_completion(control));
        let prefix = self.sequence(vec![init, condition_fragment]);
        self.connect_to_fragment(&prefix.exits, &body, SemanticFlowEdgeKind::Normal);
        let mut reaches_update = body.exits.clone();
        reaches_update.extend(continue_points);
        self.connect_to_fragment(&reaches_update, &update, SemanticFlowEdgeKind::Normal);
        if let Some(condition_entry) = prefix
            .entry
            .and_then(|_| condition.site.map(SemanticFlowPoint::before))
        {
            self.connect(
                &update.exits,
                condition_entry,
                SemanticFlowEdgeKind::LoopBack,
            );
        }
        let mut result = FlowFragment {
            entry: prefix.entry,
            normal: condition.value.constant_value() != Some(true) || !break_points.is_empty(),
            exits: break_points,
            abrupt: prefix.abrupt,
            throwing: prefix.throwing,
        };
        if condition.value.constant_value() != Some(true) {
            result.exits.extend(prefix.exits);
        }
        result.merge_abrupt(body.abrupt);
        result.merge_abrupt(update.abrupt);
        result.throwing.extend(body.throwing);
        result.throwing.extend(update.throwing);
        result
    }

    fn for_each_node(
        &mut self,
        control: SemanticLoopControl,
        iterable: &crate::ir::SemanticOperand<SemanticExpression>,
        mut body: FlowFragment,
    ) -> FlowFragment {
        let iterable_fragment = self.site(iterable.site, expression_may_throw(&iterable.value));
        let break_points = body.take_abrupt(&break_completion(control));
        let continue_points = body.take_abrupt(&continue_completion(control));
        self.connect_to_fragment(
            &iterable_fragment.exits,
            &body,
            SemanticFlowEdgeKind::Normal,
        );
        let mut back = body.exits.clone();
        back.extend(continue_points);
        for point in &iterable_fragment.exits {
            self.connect(&back, *point, SemanticFlowEdgeKind::LoopBack);
        }
        let mut result = FlowFragment {
            entry: iterable_fragment.entry,
            normal: true,
            exits: iterable_fragment.exits,
            abrupt: iterable_fragment.abrupt,
            throwing: iterable_fragment.throwing,
        };
        result.exits.extend(break_points);
        result.merge_abrupt(body.abrupt);
        result.throwing.extend(body.throwing);
        result
    }

    fn switch_node(
        &mut self,
        region: Option<RegionId>,
        selector: &crate::ir::SemanticOperand<SemanticExpression>,
        mut cases: Vec<FlowFragment>,
        has_default: bool,
    ) -> FlowFragment {
        let selector = self.site(selector.site, expression_may_throw(&selector.value));
        let mut result = FlowFragment {
            entry: selector.entry,
            normal: !has_default,
            exits: if has_default {
                BTreeSet::new()
            } else {
                selector.exits.clone()
            },
            abrupt: selector.abrupt,
            throwing: selector.throwing,
        };
        for mut case in cases.drain(..) {
            if let Some(region) = region {
                let breaks = case.take_abrupt(&Completion::Break(region));
                if !breaks.is_empty() {
                    case.normal = true;
                    case.exits.extend(breaks);
                }
            }
            self.add_alternative(&selector.exits, case, &mut result);
        }
        result
    }

    fn try_node(
        &mut self,
        catches: usize,
        has_finally: bool,
        mut children: Vec<FlowFragment>,
    ) -> FlowFragment {
        let body = children.remove(0);
        let mut catch_fragments = children.drain(..catches).collect::<Vec<_>>();
        for catch in &catch_fragments {
            if let Some(entry) = catch.entry {
                self.connect(&body.throwing, entry, SemanticFlowEdgeKind::Exceptional);
                if let Some(throws) = body.abrupt.get(&Completion::Throw) {
                    self.connect(throws, entry, SemanticFlowEdgeKind::Exceptional);
                }
            }
        }
        let mut result = FlowFragment {
            entry: body.entry,
            normal: body.normal,
            exits: body.exits,
            abrupt: body.abrupt,
            throwing: body.throwing,
        };
        for catch in catch_fragments.drain(..) {
            result.normal |= catch.normal;
            result.exits.extend(catch.exits);
            result.merge_abrupt(catch.abrupt);
            result.throwing.extend(catch.throwing);
        }
        if !has_finally {
            return result;
        }

        let finally = children.remove(0);
        let Some(finally_entry) = finally.entry else {
            if finally.is_identity() {
                return result;
            }
            self.mark_incomplete("finally body has no semantic entry");
            return result;
        };
        let mut output = FlowFragment {
            entry: result.entry,
            normal: false,
            exits: BTreeSet::new(),
            abrupt: BTreeMap::new(),
            throwing: BTreeSet::new(),
        };

        if result.normal {
            self.resume_after_finally(
                &result.exits,
                finally_entry,
                &finally,
                FinallyContinuation::Normal,
                &mut output,
            );
        }
        for (completion, sources) in result.abrupt {
            self.resume_after_finally(
                &sources,
                finally_entry,
                &finally,
                FinallyContinuation::Abrupt(completion),
                &mut output,
            );
        }
        if !result.throwing.is_empty() {
            self.resume_after_finally(
                &result.throwing,
                finally_entry,
                &finally,
                FinallyContinuation::Throwing,
                &mut output,
            );
        }

        for (completion, sources) in finally.abrupt {
            let point = self.synthetic_point();
            self.connect(&sources, point, SemanticFlowEdgeKind::FinallyOverride);
            output.add_abrupt(completion, point);
        }
        if !finally.throwing.is_empty() {
            let point = self.synthetic_point();
            self.connect(
                &finally.throwing,
                point,
                SemanticFlowEdgeKind::FinallyOverride,
            );
            output.throwing.insert(point);
        }
        output
    }

    fn resume_after_finally(
        &mut self,
        sources: &BTreeSet<SemanticFlowPoint>,
        finally_entry: SemanticFlowPoint,
        finally: &FlowFragment,
        continuation: FinallyContinuation,
        output: &mut FlowFragment,
    ) {
        if sources.is_empty() {
            return;
        }
        let token = self.next_continuation;
        self.next_continuation += 1;
        self.connect(
            sources,
            finally_entry,
            SemanticFlowEdgeKind::FinallyEnter(token),
        );
        if !finally.normal {
            return;
        }
        let resume = self.synthetic_point();
        self.connect(
            &finally.exits,
            resume,
            SemanticFlowEdgeKind::FinallyResume(token),
        );
        match continuation {
            FinallyContinuation::Normal => {
                output.normal = true;
                output.exits.insert(resume);
            }
            FinallyContinuation::Abrupt(completion) => {
                output.add_abrupt(completion, resume);
            }
            FinallyContinuation::Throwing => {
                output.throwing.insert(resume);
            }
        }
    }

    fn label(&mut self, label: SemanticLabel, mut body: FlowFragment) -> FlowFragment {
        let breaks = body.take_abrupt(&Completion::BreakLabel(label));
        if !breaks.is_empty() {
            body.normal = true;
            body.exits.extend(breaks);
        }
        body
    }

    fn add_alternative(
        &mut self,
        sources: &BTreeSet<SemanticFlowPoint>,
        fragment: FlowFragment,
        result: &mut FlowFragment,
    ) {
        if let Some(entry) = fragment.entry {
            self.connect(sources, entry, SemanticFlowEdgeKind::Normal);
            if fragment.normal {
                result.normal = true;
                result.exits.extend(fragment.exits);
            }
        } else if fragment.normal {
            result.normal = true;
            result.exits.extend(sources);
        }
        result.merge_abrupt(fragment.abrupt);
        result.throwing.extend(fragment.throwing);
    }

    fn empty_node(&mut self) -> FlowFragment {
        let point = self.synthetic_point();
        FlowFragment {
            entry: Some(point),
            normal: true,
            exits: BTreeSet::from([point]),
            abrupt: BTreeMap::new(),
            throwing: BTreeSet::new(),
        }
    }

    fn synthetic_point(&mut self) -> SemanticFlowPoint {
        let point = SemanticFlowPoint::Synthetic(self.next_synthetic);
        self.next_synthetic += 1;
        point
    }

    fn connect_to_fragment(
        &mut self,
        sources: &BTreeSet<SemanticFlowPoint>,
        fragment: &FlowFragment,
        kind: SemanticFlowEdgeKind,
    ) {
        if let Some(entry) = fragment.entry {
            self.connect(sources, entry, kind);
        }
    }

    fn connect(
        &mut self,
        sources: &BTreeSet<SemanticFlowPoint>,
        target: SemanticFlowPoint,
        kind: SemanticFlowEdgeKind,
    ) {
        for source in sources {
            self.add_edge(*source, target, kind);
        }
    }

    fn add_edge(
        &mut self,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
        kind: SemanticFlowEdgeKind,
    ) {
        let predecessors = self.graph.predecessors.entry(target).or_default();
        if !predecessors.contains(&(source, kind)) {
            predecessors.push((source, kind));
            predecessors.sort();
            let successors = self.graph.successors.entry(source).or_default();
            if !successors.contains(&target) {
                successors.push(target);
                successors.sort();
            }
        }
    }
}

fn break_completion(control: SemanticLoopControl) -> Completion {
    match control {
        SemanticLoopControl::Region(region) => Completion::Break(region),
        SemanticLoopControl::Label(label) => Completion::BreakLabel(label),
    }
}

fn continue_completion(control: SemanticLoopControl) -> Completion {
    match control {
        SemanticLoopControl::Region(region) => Completion::Continue(region),
        SemanticLoopControl::Label(label) => Completion::ContinueLabel(label),
    }
}

fn expression_may_throw(expression: &SemanticExpression) -> bool {
    expression.effects().may_throw()
}

fn predicate_may_throw(predicate: &SemanticPredicate) -> bool {
    let mut pending = vec![predicate];
    while let Some(predicate) = pending.pop() {
        match predicate {
            SemanticPredicate::Test(instruction) if instruction.effects().may_throw() => {
                return true;
            }
            SemanticPredicate::Test(_) | SemanticPredicate::True | SemanticPredicate::False => {}
            SemanticPredicate::Not(inner) => pending.push(inner),
            SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                pending.extend(terms);
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(id: u64) -> SemanticFlowPoint {
        SemanticFlowPoint::before(SemanticSiteId(id))
    }

    #[test]
    fn must_reach_accepts_linear_flow() {
        let source = point(0);
        let middle = point(1);
        let target = point(2);
        let graph = SemanticFlowGraph {
            successors: BTreeMap::from([(source, vec![middle]), (middle, vec![target])]),
            complete: true,
            ..SemanticFlowGraph::default()
        };

        assert!(graph.must_reach(source, target));
    }

    #[test]
    fn must_reach_rejects_terminal_bypass() {
        let source = point(0);
        let target = point(1);
        let terminal = point(2);
        let graph = SemanticFlowGraph {
            successors: BTreeMap::from([(source, vec![target, terminal])]),
            complete: true,
            ..SemanticFlowGraph::default()
        };

        assert!(!graph.must_reach(source, target));
    }

    #[test]
    fn must_reach_rejects_target_avoiding_cycle() {
        let source = point(0);
        let cycle = point(1);
        let target = point(2);
        let graph = SemanticFlowGraph {
            successors: BTreeMap::from([(source, vec![cycle]), (cycle, vec![target, cycle])]),
            complete: true,
            ..SemanticFlowGraph::default()
        };

        assert!(!graph.must_reach(source, target));
    }

    #[test]
    fn sparse_reachability_matches_dense_reachability() {
        let points = (0..7).map(point).collect::<Vec<_>>();
        let mut graph = SemanticFlowGraph {
            complete: true,
            ..SemanticFlowGraph::default()
        };
        for (source, target) in [(0, 1), (0, 2), (1, 3), (2, 3), (3, 4), (4, 3), (4, 5)] {
            graph
                .successors
                .entry(points[source])
                .or_default()
                .push(points[target]);
            graph
                .predecessors
                .entry(points[target])
                .or_default()
                .push((points[source], SemanticFlowEdgeKind::Normal));
        }
        graph.entries.insert(points[0]);

        let dense = SemanticReachability::analyze_with_dense_limit(&graph, usize::MAX);
        let sparse = SemanticReachability::analyze_with_dense_limit(&graph, 0);
        assert!(matches!(sparse.store, ReachabilityStore::Sparse(_)));
        for source in &points {
            for target in &points {
                assert_eq!(
                    sparse.reaches(*source, *target),
                    dense.reaches(*source, *target),
                    "reachability mismatch from {source:?} to {target:?}"
                );
            }
        }

        let candidates = BTreeSet::from([points[1], points[2], points[6]]);
        assert_eq!(
            sparse.reaches_any_between(points[0], points[5], &candidates),
            dense.reaches_any_between(points[0], points[5], &candidates)
        );
    }

    #[test]
    fn reaching_values_propagate_across_revisited_cycle_nodes() {
        let points = (0..4).map(point).collect::<Vec<_>>();
        let mut graph = SemanticFlowGraph {
            complete: true,
            ..SemanticFlowGraph::default()
        };
        for (source, target) in [(0, 1), (0, 2), (1, 3), (2, 3), (3, 1)] {
            graph
                .successors
                .entry(points[source])
                .or_default()
                .push(points[target]);
            graph
                .predecessors
                .entry(points[target])
                .or_default()
                .push((points[source], SemanticFlowEdgeKind::Normal));
        }
        graph.entries.insert(points[0]);

        let reaching = graph.reaching_values(
            &[
                SemanticValueDefinition::new(points[0], 0, 0),
                SemanticValueDefinition::new(points[1], 0, 1),
            ],
            1,
            &BTreeSet::from([points[3]]),
        );

        assert_eq!(reaching.at(0, points[3]), Some(vec![0, 1]));
    }
}
