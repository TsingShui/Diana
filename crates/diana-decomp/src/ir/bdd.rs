//! Method-local reduced ordered binary decision diagrams.
//!
//! Handles carry their arena identity, so control domains from independent
//! methods cannot be mixed accidentally. All externally supplied handles and
//! expression symbols are validated before graph operations are evaluated.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

use foldhash::{HashMap, HashMapExt, HashSet};
use std::fmt;
use std::hash::BuildHasher;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::ir::bool_expr::{BoolExpr, BoolVariable};

static NEXT_CONTEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bdd {
    context: u64,
    node: u16,
}

impl Bdd {
    // Cache keys are local to the validating manager. The node limit fits
    // in 16 bits, so two child IDs fit in a single integer.
    fn pair(self, other: Self) -> u32 {
        u32::from(self.node) << 16 | u32::from(other.node)
    }

    pub fn is_false(self) -> bool {
        self.node == 0
    }

    pub fn is_true(self) -> bool {
        self.node == 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BddError {
    UnknownVariable(BoolVariable),
    ForeignHandle,
    InvalidHandle(usize),
    MalformedEvaluation,
    ResourceLimit { resource: BddResource, limit: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BddResource {
    Nodes,
    CacheEntries,
    Operations,
}

impl fmt::Display for BddError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownVariable(variable) => {
                write!(formatter, "boolean domain does not contain {variable:?}")
            }
            Self::ForeignHandle => formatter.write_str("BDD handle belongs to a different context"),
            Self::InvalidHandle(node) => write!(formatter, "BDD node {node} does not exist"),
            Self::MalformedEvaluation => formatter.write_str("malformed BDD evaluation work stack"),
            Self::ResourceLimit { resource, limit } => {
                write!(
                    formatter,
                    "BDD {resource:?} resource limit {limit} exceeded"
                )
            }
        }
    }
}

impl std::error::Error for BddError {}

impl BddError {
    pub fn is_resource_limit(&self) -> bool {
        matches!(self, Self::ResourceLimit { .. })
    }
}

#[derive(Debug, Clone, Copy)]
struct Node {
    variable: usize,
    low: Bdd,
    high: Bdd,
}

#[derive(Clone, Copy)]
struct ArenaNode {
    variable: usize,
    low: u16,
    high: u16,
    complement: u16,
    has_complement: bool,
    cached_pairs: u8,
}

impl ArenaNode {
    fn key(&self) -> (usize, u32) {
        (
            self.variable,
            u32::from(self.low) << 16 | u32::from(self.high),
        )
    }

    fn complement(&self) -> Option<u16> {
        self.has_complement.then_some(self.complement)
    }

    fn set_complement(&mut self, id: u16) {
        self.has_complement = true;
        self.complement = id;
    }

    fn is_literal(&self) -> bool {
        self.low <= 1 && self.high <= 1
    }

    fn cofactors(&self, id: u16, variable: usize) -> (u16, u16) {
        if self.variable == variable {
            (self.low, self.high)
        } else {
            (id, id)
        }
    }
}

/// Above the linear scan limit, the first part holds bucket heads.
/// The rest holds one link per
/// nonterminal arena node, indexed by its insertion-order ID minus two.
/// Zero ends a chain. Both terminal IDs are excluded from the table.
#[derive(Clone)]
struct UniqueNodes {
    slots: Vec<u16>,
    buckets: usize,
    hasher: foldhash::fast::RandomState,
}

impl UniqueNodes {
    const NODES_PER_BUCKET: usize = 4;
    const LINEAR_LIMIT: usize = 16;

    fn new() -> Self {
        Self {
            slots: Vec::new(),
            buckets: 0,
            hasher: foldhash::fast::RandomState::default(),
        }
    }

    fn lookup(&self, nodes: &[ArenaNode], key: (usize, u32)) -> (u16, usize) {
        let youngest_child = ((key.1 >> 16) as u16).max(key.1 as u16);
        if self.buckets == 0 {
            // Small arenas need no separate index. Only nodes created after
            // both children can be parents. Most new keys use a recent child.
            let first = usize::from(youngest_child).saturating_add(1).max(2);
            for id in (first..nodes.len()).rev() {
                if nodes[id].key() == key {
                    return (id as u16, 0);
                }
            }
            return (0, 0);
        }
        let hash = self.hasher.hash_one(key) as usize;
        let mut candidate = self.slots[hash & (self.buckets - 1)];
        // Every parent was interned after both children. Bucket links lead
        // to progressively older IDs, so no older entry can match either.
        while candidate > youngest_child {
            if nodes[usize::from(candidate)].key() == key {
                return (candidate, hash);
            }
            candidate = self.slots[self.buckets + usize::from(candidate) - 2];
        }
        (0, hash)
    }

    fn insert(&mut self, mut hash: usize, id: u16, nodes: &[ArenaNode], key: (usize, u32)) {
        let count = usize::from(id) - 2;
        if count < Self::LINEAR_LIMIT {
            return;
        }
        if self.buckets == 0 || count == self.buckets * Self::NODES_PER_BUCKET {
            if self.buckets == 0 {
                self.buckets = Self::LINEAR_LIMIT * 2 / Self::NODES_PER_BUCKET;
                hash = self.hasher.hash_one(key) as usize;
            } else {
                self.buckets *= 2;
            }
            let mask = self.buckets - 1;
            let mut slots = vec![0; self.buckets * (1 + Self::NODES_PER_BUCKET)];
            for (id, node) in nodes.iter().enumerate().skip(2) {
                let bucket = self.hasher.hash_one(node.key()) as usize & mask;
                slots[self.buckets + id - 2] = slots[bucket];
                slots[bucket] = id as u16;
            }
            self.slots = slots;
        }
        let bucket = hash & (self.buckets - 1);
        self.slots[self.buckets + usize::from(id) - 2] = self.slots[bucket];
        self.slots[bucket] = id;
    }
}

/// Nonterminal binary cache keys are strictly ordered node pairs. In
/// particular zero is never a key, although either terminal is a valid result.
#[derive(Clone)]
struct BinaryMemo {
    slots: smallvec::SmallVec<[u64; 4]>,
    len: usize,
    hasher: foldhash::fast::RandomState,
}

impl BinaryMemo {
    fn new() -> Self {
        Self {
            slots: smallvec::SmallVec::from_buf([0; 4]),
            len: 0,
            hasher: foldhash::fast::RandomState::default(),
        }
    }

    fn lookup(&self, key: u32) -> MemoLookup {
        let len = self.slots.len();
        let mask = len - 1;
        let mut pos = self.hasher.hash_one(key) as usize & mask;
        loop {
            let entry = self.slots[pos];
            if entry == 0 {
                // The highest bit identifies this power-of-two table size.
                // An oversized table can still use the regular hash probe.
                return MemoLookup::Vacant(MemoVacancy(u32::try_from(len | pos).unwrap_or(0)));
            }
            if entry as u32 == key {
                return MemoLookup::Cached((entry >> 32) as u16);
            }
            pos = (pos + 1) & mask;
        }
    }

    fn place(slots: &mut [u64], hash: usize, entry: u64) {
        let mask = slots.len() - 1;
        let mut pos = hash & mask;
        while slots[pos] != 0 {
            pos = (pos + 1) & mask;
        }
        slots[pos] = entry;
    }

    fn insert(&mut self, key: u32, value: u16, vacancy: MemoVacancy) {
        // The caller missed this pair before descending. Every recursive
        // cofactor advances past its top variable, so descendants cannot
        // have installed this key while its frame was pending.
        if self.len == self.slots.len() / 4 * 3 {
            let mut larger = smallvec::smallvec![0; self.slots.len() * 2];
            for &entry in &self.slots {
                if entry != 0 {
                    Self::place(
                        &mut larger,
                        self.hasher.hash_one(entry as u32) as usize,
                        entry,
                    );
                }
            }
            self.slots = larger;
        }
        let entry = u64::from(value) << 32 | u64::from(key);
        // Descendants may have occupied the first vacant bucket or grown
        // the table. Without growth, all earlier probe buckets stay full.
        let position = (vacancy.0 as usize)
            .checked_sub(self.slots.len())
            .unwrap_or_else(|| self.hasher.hash_one(key) as usize);
        Self::place(&mut self.slots, position, entry);
        self.len += 1;
    }
}

#[derive(Clone, Copy)]
struct MemoVacancy(u32);

enum MemoLookup {
    Cached(u16),
    Vacant(MemoVacancy),
}

#[derive(Clone)]
struct Manager {
    context: u64,
    nodes: Vec<ArenaNode>,
    // Nodes already own their immutable keys. The index stores only arena IDs.
    unique: UniqueNodes,
    binary_cache: BinaryMemo,
    apply_work: Vec<ApplyFrame>,
    implication_work: ImplicationWork,
    cache_entries: usize,
    implication_cache: HashMap<u32, bool>,
    limits: BddLimits,
    operations_used: usize,
}

impl Manager {
    fn new(context: u64, limits: BddLimits) -> Self {
        let falsity = ArenaNode {
            variable: usize::MAX,
            low: 0,
            high: 1,
            complement: 1,
            has_complement: true,
            cached_pairs: 0,
        };
        Self {
            context,
            nodes: vec![
                falsity,
                ArenaNode {
                    complement: 0,
                    ..falsity
                },
            ],
            unique: UniqueNodes::new(),
            binary_cache: BinaryMemo::new(),
            apply_work: Vec::new(),
            implication_work: ImplicationWork::default(),
            // Both terminals already know their complements.
            cache_entries: 2,
            implication_cache: HashMap::new(),
            limits,
            operations_used: 0,
        }
    }

    fn handle(&self, node: u16) -> Bdd {
        Bdd {
            context: self.context,
            node,
        }
    }

    fn validate(&self, value: Bdd) -> Result<Node, BddError> {
        if value.context != self.context {
            return Err(BddError::ForeignHandle);
        }
        let node = self
            .nodes
            .get(usize::from(value.node))
            .ok_or(BddError::InvalidHandle(usize::from(value.node)))?;
        Ok(Node {
            variable: node.variable,
            low: self.handle(node.low),
            high: self.handle(node.high),
        })
    }

    fn variable(&mut self, variable: usize) -> Result<Bdd, BddError> {
        self.node(variable, self.handle(0), self.handle(1))
    }

    fn node(&mut self, variable: usize, low: Bdd, high: Bdd) -> Result<Bdd, BddError> {
        self.validate(low)?;
        self.validate(high)?;
        self.intern(variable, low.node, high.node)
            .map(|node| self.handle(node))
    }

    /// Both children originate in this arena. Public operations validate their
    /// handles at entry and the arena only stores children it already owns.
    fn intern(&mut self, variable: usize, low: u16, high: u16) -> Result<u16, BddError> {
        if low == high {
            return Ok(low);
        }
        let key = (variable, u32::from(low) << 16 | u32::from(high));
        let (existing, hash) = self.unique.lookup(&self.nodes, key);
        if existing != 0 {
            return Ok(existing);
        }
        self.insert_node(variable, low, high, hash)
    }

    fn insert_node(
        &mut self,
        variable: usize,
        low: u16,
        high: u16,
        hash: usize,
    ) -> Result<u16, BddError> {
        if self.nodes.len() >= self.limits.nodes {
            return Err(BddError::ResourceLimit {
                resource: BddResource::Nodes,
                limit: self.limits.nodes,
            });
        }
        let id = u16::try_from(self.nodes.len()).map_err(|_| BddError::ResourceLimit {
            resource: BddResource::Nodes,
            limit: self.limits.nodes,
        })?;
        self.unique.insert(
            hash,
            id,
            &self.nodes,
            (variable, u32::from(low) << 16 | u32::from(high)),
        );
        self.nodes.push(ArenaNode {
            variable,
            low,
            high,
            complement: 0,
            has_complement: false,
            cached_pairs: 0,
        });
        Ok(id)
    }

    fn literal_prefix<const AND: bool>(
        &mut self,
        literal: ArenaNode,
        other: u16,
    ) -> Result<u16, BddError> {
        // The resulting node has exactly one terminal child. Its variable,
        // the position and value of that child, and its other child uniquely
        // recover the earlier literal, operation and later operand.
        const CACHED_PREFIX: u8 = 32;
        let terminal = u16::from(!AND);
        let (low, high) = if literal.low == terminal {
            (terminal, other)
        } else {
            (other, terminal)
        };
        let key = (literal.variable, u32::from(low) << 16 | u32::from(high));
        let (existing, hash) = self.unique.lookup(&self.nodes, key);
        if existing != 0 && self.nodes[usize::from(existing)].cached_pairs & CACHED_PREFIX != 0 {
            return Ok(existing);
        }
        self.charge_operation()?;
        let result = if existing == 0 {
            self.insert_node(literal.variable, low, high, hash)?
        } else {
            existing
        };
        self.charge_cache_entries(1)?;
        self.nodes[usize::from(result)].cached_pairs |= CACHED_PREFIX;
        Ok(result)
    }

    fn and(&mut self, left: Bdd, right: Bdd) -> Result<Bdd, BddError> {
        self.apply::<true>(left, right)
    }

    fn or(&mut self, left: Bdd, right: Bdd) -> Result<Bdd, BddError> {
        self.apply::<false>(left, right)
    }

    fn apply<const AND: bool>(&mut self, left: Bdd, right: Bdd) -> Result<Bdd, BddError> {
        self.validate(left)?;
        self.validate(right)?;
        let result = self.apply_nodes::<AND>(left.node, right.node);
        if result.is_err() {
            self.apply_work.clear();
        }
        result.map(|id| self.handle(id))
    }

    fn apply_nodes<const AND: bool>(
        &mut self,
        mut left: u16,
        mut right: u16,
    ) -> Result<u16, BddError> {
        if let Some(result) = Self::terminal_apply::<AND>(left, right) {
            return Ok(result);
        }
        'descend: loop {
            let (lo, hi) = (left.min(right), left.max(right));
            let (first, second) = if AND { (lo, hi) } else { (hi, lo) };
            let key = u32::from(first) << 16 | u32::from(second);
            let a = self.nodes[usize::from(left)];
            let b = self.nodes[usize::from(right)];
            let lookup = if a.variable < b.variable && a.is_literal() {
                MemoLookup::Cached(self.literal_prefix::<AND>(a, right)?)
            } else if b.variable < a.variable && b.is_literal() {
                MemoLookup::Cached(self.literal_prefix::<AND>(b, left)?)
            } else {
                self.binary_cache.lookup(key)
            };
            let mut result = match lookup {
                MemoLookup::Cached(result) => result,
                MemoLookup::Vacant(vacancy) => {
                    self.charge_operation()?;
                    let variable = a.variable.min(b.variable);
                    let (a_low, a_high) = a.cofactors(left, variable);
                    let (b_low, b_high) = b.cofactors(right, variable);
                    // Terminal and equal cofactors have no memo entry, charge or
                    // allocation. Descend only for pairs that can produce work.
                    match (
                        Self::terminal_apply::<AND>(a_low, b_low),
                        Self::terminal_apply::<AND>(a_high, b_high),
                    ) {
                        (Some(low), Some(high)) => {
                            self.finish_apply(key, variable, low, high, vacancy)?
                        }
                        (low, high) => {
                            let (pair, next) = match (low, high) {
                                (Some(low), _) => ((a_high, b_high), ApplyChild::LowResult(low)),
                                (_, Some(high)) => ((a_low, b_low), ApplyChild::HighResult(high)),
                                _ => ((a_low, b_low), ApplyChild::High(a_high, b_high)),
                            };
                            self.apply_work.push(ApplyFrame {
                                vacancy,
                                key,
                                variable,
                                next,
                            });
                            (left, right) = pair;
                            continue;
                        }
                    }
                }
            };
            while let Some(frame) = self.apply_work.last_mut() {
                let (low, high) = match frame.next {
                    ApplyChild::High(a, b) => {
                        frame.next = ApplyChild::LowResult(result);
                        (left, right) = (a, b);
                        continue 'descend;
                    }
                    ApplyChild::LowResult(low) => (low, result),
                    ApplyChild::HighResult(high) => (result, high),
                };
                let (key, variable, vacancy) = (frame.key, frame.variable, frame.vacancy);
                self.apply_work.pop();
                result = self.finish_apply(key, variable, low, high, vacancy)?;
            }
            return Ok(result);
        }
    }

    fn terminal_apply<const AND: bool>(left: u16, right: u16) -> Option<u16> {
        let absorbing = u16::from(!AND);
        let neutral = u16::from(AND);
        if left == right || right == neutral {
            Some(left)
        } else if left == neutral {
            Some(right)
        } else if left == absorbing || right == absorbing {
            Some(absorbing)
        } else {
            None
        }
    }

    fn finish_apply(
        &mut self,
        key: u32,
        variable: usize,
        low: u16,
        high: u16,
        vacancy: MemoVacancy,
    ) -> Result<u16, BddError> {
        let result = self.intern(variable, low, high)?;
        self.charge_cache_entries(1)?;
        self.binary_cache.insert(key, result, vacancy);
        Ok(result)
    }

    fn negate(&mut self, value: Bdd) -> Result<Bdd, BddError> {
        self.validate(value)?;
        let mut node = value.node;
        // A completed child stores its complement on the arena node. Retain
        // only outstanding parents, and read both child results from the arena.
        let mut parents = smallvec::SmallVec::<[u16; 16]>::new();
        loop {
            let current = self.nodes[usize::from(node)];
            if current.complement().is_none() {
                self.charge_operation()?;
                parents.push(node);
                node = current.low;
                continue;
            }
            loop {
                let Some(&parent) = parents.last() else {
                    return Ok(self.handle(
                        self.nodes[usize::from(node)]
                            .complement()
                            .ok_or(BddError::MalformedEvaluation)?,
                    ));
                };
                let current = self.nodes[usize::from(parent)];
                let low = self.nodes[usize::from(current.low)]
                    .complement()
                    .ok_or(BddError::MalformedEvaluation)?;
                let Some(high) = self.nodes[usize::from(current.high)].complement() else {
                    node = current.high;
                    break;
                };
                let result = self.intern(current.variable, low, high)?;
                let additional = usize::from(!self.nodes[usize::from(parent)].has_complement)
                    + usize::from(!self.nodes[usize::from(result)].has_complement);
                self.charge_cache_entries(additional)?;
                self.nodes[usize::from(parent)].set_complement(result);
                self.nodes[usize::from(result)].set_complement(parent);
                parents.pop();
                node = parent;
            }
        }
    }

    fn restrict(
        &mut self,
        value: Bdd,
        assignments: &BTreeMap<usize, bool>,
    ) -> Result<Bdd, BddError> {
        self.validate(value)?;
        let mut tasks = vec![RestrictTask::Visit(value)];
        let mut results = Vec::new();
        let mut memo = BTreeMap::<Bdd, Bdd>::new();
        while let Some(task) = tasks.pop() {
            match task {
                RestrictTask::Visit(value) => {
                    if let Some(result) = memo.get(&value) {
                        results.push(*result);
                        continue;
                    }
                    if value.is_false() || value.is_true() {
                        memo.insert(value, value);
                        results.push(value);
                        continue;
                    }
                    self.charge_operation()?;
                    let node = self.validate(value)?;
                    if let Some(high) = assignments.get(&node.variable) {
                        tasks.push(RestrictTask::Alias(value));
                        tasks.push(RestrictTask::Visit(if *high {
                            node.high
                        } else {
                            node.low
                        }));
                    } else {
                        tasks.push(RestrictTask::Combine {
                            value,
                            variable: node.variable,
                        });
                        tasks.push(RestrictTask::Visit(node.high));
                        tasks.push(RestrictTask::Visit(node.low));
                    }
                }
                RestrictTask::Alias(value) => {
                    let result = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    memo.insert(value, result);
                    results.push(result);
                }
                RestrictTask::Combine { value, variable } => {
                    let high = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    let low = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    let result = self.node(variable, low, high)?;
                    memo.insert(value, result);
                    results.push(result);
                }
            }
        }
        if results.len() != 1 {
            return Err(BddError::MalformedEvaluation);
        }
        results.pop().ok_or(BddError::MalformedEvaluation)
    }

    fn exists(&mut self, value: Bdd, variables: &BTreeSet<usize>) -> Result<Bdd, BddError> {
        self.validate(value)?;
        let mut tasks = vec![ExistsTask::Visit(value)];
        let mut results = Vec::new();
        let mut memo = BTreeMap::<Bdd, Bdd>::new();
        while let Some(task) = tasks.pop() {
            match task {
                ExistsTask::Visit(value) => {
                    if let Some(result) = memo.get(&value) {
                        results.push(*result);
                        continue;
                    }
                    if value.is_false() || value.is_true() {
                        memo.insert(value, value);
                        results.push(value);
                        continue;
                    }
                    self.charge_operation()?;
                    let node = self.validate(value)?;
                    tasks.push(ExistsTask::Combine {
                        value,
                        variable: node.variable,
                        quantified: variables.contains(&node.variable),
                    });
                    tasks.push(ExistsTask::Visit(node.high));
                    tasks.push(ExistsTask::Visit(node.low));
                }
                ExistsTask::Combine {
                    value,
                    variable,
                    quantified,
                } => {
                    let high = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    let low = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    let result = if quantified {
                        self.or(low, high)?
                    } else {
                        self.node(variable, low, high)?
                    };
                    memo.insert(value, result);
                    results.push(result);
                }
            }
        }
        if results.len() != 1 {
            return Err(BddError::MalformedEvaluation);
        }
        results.pop().ok_or(BddError::MalformedEvaluation)
    }

    fn constrain(&mut self, value: Bdd, care: Bdd) -> Result<Bdd, BddError> {
        self.validate(value)?;
        self.validate(care)?;
        let mut tasks = vec![ConstrainTask::Visit(value, care)];
        let mut results = Vec::new();
        let mut memo = BTreeMap::<(Bdd, Bdd), Bdd>::new();
        while let Some(task) = tasks.pop() {
            match task {
                ConstrainTask::Visit(value, care) => {
                    if let Some(result) = memo.get(&(value, care)) {
                        results.push(*result);
                        continue;
                    }
                    if care.is_false() {
                        results.push(self.handle(0));
                        continue;
                    }
                    if care.is_true() || value.is_false() || value.is_true() {
                        memo.insert((value, care), value);
                        results.push(value);
                        continue;
                    }
                    self.charge_operation()?;
                    let variable = self.top_variable(value, care)?;
                    let (value_low, value_high) = self.cofactors(value, variable)?;
                    let (care_low, care_high) = self.cofactors(care, variable)?;
                    if care_low.is_false() {
                        tasks.push(ConstrainTask::Alias(value, care));
                        tasks.push(ConstrainTask::Visit(value_high, care_high));
                    } else if care_high.is_false() {
                        tasks.push(ConstrainTask::Alias(value, care));
                        tasks.push(ConstrainTask::Visit(value_low, care_low));
                    } else {
                        tasks.push(ConstrainTask::Combine {
                            value,
                            care,
                            variable,
                        });
                        tasks.push(ConstrainTask::Visit(value_high, care_high));
                        tasks.push(ConstrainTask::Visit(value_low, care_low));
                    }
                }
                ConstrainTask::Alias(value, care) => {
                    let result = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    memo.insert((value, care), result);
                    results.push(result);
                }
                ConstrainTask::Combine {
                    value,
                    care,
                    variable,
                } => {
                    let high = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    let low = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    let result = self.node(variable, low, high)?;
                    memo.insert((value, care), result);
                    results.push(result);
                }
            }
        }
        if results.len() != 1 {
            return Err(BddError::MalformedEvaluation);
        }
        results.pop().ok_or(BddError::MalformedEvaluation)
    }

    fn implies(&mut self, premise: Bdd, consequence: Bdd) -> Result<bool, BddError> {
        self.validate(premise)?;
        self.validate(consequence)?;
        // Each identity or terminal pair has a fixed answer. Remember that
        // exact cache entry on the other node and charge it once, just as for
        // a pair stored in the general implication table.
        let (a, b) = (premise.node, consequence.node);
        let trivial = if a == b {
            Some((a, 1, true))
        } else if a <= 1 {
            Some((b, 2 << a, a == 0))
        } else if b <= 1 {
            Some((a, 8 << b, b == 1))
        } else {
            None
        };
        if let Some((id, flag, answer)) = trivial {
            let index = usize::from(id);
            if self.nodes[index].cached_pairs & flag == 0 {
                self.charge_cache_entries(1)?;
                self.nodes[index].cached_pairs |= flag;
            }
            return Ok(answer);
        }
        let key = premise.pair(consequence);
        if let Some(result) = self.implication_cache.get(&key) {
            return Ok(*result);
        }
        let result = self.implies_nodes(premise.node, consequence.node)?;
        self.charge_cache_entries(1)?;
        self.implication_cache.insert(key, result);
        Ok(result)
    }

    fn implies_nodes(&mut self, premise: u16, consequence: u16) -> Result<bool, BddError> {
        self.implication_work.begin();
        let mut next = (premise, consequence);
        loop {
            let (premise, consequence) = next;
            if self
                .implication_work
                .visit(u32::from(premise) << 16 | u32::from(consequence))
            {
                self.charge_operation()?;
                let a = self.nodes[usize::from(premise)];
                let b = self.nodes[usize::from(consequence)];
                let variable = a.variable.min(b.variable);
                let (a_low, a_high) = a.cofactors(premise, variable);
                let (b_low, b_high) = b.cofactors(consequence, variable);
                // Check the high cofactor first. A false low cofactor must
                // wait for any unfinished high proof and its budget checks.
                match Self::terminal_implication(a_high, b_high) {
                    Some(false) => return Ok(false),
                    Some(true) => match Self::terminal_implication(a_low, b_low) {
                        Some(false) => return Ok(false),
                        Some(true) => {}
                        None => {
                            next = (a_low, b_low);
                            continue;
                        }
                    },
                    None => {
                        if Self::terminal_implication(a_low, b_low) != Some(true) {
                            self.implication_work.pending.push((a_low, b_low));
                        }
                        next = (a_high, b_high);
                        continue;
                    }
                }
            }
            let Some((a, b)) = self.implication_work.pending.pop() else {
                return Ok(true);
            };
            if a == 1 || b == 0 {
                return Ok(false);
            }
            next = (a, b);
        }
    }

    fn terminal_implication(premise: u16, consequence: u16) -> Option<bool> {
        if premise == 0 || consequence == 1 || premise == consequence {
            Some(true)
        } else if premise == 1 || consequence == 0 {
            Some(false)
        } else {
            None
        }
    }

    fn top_variable(&self, left: Bdd, right: Bdd) -> Result<usize, BddError> {
        Ok(self.variable_of(left)?.min(self.variable_of(right)?))
    }

    fn variable_of(&self, value: Bdd) -> Result<usize, BddError> {
        Ok(self.validate(value)?.variable)
    }

    fn cofactors(&self, value: Bdd, variable: usize) -> Result<(Bdd, Bdd), BddError> {
        let node = self.validate(value)?;
        Ok(if node.variable == variable {
            (node.low, node.high)
        } else {
            (value, value)
        })
    }

    fn charge_cache_entries(&mut self, additional: usize) -> Result<(), BddError> {
        let entries = self.cache_entries.saturating_add(additional);
        if entries > self.limits.cache_entries {
            return Err(BddError::ResourceLimit {
                resource: BddResource::CacheEntries,
                limit: self.limits.cache_entries,
            });
        }
        self.cache_entries = entries;
        Ok(())
    }

    fn charge_operation(&mut self) -> Result<(), BddError> {
        if self.operations_used >= self.limits.operations {
            return Err(BddError::ResourceLimit {
                resource: BddResource::Operations,
                limit: self.limits.operations,
            });
        }
        self.operations_used += 1;
        Ok(())
    }

    fn begin_operation(&mut self) {
        self.operations_used = 0;
    }
}

#[derive(Clone)]
struct ApplyFrame {
    vacancy: MemoVacancy,
    key: u32,
    variable: usize,
    next: ApplyChild,
}

#[derive(Clone)]
enum ApplyChild {
    High(u16, u16),
    LowResult(u16),
    HighResult(u16),
}

#[derive(Clone, Default)]
struct ImplicationWork {
    pending: Vec<(u16, u16)>,
    first: [u32; 8],
    first_len: usize,
    visited: HashSet<u32>,
}

impl ImplicationWork {
    fn begin(&mut self) {
        self.pending.clear();
        self.first_len = 0;
    }

    fn visit(&mut self, pair: u32) -> bool {
        // Short proofs use only a handful of pairs. Keep those on the
        // work stack even after a larger proof grew the fallback table.
        if self.first_len <= self.first.len() {
            if self.first[..self.first_len].contains(&pair) {
                return false;
            }
            if self.first_len < self.first.len() {
                self.first[self.first_len] = pair;
                self.first_len += 1;
                return true;
            }
            self.visited.clear();
            self.visited.extend(self.first);
            self.first_len += 1;
        }
        self.visited.insert(pair)
    }
}

enum RestrictTask {
    Visit(Bdd),
    Alias(Bdd),
    Combine { value: Bdd, variable: usize },
}

enum ConstrainTask {
    Visit(Bdd, Bdd),
    Alias(Bdd, Bdd),
    Combine {
        value: Bdd,
        care: Bdd,
        variable: usize,
    },
}

enum ExistsTask {
    Visit(Bdd),
    Combine {
        value: Bdd,
        variable: usize,
        quantified: bool,
    },
}

type Cube = BTreeMap<BoolVariable, bool>;

struct DnfBuilder {
    cube_limit: usize,
}

impl DnfBuilder {
    fn new(cube_limit: usize) -> Self {
        Self { cube_limit }
    }

    fn build(&self, expression: &BoolExpr) -> Option<Vec<Cube>> {
        self.visit(expression, false, 0)
    }

    fn visit(&self, expression: &BoolExpr, negated: bool, depth: usize) -> Option<Vec<Cube>> {
        if depth > 128 {
            return None;
        }
        match expression {
            BoolExpr::True => Some(if negated {
                Vec::new()
            } else {
                vec![Cube::new()]
            }),
            BoolExpr::False => Some(if negated {
                vec![Cube::new()]
            } else {
                Vec::new()
            }),
            BoolExpr::Symbol(variable) => Some(vec![Cube::from([(variable.clone(), !negated)])]),
            BoolExpr::Not(inner) => self.visit(inner, !negated, depth + 1),
            BoolExpr::And(terms) | BoolExpr::Or(terms) => {
                let conjunction = matches!(expression, BoolExpr::And(_)) != negated;
                if conjunction {
                    self.conjunction(terms, negated, depth + 1)
                } else {
                    self.disjunction(terms, negated, depth + 1)
                }
            }
        }
    }

    fn conjunction(&self, terms: &[BoolExpr], negated: bool, depth: usize) -> Option<Vec<Cube>> {
        let mut product = vec![Cube::new()];
        for term in terms {
            let factor = self.visit(term, negated, depth)?;
            let mut next = Vec::new();
            for left in &product {
                for right in &factor {
                    if let Some(cube) = Self::merge(left, right) {
                        next.push(cube);
                        if next.len() > self.cube_limit {
                            return None;
                        }
                    }
                }
            }
            product = next;
            if product.is_empty() {
                break;
            }
        }
        Some(product)
    }

    fn disjunction(&self, terms: &[BoolExpr], negated: bool, depth: usize) -> Option<Vec<Cube>> {
        let mut union = Vec::new();
        for term in terms {
            union.extend(self.visit(term, negated, depth)?);
            if union.len() > self.cube_limit {
                return None;
            }
        }
        Some(union)
    }

    fn merge(left: &Cube, right: &Cube) -> Option<Cube> {
        let mut merged = left.clone();
        for (variable, polarity) in right {
            match merged.get(variable) {
                Some(existing) if existing != polarity => return None,
                Some(_) => {}
                None => {
                    merged.insert(variable.clone(), *polarity);
                }
            }
        }
        Some(merged)
    }
}

#[derive(Clone, Default)]
struct LiteralSlots {
    positive: Cell<Option<u16>>,
    negative: Cell<Option<u16>>,
}

pub struct BddContext {
    context: u64,
    variables_by_index: Vec<BoolVariable>,
    unordered_indices: Option<Box<HashMap<BoolVariable, usize>>>,
    literals: Vec<LiteralSlots>,
    manager: RefCell<Manager>,
    expressions: RefCell<BTreeMap<BoolExpr, Bdd>>,
    compiled_literals: Cell<usize>,
    limits: BddLimits,
}

#[derive(Debug, Clone, Copy)]
struct BddLimits {
    nodes: usize,
    cache_entries: usize,
    operations: usize,
}

impl BddLimits {
    fn for_variables(variables: usize) -> Self {
        let nodes = variables.max(1).saturating_mul(256).clamp(2_048, 65_536);
        Self {
            nodes,
            cache_entries: nodes.saturating_mul(4),
            operations: nodes.saturating_mul(8),
        }
    }
}

impl BddContext {
    pub fn new(names: &BTreeSet<BoolVariable>) -> Self {
        Self::ordered(names.iter().cloned())
    }

    pub fn ordered(names: impl IntoIterator<Item = BoolVariable>) -> Self {
        let context = NEXT_CONTEXT.fetch_add(1, Ordering::Relaxed);
        let mut variables_by_index: Vec<_> = names.into_iter().collect();
        // CFG predicate IDs and sets already arrive in ordinal order. Only a
        // caller supplying a different order needs a separate name index.
        let unordered_indices = if variables_by_index.is_sorted() {
            variables_by_index.dedup();
            None
        } else {
            let mut indices = HashMap::with_capacity(variables_by_index.len());
            variables_by_index.retain(|name| {
                let index = indices.len();
                if let std::collections::hash_map::Entry::Vacant(slot) = indices.entry(name.clone())
                {
                    slot.insert(index);
                    true
                } else {
                    false
                }
            });
            Some(Box::new(indices))
        };
        let limits = BddLimits::for_variables(variables_by_index.len());
        Self {
            context,
            literals: vec![LiteralSlots::default(); variables_by_index.len()],
            variables_by_index,
            unordered_indices,
            manager: RefCell::new(Manager::new(context, limits)),
            expressions: RefCell::new(BTreeMap::new()),
            compiled_literals: Cell::new(0),
            limits,
        }
    }

    pub(crate) fn variable_order(&self) -> &[BoolVariable] {
        &self.variables_by_index
    }

    pub(crate) fn variable_index(&self, name: &BoolVariable) -> Option<usize> {
        match &self.unordered_indices {
            Some(indices) => indices.get(name).copied(),
            None => self.variables_by_index.binary_search(name).ok(),
        }
    }

    /// Fork the exact bounded arena, rebinding only handles owned by this
    /// context. The two branches can subsequently intern different nodes at
    /// the same numeric position and must retain distinct handle identities.
    pub(crate) fn fork(
        &self,
        roots: impl IntoIterator<Item = Option<Bdd>>,
    ) -> Result<(Self, Vec<Option<Bdd>>), BddError> {
        let manager = self.manager.borrow();
        let context = NEXT_CONTEXT.fetch_add(1, Ordering::Relaxed);
        let rebind = |value: Bdd| {
            manager.validate(value)?;
            Ok(Bdd { context, ..value })
        };
        let roots = roots
            .into_iter()
            .map(|root| root.map(rebind).transpose())
            .collect::<Result<Vec<_>, BddError>>()?;
        let expressions = self
            .expressions
            .borrow()
            .iter()
            .map(|(expression, &proof)| Ok((expression.clone(), rebind(proof)?)))
            .collect::<Result<_, BddError>>()?;
        let mut manager = manager.clone();
        manager.context = context;
        Ok((
            Self {
                context,
                unordered_indices: self.unordered_indices.clone(),
                literals: self.literals.clone(),
                variables_by_index: self.variables_by_index.clone(),
                manager: RefCell::new(manager),
                expressions: RefCell::new(expressions),
                compiled_literals: self.compiled_literals.clone(),
                limits: self.limits,
            },
            roots,
        ))
    }

    pub fn truth(&self) -> Bdd {
        Bdd {
            context: self.context,
            node: 1,
        }
    }

    pub fn falsity(&self) -> Bdd {
        Bdd {
            context: self.context,
            node: 0,
        }
    }

    pub fn and(&self, left: Bdd, right: Bdd) -> Result<Bdd, BddError> {
        let mut manager = self.manager.borrow_mut();
        manager.begin_operation();
        manager.and(left, right)
    }

    pub fn or(&self, left: Bdd, right: Bdd) -> Result<Bdd, BddError> {
        let mut manager = self.manager.borrow_mut();
        manager.begin_operation();
        manager.or(left, right)
    }

    pub fn not(&self, value: Bdd) -> Result<Bdd, BddError> {
        let mut manager = self.manager.borrow_mut();
        manager.begin_operation();
        manager.negate(value)
    }

    pub fn restrict(
        &self,
        value: Bdd,
        assignments: &BTreeMap<BoolVariable, bool>,
    ) -> Result<Bdd, BddError> {
        let assignments = assignments
            .iter()
            .map(|(variable, value)| {
                self.variable_index(variable)
                    .map(|index| (index, *value))
                    .ok_or_else(|| BddError::UnknownVariable(variable.clone()))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let mut manager = self.manager.borrow_mut();
        manager.begin_operation();
        manager.restrict(value, &assignments)
    }

    pub fn exists(&self, value: Bdd, variables: &BTreeSet<BoolVariable>) -> Result<Bdd, BddError> {
        let variables = variables
            .iter()
            .map(|variable| {
                self.variable_index(variable)
                    .ok_or_else(|| BddError::UnknownVariable(variable.clone()))
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let mut manager = self.manager.borrow_mut();
        manager.begin_operation();
        manager.exists(value, &variables)
    }

    pub fn constrain(&self, value: Bdd, care: Bdd) -> Result<Bdd, BddError> {
        let mut manager = self.manager.borrow_mut();
        manager.begin_operation();
        manager.constrain(value, care)
    }

    pub(crate) fn compile_literal(
        &self,
        name: &BoolVariable,
        positive: bool,
    ) -> Result<Bdd, BddError> {
        let index = self
            .variable_index(name)
            .ok_or_else(|| BddError::UnknownVariable(name.clone()))?;
        self.compile_index(index, positive)
    }

    pub(crate) fn compile_index(&self, index: usize, positive: bool) -> Result<Bdd, BddError> {
        let symbol = self
            .literals
            .get(index)
            .ok_or(BddError::MalformedEvaluation)?;
        let slot = if positive {
            &symbol.positive
        } else {
            &symbol.negative
        };
        let handle = |node| Bdd {
            context: self.context,
            node,
        };
        if let Some(node) = slot.get() {
            return Ok(handle(node));
        }
        let value = if let Some(node) = symbol.positive.get() {
            handle(node)
        } else {
            let value = self.manager.borrow_mut().variable(index)?;
            self.record_literal(&symbol.positive, value)?;
            value
        };
        if positive {
            return Ok(value);
        }
        // Preserve the expression compiler's child-first budget and cache
        // behavior, including when recording the negation fails.
        let negated = self.not(value)?;
        self.record_literal(&symbol.negative, negated)?;
        Ok(negated)
    }

    fn record_literal(&self, slot: &Cell<Option<u16>>, value: Bdd) -> Result<(), BddError> {
        self.reserve_expression(self.expressions.borrow().len())?;
        slot.set(Some(value.node));
        self.compiled_literals.set(self.compiled_literals.get() + 1);
        Ok(())
    }

    fn compiled(&self, expression: &BoolExpr) -> Result<Option<Bdd>, BddError> {
        match expression {
            BoolExpr::True => Ok(Some(self.truth())),
            BoolExpr::False => Ok(Some(self.falsity())),
            BoolExpr::Symbol(name) => self.compile_literal(name, true).map(Some),
            BoolExpr::Not(inner) => match inner.as_ref() {
                BoolExpr::Symbol(name) => self.compile_literal(name, false).map(Some),
                _ => Ok(self.expressions.borrow().get(expression).copied()),
            },
            _ => Ok(self.expressions.borrow().get(expression).copied()),
        }
    }

    pub fn compile(&self, expression: &BoolExpr) -> Result<Bdd, BddError> {
        if let Some(value) = self.compiled(expression)? {
            return Ok(value);
        }
        let mut tasks = vec![CompileTask::Visit(expression)];
        let mut results = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                CompileTask::Visit(expression) => {
                    if let Some(value) = self.compiled(expression)? {
                        results.push(value);
                        continue;
                    }
                    let (operation, operands) = match expression {
                        BoolExpr::Not(inner) => {
                            (CompileOp::Not, std::slice::from_ref(inner.as_ref()))
                        }
                        BoolExpr::And(terms) => (CompileOp::And, terms.as_slice()),
                        BoolExpr::Or(terms) => (CompileOp::Or, terms.as_slice()),
                        _ => unreachable!("all leaf expressions compile directly"),
                    };
                    tasks.push(CompileTask::Combine {
                        expression,
                        operation,
                        operands: operands.len(),
                    });
                    tasks.extend(operands.iter().rev().map(CompileTask::Visit));
                }
                CompileTask::Combine {
                    expression,
                    operation,
                    operands,
                } => {
                    let start = results
                        .len()
                        .checked_sub(operands)
                        .ok_or(BddError::MalformedEvaluation)?;
                    let mut values = results.drain(start..);
                    let value =
                        match operation {
                            CompileOp::Not => {
                                let [value] = values.as_slice() else {
                                    return Err(BddError::MalformedEvaluation);
                                };
                                self.not(*value)?
                            }
                            CompileOp::And => values
                                .try_fold(self.truth(), |left, right| self.and(left, right))?,
                            CompileOp::Or => values
                                .try_fold(self.falsity(), |left, right| self.or(left, right))?,
                        };
                    drop(values);
                    self.record_expression(expression, value)?;
                    results.push(value);
                }
            }
        }
        if results.len() != 1 {
            return Err(BddError::MalformedEvaluation);
        }
        results.pop().ok_or(BddError::MalformedEvaluation)
    }

    pub fn are_equivalent(&self, left: &BoolExpr, right: &BoolExpr) -> Result<bool, BddError> {
        Ok(self.compile(left)? == self.compile(right)?)
    }

    pub fn implies(&self, premise: &BoolExpr, consequence: &BoolExpr) -> Result<bool, BddError> {
        let premise = self.compile(premise)?;
        let consequence = self.compile(consequence)?;
        self.implies_bdd(premise, consequence)
    }

    pub fn equivalent(&self, left: Bdd, right: Bdd) -> Result<bool, BddError> {
        self.manager.borrow().validate(left)?;
        self.manager.borrow().validate(right)?;
        Ok(left == right)
    }

    pub fn equivalent_under(&self, care: Bdd, left: Bdd, right: Bdd) -> Result<bool, BddError> {
        self.manager.borrow().validate(care)?;
        self.manager.borrow().validate(left)?;
        self.manager.borrow().validate(right)?;
        let difference = self.or(
            self.and(left, self.not(right)?)?,
            self.and(self.not(left)?, right)?,
        )?;
        Ok(self.and(care, difference)?.is_false())
    }

    pub fn reduce_under(
        &self,
        expression: &BoolExpr,
        care: Bdd,
        node_limit: usize,
    ) -> Result<Option<(BoolExpr, usize)>, BddError> {
        self.reduce_under_with_support(expression, care, &expression.symbols(), node_limit)
    }

    pub fn reduce_under_with_support(
        &self,
        expression: &BoolExpr,
        care: Bdd,
        support: &BTreeSet<BoolVariable>,
        node_limit: usize,
    ) -> Result<Option<(BoolExpr, usize)>, BddError> {
        self.manager.borrow().validate(care)?;
        if let Some(variable) = support
            .iter()
            .find(|variable| self.variable_index(variable).is_none())
        {
            return Err(BddError::UnknownVariable(variable.clone()));
        }
        let hidden = self
            .variables_by_index
            .iter()
            .filter(|variable| !support.contains(*variable))
            .cloned()
            .collect::<BTreeSet<_>>();
        let care = if hidden.is_empty() {
            care
        } else {
            let projected = self.exists(care, &hidden)?;
            if let Some((projected_expression, _)) = self.expression(projected, 4096)? {
                let local = Self::new(support);
                let local_care = local.compile(&projected_expression)?;
                return local
                    .reduce_under_with_support(expression, local_care, support, node_limit);
            }
            projected
        };
        let value = self.compile(expression)?;
        let mut best = (expression.clone(), expression.node_count());
        let mut best_rank = (best.1, expression.symbols().len());

        let constrained = self.constrain(value, care)?;
        if let Some(candidate) = self.expression(constrained, node_limit)? {
            let rank = (candidate.1, candidate.0.symbols().len());
            if rank < best_rank && self.equivalent_under(care, value, constrained)? {
                best_rank = rank;
                best = candidate;
            }
        }
        if expression.node_count() >= 8 {
            if let Some(candidate) = self.implicant_cover(expression, care, value, node_limit)? {
                let rank = (candidate.node_count(), candidate.symbols().len());
                if rank <= best_rank {
                    best_rank = rank;
                    best = (candidate, rank.0);
                }
            }
        }
        let mut candidates = expression.subexpressions(256);
        candidates.extend([BoolExpr::False, BoolExpr::True]);
        let complements = candidates
            .iter()
            .cloned()
            .map(BoolExpr::not)
            .collect::<Vec<_>>();
        candidates.extend(complements);
        for candidate in candidates {
            let size = candidate.node_count();
            let rank = (size, candidate.symbols().len());
            if rank >= best_rank || size > node_limit {
                continue;
            }
            let candidate_value = self.compile(&candidate)?;
            if self.equivalent_under(care, value, candidate_value)? {
                best_rank = rank;
                best = (candidate, size);
            }
        }
        if expression.node_count() >= 8 || support.len() <= 16 {
            let candidate = match self.care_set_cover(care, value, support, node_limit) {
                Ok(candidate) => candidate,
                Err(error) if error.is_resource_limit() => None,
                Err(error) => return Err(error),
            };
            if let Some(candidate) = candidate {
                let rank = (candidate.node_count(), candidate.symbols().len());
                if rank <= best_rank {
                    best = (candidate, rank.0);
                }
            }
        }
        Ok((best.1 <= node_limit).then_some(best))
    }

    fn care_set_cover(
        &self,
        care: Bdd,
        value: Bdd,
        support: &BTreeSet<BoolVariable>,
        node_limit: usize,
    ) -> Result<Option<BoolExpr>, BddError> {
        const CUBE_LIMIT: usize = 256;
        const EXPANSION_LIMIT: usize = 2048;

        let on_set = self.and(care, value)?;
        if on_set.is_false() {
            return Ok(Some(BoolExpr::False));
        }
        let off_set = self.and(care, self.not(value)?)?;
        if off_set.is_false() {
            return Ok(Some(BoolExpr::True));
        }

        let expansion_limit = node_limit
            .saturating_mul(8)
            .max(node_limit)
            .min(EXPANSION_LIMIT);
        let Some((on_expression, _)) = self.expression(on_set, expansion_limit)? else {
            return Ok(None);
        };
        let Some(mut cubes) = DnfBuilder::new(CUBE_LIMIT).build(&on_expression) else {
            return Ok(None);
        };
        for variable in support {
            for polarity in [true, false] {
                let literal = BoolExpr::Symbol(variable.clone());
                let literal = if polarity {
                    literal
                } else {
                    BoolExpr::not(literal)
                };
                let literal_value = self.compile(&literal)?;
                if !self.and(on_set, literal_value)?.is_false()
                    && self.and(off_set, literal_value)?.is_false()
                {
                    cubes.push(Cube::from([(variable.clone(), polarity)]));
                }
            }
        }
        Self::absorb_cubes(&mut cubes);

        for cube in &mut cubes {
            let mut literals = cube.keys().cloned().collect::<Vec<_>>();
            literals.sort_by_key(|variable| (support.contains(variable), variable.clone()));
            for literal in literals {
                let Some(polarity) = cube.remove(&literal) else {
                    continue;
                };
                let candidate = self.compile(&Self::cube_expression(cube))?;
                if !self.and(candidate, off_set)?.is_false() {
                    cube.insert(literal, polarity);
                }
            }
        }
        cubes.retain(|cube| cube.keys().all(|variable| support.contains(variable)));
        Self::absorb_cubes(&mut cubes);
        self.absorb_cubes_under(&mut cubes, on_set)?;
        if cubes.is_empty() {
            return Ok(None);
        }

        let candidate = Self::factored_cover(&cubes, 0);
        if candidate.node_count() > node_limit {
            return Ok(None);
        }
        let candidate_value = self.compile(&candidate)?;
        Ok(self
            .equivalent_under(care, value, candidate_value)?
            .then_some(candidate))
    }

    fn absorb_cubes_under(&self, cubes: &mut Vec<Cube>, on_set: Bdd) -> Result<(), BddError> {
        cubes.sort_by(|left, right| {
            left.len()
                .cmp(&right.len())
                .then_with(|| left.iter().cmp(right.iter()))
        });
        let mut covered = self.falsity();
        let mut irredundant = Vec::new();
        for cube in std::mem::take(cubes) {
            let value = self.compile(&Self::cube_expression(&cube))?;
            let contribution = self.and(on_set, value)?;
            if contribution.is_false() || self.implies_bdd(contribution, covered)? {
                continue;
            }
            covered = self.or(covered, contribution)?;
            irredundant.push(cube);
        }
        *cubes = irredundant;
        Ok(())
    }

    fn implicant_cover(
        &self,
        expression: &BoolExpr,
        care: Bdd,
        value: Bdd,
        node_limit: usize,
    ) -> Result<Option<BoolExpr>, BddError> {
        const CUBE_LIMIT: usize = 64;
        let Some(mut cubes) = DnfBuilder::new(CUBE_LIMIT).build(expression) else {
            return Ok(None);
        };
        if cubes.is_empty() {
            return Ok(Some(BoolExpr::False));
        }
        Self::absorb_cubes(&mut cubes);
        for cube in &mut cubes {
            let literals = cube.keys().cloned().collect::<Vec<_>>();
            for literal in literals {
                let Some(polarity) = cube.remove(&literal) else {
                    continue;
                };
                let candidate = Self::cube_expression(cube);
                let candidate = self.compile(&candidate)?;
                let premise = self.and(care, candidate)?;
                if !self.implies_bdd(premise, value)? {
                    cube.insert(literal, polarity);
                }
            }
        }
        Self::absorb_cubes(&mut cubes);
        let candidate = Self::factored_cover(&cubes, 0);
        if candidate.node_count() > node_limit {
            return Ok(None);
        }
        let candidate_value = self.compile(&candidate)?;
        Ok(self
            .equivalent_under(care, value, candidate_value)?
            .then_some(candidate))
    }

    fn cube_expression(cube: &Cube) -> BoolExpr {
        let mut literals = cube.iter().collect::<Vec<_>>();
        literals.sort_by(
            |(left_variable, left_polarity), (right_variable, right_polarity)| {
                right_polarity
                    .cmp(left_polarity)
                    .then_with(|| left_variable.cmp(right_variable))
            },
        );
        BoolExpr::and(
            literals
                .into_iter()
                .map(|(variable, polarity)| {
                    let literal = BoolExpr::Symbol(variable.clone());
                    if *polarity {
                        literal
                    } else {
                        BoolExpr::not(literal)
                    }
                })
                .collect(),
        )
    }

    fn absorb_cubes(cubes: &mut Vec<Cube>) {
        cubes.sort_by(|left, right| {
            left.len()
                .cmp(&right.len())
                .then_with(|| left.iter().cmp(right.iter()))
        });
        let mut irredundant = Vec::<Cube>::new();
        for cube in std::mem::take(cubes) {
            if irredundant
                .iter()
                .any(|candidate| Self::cube_contains(&cube, candidate))
            {
                continue;
            }
            irredundant.push(cube);
        }
        *cubes = irredundant;
    }

    fn cube_contains(cube: &Cube, subset: &Cube) -> bool {
        subset
            .iter()
            .all(|(variable, polarity)| cube.get(variable) == Some(polarity))
    }

    fn factored_cover(cubes: &[Cube], depth: usize) -> BoolExpr {
        if cubes.is_empty() {
            return BoolExpr::False;
        }
        if cubes.iter().any(BTreeMap::is_empty) {
            return BoolExpr::True;
        }
        let flat = BoolExpr::or(cubes.iter().map(Self::cube_expression).collect());
        if cubes.len() < 2 || depth >= 32 {
            return flat;
        }

        let mut frequencies = BTreeMap::<(BoolVariable, bool), usize>::new();
        for cube in cubes {
            for (variable, polarity) in cube {
                *frequencies
                    .entry((variable.clone(), *polarity))
                    .or_default() += 1;
            }
        }
        let Some(((variable, polarity), count)) = frequencies
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .max_by_key(|(_, count)| *count)
        else {
            return flat;
        };

        let mut containing = Vec::with_capacity(count);
        let mut remaining = Vec::with_capacity(cubes.len().saturating_sub(count));
        for cube in cubes {
            if cube.get(&variable) == Some(&polarity) {
                let mut reduced = cube.clone();
                reduced.remove(&variable);
                containing.push(reduced);
            } else {
                remaining.push(cube.clone());
            }
        }
        Self::absorb_cubes(&mut containing);
        Self::absorb_cubes(&mut remaining);
        let literal = BoolExpr::Symbol(variable);
        let literal = if polarity {
            literal
        } else {
            BoolExpr::not(literal)
        };
        let common = BoolExpr::and(vec![literal, Self::factored_cover(&containing, depth + 1)]);
        let factored = if remaining.is_empty() {
            common
        } else {
            BoolExpr::or(vec![Self::factored_cover(&remaining, depth + 1), common])
        };
        if factored.node_count() < flat.node_count() {
            factored
        } else {
            flat
        }
    }

    pub fn implies_bdd(&self, premise: Bdd, consequence: Bdd) -> Result<bool, BddError> {
        let mut manager = self.manager.borrow_mut();
        manager.begin_operation();
        manager.implies(premise, consequence)
    }

    pub fn expression(
        &self,
        value: Bdd,
        node_limit: usize,
    ) -> Result<Option<(BoolExpr, usize)>, BddError> {
        self.manager.borrow().validate(value)?;
        let mut pending = vec![ExpandTask::Visit(value)];
        let mut results = Vec::new();
        let mut memo = BTreeMap::<Bdd, (BoolExpr, usize)>::new();
        while let Some(task) = pending.pop() {
            match task {
                ExpandTask::Visit(value) => {
                    if let Some(result) = memo.get(&value) {
                        results.push(result.clone());
                        continue;
                    }
                    if value.is_false() {
                        if node_limit == 0 {
                            return Ok(None);
                        }
                        results.push((BoolExpr::False, 1));
                        continue;
                    }
                    if value.is_true() {
                        if node_limit == 0 {
                            return Ok(None);
                        }
                        results.push((BoolExpr::True, 1));
                        continue;
                    }
                    let node = self.manager.borrow().validate(value)?;
                    pending.push(ExpandTask::Build {
                        value,
                        variable: node.variable,
                    });
                    pending.push(ExpandTask::Visit(node.high));
                    pending.push(ExpandTask::Visit(node.low));
                }
                ExpandTask::Build { value, variable } => {
                    let (high, high_nodes) = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    let (low, low_nodes) = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    let symbol = BoolExpr::Symbol(
                        self.variables_by_index
                            .get(variable)
                            .cloned()
                            .ok_or(BddError::InvalidHandle(variable))?,
                    );
                    let (expression, nodes) = match (&low, &high) {
                        (BoolExpr::False, BoolExpr::True) => (symbol, 1),
                        (BoolExpr::True, BoolExpr::False) => (BoolExpr::not(symbol), 2),
                        (BoolExpr::False, _) => (
                            BoolExpr::and(vec![symbol, high]),
                            high_nodes.saturating_add(2),
                        ),
                        (_, BoolExpr::False) => (
                            BoolExpr::and(vec![BoolExpr::not(symbol), low]),
                            low_nodes.saturating_add(3),
                        ),
                        (BoolExpr::True, _) => (
                            BoolExpr::or(vec![BoolExpr::not(symbol), high]),
                            high_nodes.saturating_add(3),
                        ),
                        (_, BoolExpr::True) => {
                            (BoolExpr::or(vec![symbol, low]), low_nodes.saturating_add(2))
                        }
                        _ => (
                            BoolExpr::or(vec![
                                BoolExpr::and(vec![BoolExpr::not(symbol.clone()), low]),
                                BoolExpr::and(vec![symbol, high]),
                            ]),
                            low_nodes.saturating_add(high_nodes).saturating_add(6),
                        ),
                    };
                    if nodes > node_limit {
                        return Ok(None);
                    }
                    memo.insert(value, (expression.clone(), nodes));
                    results.push((expression, nodes));
                }
            }
        }
        if results.len() != 1 {
            return Err(BddError::MalformedEvaluation);
        }
        Ok(results.pop())
    }

    fn reserve_expression(&self, compounds: usize) -> Result<(), BddError> {
        if compounds.saturating_add(self.compiled_literals.get()) >= self.limits.cache_entries {
            return Err(BddError::ResourceLimit {
                resource: BddResource::CacheEntries,
                limit: self.limits.cache_entries,
            });
        }
        Ok(())
    }

    fn record_expression(&self, expression: &BoolExpr, value: Bdd) -> Result<(), BddError> {
        let mut expressions = self.expressions.borrow_mut();
        // A compound is scheduled only after its cache miss; a strict child
        // cannot equal that compound, and siblings run to completion in order.
        self.reserve_expression(expressions.len())?;
        expressions.insert(expression.clone(), value);
        Ok(())
    }
}

enum ExpandTask {
    Visit(Bdd),
    Build { value: Bdd, variable: usize },
}

enum CompileTask<'a> {
    Visit(&'a BoolExpr),
    Combine {
        expression: &'a BoolExpr,
        operation: CompileOp,
        operands: usize,
    },
}

#[derive(Clone, Copy)]
enum CompileOp {
    Not,
    And,
    Or,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(name: &str) -> BoolExpr {
        BoolExpr::symbol(name)
    }

    #[test]
    fn boolean_identities_share_canonical_handles() {
        let variables = ["a", "b"]
            .into_iter()
            .map(|name| BoolVariable::Named(name.to_string()))
            .collect();
        let context = BddContext::new(&variables);
        let a = sym("a");
        let b = sym("b");

        assert!(context.are_equivalent(&a, &a).unwrap());
        assert!(!context
            .are_equivalent(&a, &BoolExpr::not(a.clone()))
            .unwrap());
        assert!(context
            .are_equivalent(
                &BoolExpr::not(BoolExpr::and(vec![a.clone(), b.clone()])),
                &BoolExpr::or(vec![BoolExpr::not(a.clone()), BoolExpr::not(b.clone())]),
            )
            .unwrap());
        assert!(context
            .are_equivalent(
                &BoolExpr::or(vec![a.clone(), BoolExpr::and(vec![a.clone(), b.clone()])]),
                &a,
            )
            .unwrap());
    }

    #[test]
    fn existential_abstraction_forgets_quantified_variables() {
        let variables = ["a", "b"]
            .into_iter()
            .map(|name| BoolVariable::Named(name.to_string()))
            .collect();
        let context = BddContext::new(&variables);
        let a = sym("a");
        let b = sym("b");
        let conjunction = context.compile(&BoolExpr::and(vec![a.clone(), b])).unwrap();
        let quantified = context
            .exists(
                conjunction,
                &BTreeSet::from([BoolVariable::Named("b".to_string())]),
            )
            .unwrap();

        assert_eq!(quantified, context.compile(&a).unwrap());
    }

    #[test]
    fn care_set_reduction_uses_implied_subexpressions() {
        let variables = ["a", "b"]
            .into_iter()
            .map(|name| BoolVariable::Named(name.to_string()))
            .collect();
        let context = BddContext::new(&variables);
        let a = sym("a");
        let b = sym("b");
        let care = context
            .compile(&BoolExpr::or(vec![a.clone(), BoolExpr::not(b.clone())]))
            .unwrap();
        let original = BoolExpr::and(vec![a, b.clone()]);
        let (reduced, _) = context.reduce_under(&original, care, 32).unwrap().unwrap();

        assert_eq!(reduced, b);
    }

    #[test]
    fn implicant_cover_factors_mux_shaped_conditions() {
        let variables = ["c", "d", "f"]
            .into_iter()
            .map(|name| BoolVariable::Named(name.to_string()))
            .collect();
        let context = BddContext::new(&variables);
        let c = sym("c");
        let d = sym("d");
        let f = sym("f");
        let original = BoolExpr::or(vec![
            BoolExpr::and(vec![f.clone(), BoolExpr::or(vec![c.clone(), d.clone()])]),
            BoolExpr::and(vec![BoolExpr::not(f.clone()), d.clone()]),
        ]);
        let (reduced, _) = context
            .reduce_under(&original, context.truth(), 64)
            .unwrap()
            .unwrap();

        assert_eq!(reduced, BoolExpr::or(vec![d, BoolExpr::and(vec![c, f])]));
    }

    #[test]
    fn care_set_cover_can_use_an_equivalent_support_atom() {
        let variables = ["s0", "s1", "s2"]
            .into_iter()
            .map(|name| BoolVariable::Named(name.to_string()))
            .collect::<BTreeSet<_>>();
        let context = BddContext::new(&variables);
        let s0 = sym("s0");
        let s1 = sym("s1");
        let s2 = sym("s2");
        let care = context
            .compile(&BoolExpr::and(vec![
                BoolExpr::or(vec![s0.clone(), s1.clone(), s2.clone()]),
                BoolExpr::or(vec![BoolExpr::not(s0.clone()), BoolExpr::not(s1.clone())]),
                BoolExpr::or(vec![BoolExpr::not(s0.clone()), BoolExpr::not(s2.clone())]),
                BoolExpr::or(vec![BoolExpr::not(s1.clone()), BoolExpr::not(s2.clone())]),
            ]))
            .unwrap();
        let original = BoolExpr::and(vec![BoolExpr::not(s0), BoolExpr::not(s1)]);
        let (reduced, _) = context
            .reduce_under_with_support(&original, care, &variables, 32)
            .unwrap()
            .unwrap();

        assert_eq!(reduced, s2);
    }

    #[test]
    fn care_set_cover_recovers_a_finite_domain_tail() {
        let variables = ["l0", "l1", "l2", "l3", "l4", "l5", "p3", "q4", "q5"]
            .into_iter()
            .map(|name| BoolVariable::Named(name.to_string()))
            .collect::<BTreeSet<_>>();
        let context = BddContext::new(&variables);
        let labels = (0..=5)
            .map(|value| sym(&format!("l{value}")))
            .collect::<Vec<_>>();
        let mut theory = vec![BoolExpr::or(labels.clone())];
        for left in 0..labels.len() {
            for right in left + 1..labels.len() {
                theory.push(BoolExpr::or(vec![
                    BoolExpr::not(labels[left].clone()),
                    BoolExpr::not(labels[right].clone()),
                ]));
            }
        }
        let care = context.compile(&BoolExpr::and(theory)).unwrap();
        let original = BoolExpr::or(vec![
            BoolExpr::and(
                (0..=4)
                    .map(|value| BoolExpr::not(labels[value].clone()))
                    .collect(),
            ),
            BoolExpr::and(vec![
                sym("q5"),
                BoolExpr::or(vec![
                    BoolExpr::and(
                        (0..=3)
                            .map(|value| BoolExpr::not(labels[value].clone()))
                            .collect(),
                    ),
                    BoolExpr::and(vec![sym("q4"), sym("p3")]),
                ]),
            ]),
        ]);
        let (reduced, _) = context
            .reduce_under_with_support(&original, care, &variables, 128)
            .unwrap()
            .unwrap();

        assert_eq!(
            reduced,
            BoolExpr::or(vec![
                sym("l5"),
                BoolExpr::and(vec![sym("l4"), sym("q5")]),
                BoolExpr::and(vec![sym("p3"), sym("q4"), sym("q5")]),
            ])
        );
    }
}

#[cfg(test)]
#[path = "../../tests/internal/bdd_apply.rs"]
mod apply_tests;
