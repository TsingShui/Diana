use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};

use super::UnmatchedGroup;
use super::index::{Index, Method};
use super::{alignment, inline};

// A common feature is useful within a method pair but cannot justify comparing
// every member of its Cartesian product. Exact shape groups survive as groups.
const MAX_POSTING_PRODUCT: usize = 64;
const MAX_NEIGHBOR_PRODUCT: usize = 128;
const SAMPLED_GRAMS: usize = 12;

#[derive(Clone, Copy, Default)]
pub(super) struct Evidence {
    identity: bool,
    shape: bool,
    exact: bool,
    unique_body: bool,
    same_owner: bool,
    other_owner: bool,
    anchors: u16,
    grams: u16,
    owner_grams: u8,
    neighbors: u16,
    callers: u16,
    inline_caller: bool,
    embedded: bool,
}

impl Evidence {
    fn observe_call_edge(&mut self, from_caller: bool) {
        self.neighbors = self.neighbors.saturating_add(1);
        if from_caller {
            self.callers = self.callers.saturating_add(1);
        }
    }

    pub(super) fn names(self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.identity {
            names.push("same_dex_signature");
        }
        if self.exact {
            names.push("same_normalized_body");
        }
        if self.shape {
            names.push("same_instruction_shape");
        }
        if self.anchors != 0 {
            names.push("shared_rare_reference");
        }
        if self.grams != 0 || self.owner_grams != 0 {
            names.push("shared_instruction_fragments");
        }
        if self.same_owner {
            names.push("paired_owner");
        }
        if self.inline_caller {
            names.push("preserved_inline_caller");
        }
        if self.neighbors != 0 {
            names.push("paired_call_neighbors");
        }
        names
    }
}

pub(super) struct Pair {
    pub old: usize,
    pub new: usize,
    pub evidence: Evidence,
}

pub(super) struct Matching {
    pub pairs: Vec<Pair>,
    pub old_to_new: Vec<Option<usize>>,
    new_to_old: Vec<Option<usize>>,
    pub callees: HashMap<String, String>,
    pub candidate_pairs: usize,
    pending: Vec<(usize, usize)>,
    owners: HashMap<String, String>,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Feature {
    Identity(u64),
    Body(u64),
    Shape(u64),
    Anchor(u64),
    Gram(u64),
}

#[derive(Clone, Copy)]
struct Posting {
    feature: Feature,
    is_new: bool,
    method: usize,
}

struct Profile {
    prototype: u64,
    return_type: u64,
    shape: Option<u64>,
    size: usize,
    independent_body: bool,
    has_anchors: bool,
    declaration: bool,
    special: u8,
}

impl Profile {
    fn permits_candidate(&self, other: &Self) -> bool {
        self.special == other.special
            && (self.prototype == other.prototype
                || self.special == 0 && self.return_type == other.return_type)
    }
}

fn digest<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hash = DefaultHasher::new();
    value.hash(&mut hash);
    hash.finish()
}

fn prototype_shape(prototype: &str) -> u64 {
    let mut hash = DefaultHasher::new();
    let mut object = false;
    for byte in prototype.bytes() {
        if !object {
            byte.hash(&mut hash);
        }
        if byte == b';' {
            object = false;
        }
        if byte == b'L' {
            object = true;
        }
    }
    hash.finish()
}

struct Grams {
    smallest: [u64; SAMPLED_GRAMS],
    last: [u64; 2],
    count: usize,
}

impl Grams {
    fn new() -> Self {
        Self {
            smallest: [u64::MAX; SAMPLED_GRAMS],
            last: [0; 2],
            count: 0,
        }
    }

    fn push(&mut self, token: u64) {
        if self.count >= 2 {
            let gram = digest(&[self.last[0], self.last[1], token]);
            if let Err(position) = self.smallest.binary_search(&gram)
                && position < self.smallest.len()
            {
                let end = self.smallest.len() - 1;
                self.smallest.copy_within(position..end, position + 1);
                self.smallest[position] = gram;
            }
        }
        self.last = [self.last[1], token];
        self.count += 1;
    }

    fn values(&self) -> &[u64] {
        &self.smallest[..self.smallest.partition_point(|&value| value != u64::MAX)]
    }

    fn of_method(method: &Method) -> Self {
        let mut grams = Self::new();
        if let Some(body) = &method.body {
            for op in &body.code {
                grams.push(op_shape(op));
            }
        }
        grams
    }

    fn shared_in_owner(before: &Method, after: &Method) -> u8 {
        let a = Self::of_method(before);
        let b = Self::of_method(after);
        let a = a.values();
        let b = b.values();
        let shared = a
            .iter()
            .filter(|gram| b.binary_search(gram).is_ok())
            .count();
        if shared >= 4 && shared * 2 >= a.len().min(b.len()) {
            shared as u8
        } else {
            0
        }
    }
}

fn profile_and_postings(
    methods: &[Method],
    identities: &[String],
    is_new: bool,
    postings: &mut Vec<Posting>,
) -> Vec<Profile> {
    methods
        .iter()
        .enumerate()
        .map(|(method, entry)| {
            let mut push = |feature| {
                postings.push(Posting {
                    feature,
                    is_new,
                    method,
                });
            };
            let prototype = prototype_shape(&entry.prototype);
            let return_type = prototype_shape(
                entry
                    .prototype
                    .rsplit_once(')')
                    .map_or("", |(_, result)| result),
            );
            push(Feature::Identity(digest(&identities[method])));
            let Some(body) = &entry.body else {
                return Profile {
                    prototype,
                    return_type,
                    shape: None,
                    size: 0,
                    independent_body: false,
                    has_anchors: false,
                    declaration: entry.issues.is_empty(),
                    special: special_method(entry),
                };
            };

            let mut hash = DefaultHasher::new();
            let mut grams = Grams::new();
            for op in &body.code {
                let token = op_shape(op);
                token.hash(&mut hash);
                grams.push(token);
            }
            let shape = hash.finish();
            push(Feature::Shape(digest(&(prototype, shape))));
            if entry.issues.is_empty() {
                let mut body_key = DefaultHasher::new();
                prototype.hash(&mut body_key);
                body.frame[1].hash(&mut body_key);
                body.operands.hash(&mut body_key);
                body.protection.hash(&mut body_key);
                for op in &body.code {
                    op.opcode.hash(&mut body_key);
                    op.end.hash(&mut body_key);
                }
                push(Feature::Body(body_key.finish()));
            }
            for &anchor in &body.anchors {
                push(Feature::Anchor(anchor));
            }
            for &gram in grams.values() {
                push(Feature::Gram(gram));
            }
            Profile {
                prototype,
                return_type,
                shape: Some(shape),
                size: grams.count,
                independent_body: !body.has_owner_member_ref(),
                has_anchors: !body.anchors.is_empty(),
                declaration: false,
                special: special_method(entry),
            }
        })
        .collect()
}

fn op_shape(op: &super::index::Op) -> u64 {
    u64::from(op.shape) | (u64::from(op.opcode) << 32)
}

type Candidates = HashMap<(usize, usize), Evidence>;

fn candidates(
    mut postings: Vec<Posting>,
    before: &[Profile],
    after: &[Profile],
    identities: &(Vec<String>, Vec<String>),
) -> Candidates {
    postings.sort_unstable_by_key(|row| (row.feature, row.is_new, row.method));
    let mut edges = HashMap::<_, Evidence>::new();
    for group in postings.chunk_by(|a, b| a.feature == b.feature) {
        let split = group.partition_point(|row| !row.is_new);
        let (old, new) = group.split_at(split);
        if old.is_empty() || new.is_empty() {
            continue;
        }
        if old.len().saturating_mul(new.len()) > MAX_POSTING_PRODUCT {
            continue;
        }
        for a in old {
            for b in new {
                let (i, j) = (a.method, b.method);
                if !before[i].permits_candidate(&after[j]) {
                    continue;
                }
                // Prototype drift requires a shared anchor. Anchor postings
                // precede grams, so a gram can refine an existing candidate.
                if before[i].prototype != after[j].prototype
                    && !matches!(a.feature, Feature::Anchor(_))
                    && !edges.contains_key(&(i, j))
                {
                    continue;
                }
                let edge = edges.entry((i, j)).or_default();
                match a.feature {
                    Feature::Identity(_) => edge.identity = identities.0[i] == identities.1[j],
                    Feature::Body(_) => edge.unique_body = old.len() == 1 && new.len() == 1,
                    Feature::Shape(_) => edge.shape = true,
                    Feature::Anchor(_) => edge.anchors = edge.anchors.saturating_add(1),
                    Feature::Gram(_) => edge.grams = edge.grams.saturating_add(1),
                }
            }
        }
    }
    edges
}

struct Calls {
    callers: Vec<Vec<usize>>,
    callees: Vec<Vec<usize>>,
    unique: Vec<bool>,
}

fn special_method(method: &Method) -> u8 {
    match method.name.as_str() {
        "<init>" => 1,
        "<clinit>" => 2,
        _ => 0,
    }
}

fn calls(methods: &[Method], identities: &[String]) -> Calls {
    let mut lookup = HashMap::with_capacity(methods.len());
    for (i, key) in identities.iter().enumerate() {
        lookup
            .entry(key.as_str())
            .and_modify(|id| *id = None)
            .or_insert(Some(i));
    }
    let mut callers = vec![Vec::new(); methods.len()];
    let mut callees = vec![Vec::new(); methods.len()];
    for (i, method) in methods.iter().enumerate() {
        if let Some(body) = &method.body {
            let row = &mut callees[i];
            row.extend(
                body.call_names()
                    .filter_map(|name| lookup.get(name)?.as_ref().copied()),
            );
            row.sort_unstable();
            row.dedup();
            for &callee in row.iter() {
                callers[callee].push(i);
            }
        }
    }
    let unique = identities
        .iter()
        .map(|name| lookup[name.as_str()].is_some())
        .collect();
    Calls {
        callers,
        callees,
        unique,
    }
}

fn exact_body(old: &Method, new: &Method) -> bool {
    if !old.issues.is_empty() || !new.issues.is_empty() {
        return false;
    }
    match (&old.body, &new.body) {
        (Some(a), Some(b)) => {
            a.frame[1] == b.frame[1]
                && a.protection == b.protection
                && a.operands == b.operands
                && a.code.len() == b.code.len()
                && a.code
                    .iter()
                    .zip(&b.code)
                    .all(|(i, j)| i.opcode == j.opcode && i.end == j.end)
        }
        (None, None) => false,
        _ => false,
    }
}

fn same_shape(old: &Method, new: &Method) -> bool {
    match (&old.body, &new.body) {
        (Some(a), Some(b)) => {
            a.code.len() == b.code.len()
                && a.code
                    .iter()
                    .zip(&b.code)
                    .all(|(x, y)| x.opcode == y.opcode && x.shape == y.shape)
        }
        _ => false,
    }
}

fn statically_check(
    edge: &mut Evidence,
    before: &Method,
    after: &Method,
    old_profile: &Profile,
    new_profile: &Profile,
) {
    edge.shape = old_profile.shape.is_some()
        && old_profile.shape == new_profile.shape
        && same_shape(before, after);
    if edge.shape || edge.identity {
        edge.exact = exact_body(before, after);
    }
}

fn score(edge: Evidence, old: &Profile, new: &Profile, anchors_only: bool) -> u32 {
    if edge.embedded {
        return 0;
    }
    if edge.inline_caller {
        return 220;
    }
    let size = old.size.min(new.size);
    if old.prototype != new.prototype {
        // A calling convention change needs a matched call site plus body
        // evidence. Reference or opcode similarity alone joins overloads.
        if anchors_only
            || edge.callers == 0
            || edge.anchors == 0
            || !(edge.shape || edge.anchors >= 2 && edge.grams >= 2)
        {
            return 0;
        }
        return 39
            + u32::from(edge.shape) * 15
            + u32::from(edge.anchors.min(4)) * 9
            + u32::from(edge.callers.min(4)) * 8;
    }
    if edge.exact && edge.shape && edge.anchors > 0 {
        return 140
            + u32::from(edge.anchors.min(4)) * 4
            + u32::from(edge.same_owner) * 16
            + u32::from(edge.neighbors.min(4)) * 5;
    }
    if anchors_only {
        return if edge.anchors >= 2 && edge.grams >= 4 {
            100 + u32::from(edge.anchors.min(5)) * 4 + u32::from(edge.grams.min(7)) * 3
        } else if edge.shape && size >= 4 && edge.anchors > 0 {
            65 + u32::from(edge.anchors.min(5)) * 9 + u32::from(edge.grams.min(7)) * 3
        } else {
            0
        };
    }
    let corroborated_move = edge.shape && edge.callers >= 2;
    if edge.other_owner && edge.anchors == 0 && !corroborated_move {
        return 0;
    }
    let independent_exact =
        edge.exact && edge.unique_body && size >= 2 && old.independent_body && new.independent_body;
    if independent_exact {
        return 130 + u32::from(edge.same_owner) * 12 + u32::from(edge.callers.min(4)) * 9;
    }
    if edge.same_owner && old.declaration && new.declaration {
        return 75;
    }
    if edge.exact && (edge.same_owner || edge.neighbors > 0) {
        return 100 + u32::from(edge.same_owner) * 14 + u32::from(edge.neighbors.min(4)) * 6;
    }
    if size < 4 && old.has_anchors && new.has_anchors && edge.anchors == 0 && edge.neighbors == 0 {
        return 0;
    }
    let grams = edge.grams.max(u16::from(edge.owner_grams));
    let distinctive_grams = size >= 12
        && old.size.max(new.size) <= size.saturating_mul(2)
        && grams >= if edge.same_owner { 2 } else { 3 };
    if !(edge.shape && (edge.anchors > 0 || edge.neighbors > 0 || edge.same_owner)
        || edge.anchors > 0 && (edge.grams > 0 || edge.identity || edge.neighbors > 0)
        || edge.neighbors > 0 && (edge.grams > 0 || edge.identity)
        || edge.identity && edge.grams >= 2
        || distinctive_grams)
    {
        return 0;
    }
    u32::from(edge.identity && edge.grams > 0) * 5
        + u32::from(edge.shape) * 35
        + u32::from(edge.anchors.min(4)) * 16
        + u32::from(grams.min(7)) * 7
        + u32::from(edge.neighbors.min(4)) * 10
        + u32::from(edge.same_owner) * 20
}

#[derive(Clone, Copy, Default)]
struct Choice {
    target: usize,
    best: u32,
    second: u32,
}

impl Choice {
    fn consider(&mut self, target: usize, value: u32) {
        if value > self.best {
            self.second = self.best;
            self.best = value;
            self.target = target;
        } else {
            self.second = self.second.max(value);
        }
    }

    fn clear(&self) -> bool {
        self.best >= 20 && self.best - self.second >= 10
    }
}

struct Search<'a> {
    old: &'a [Method],
    new: &'a [Method],
    before: Vec<Profile>,
    after: Vec<Profile>,
    old_calls: Calls,
    new_calls: Calls,
    edges: Candidates,
    old_edges: Vec<Vec<usize>>,
    new_edges: Vec<Vec<usize>>,
    old_dirty: Vec<usize>,
    new_dirty: Vec<usize>,
    old_choice: Vec<Choice>,
    new_choice: Vec<Choice>,
    old_to_new: Vec<Option<usize>>,
    new_to_old: Vec<Option<usize>>,
    old_owner: HashMap<&'a str, Option<&'a str>>,
    new_owner: HashMap<&'a str, Option<&'a str>>,
    anchors_only: bool,
}

impl Search<'_> {
    fn owner_evidence(&self, i: usize, j: usize, mut edge: Evidence) -> Evidence {
        let (before, after) = (&self.old[i], &self.new[j]);
        let forward = self.old_owner.get(before.owner.as_str()).copied().flatten();
        let backward = self.new_owner.get(after.owner.as_str()).copied().flatten();
        edge.same_owner =
            forward == Some(after.owner.as_str()) && backward == Some(before.owner.as_str());
        edge.other_owner = forward.is_some_and(|owner| owner != after.owner)
            || backward.is_some_and(|owner| owner != before.owner);
        edge
    }

    fn rank(&self, index: usize, is_new: bool) -> Choice {
        let (neighbors, paired) = if is_new {
            (&self.new_edges[index], &self.old_to_new)
        } else {
            (&self.old_edges[index], &self.new_to_old)
        };
        let mut choice = Choice::default();
        for &other in neighbors {
            if paired[other].is_some() {
                continue;
            }
            let (i, j) = if is_new {
                (other, index)
            } else {
                (index, other)
            };
            let edge = self.owner_evidence(i, j, self.edges[&(i, j)]);
            let value = score(edge, &self.before[i], &self.after[j], self.anchors_only);
            choice.consider(other, value);
        }
        choice
    }

    fn take(&mut self, (i, j): (usize, usize)) -> Pair {
        self.old_to_new[i] = Some(j);
        self.new_to_old[j] = Some(i);
        self.new_dirty.extend(&self.old_edges[i]);
        self.old_dirty.extend(&self.new_edges[j]);
        Pair {
            old: i,
            new: j,
            evidence: self.owner_evidence(i, j, self.edges[&(i, j)]),
        }
    }

    fn seed_owners(&mut self, pairs: &[Pair]) {
        for pair in pairs {
            let before = self.old[pair.old].owner.as_str();
            let after = self.new[pair.new].owner.as_str();
            Self::record_owner(&mut self.old_owner, before, after);
            Self::record_owner(&mut self.new_owner, after, before);
        }

        let mut old_members = HashMap::<_, Vec<usize>>::new();
        let mut new_members = HashMap::<_, Vec<usize>>::new();
        for (i, method) in self.old.iter().enumerate() {
            if self.old_to_new[i].is_none() && self.old_owner.contains_key(method.owner.as_str()) {
                old_members
                    .entry(method.owner.as_str())
                    .or_default()
                    .push(i);
            }
        }
        for (j, method) in self.new.iter().enumerate() {
            if self.new_to_old[j].is_none() && self.new_owner.contains_key(method.owner.as_str()) {
                new_members
                    .entry(method.owner.as_str())
                    .or_default()
                    .push(j);
            }
        }
        for (owner, before) in old_members {
            let Some(after) = self.old_owner.get(owner).copied().flatten() else {
                continue;
            };
            if self.new_owner.get(after).copied().flatten() != Some(owner) {
                continue;
            }
            let Some(after) = new_members.get(after) else {
                continue;
            };
            if before.len().saturating_mul(after.len()) > MAX_NEIGHBOR_PRODUCT {
                continue;
            }
            for i in before {
                for &j in after {
                    if self.before[i].prototype != self.after[j].prototype
                        || self.before[i].special != self.after[j].special
                    {
                        continue;
                    }
                    let edge = self.edges.entry((i, j)).or_insert_with(|| {
                        self.old_edges[i].push(j);
                        self.new_edges[j].push(i);
                        let mut edge = Evidence::default();
                        statically_check(
                            &mut edge,
                            &self.old[i],
                            &self.new[j],
                            &self.before[i],
                            &self.after[j],
                        );
                        edge
                    });
                    if !edge.shape && edge.grams < 4 {
                        let (a, b) = (&self.before[i], &self.after[j]);
                        let size = a.size.min(b.size);
                        if size >= 10 && a.size.max(b.size) <= size.saturating_mul(2) {
                            edge.owner_grams = Grams::shared_in_owner(&self.old[i], &self.new[j]);
                        }
                    }
                }
            }
        }
        self.old_dirty.extend(0..self.old.len());
        self.new_dirty.extend(0..self.new.len());
    }

    fn record_owner<'a>(map: &mut HashMap<&'a str, Option<&'a str>>, key: &'a str, value: &'a str) {
        map.entry(key)
            .and_modify(|old| {
                if *old != Some(value) {
                    *old = None;
                }
            })
            .or_insert(Some(value));
    }

    fn expand(&mut self, i: usize, j: usize, outgoing: bool) {
        let (old, new) = if outgoing {
            (&self.old_calls.callees[i], &self.new_calls.callees[j])
        } else {
            (&self.old_calls.callers[i], &self.new_calls.callers[j])
        };
        if old.len().saturating_mul(new.len()) > MAX_NEIGHBOR_PRODUCT {
            // Context can refine existing candidates even when it is too broad
            // to generate a new candidate for every neighbor pair.
            for &a in old {
                if self.old_to_new[a].is_some() {
                    continue;
                }
                for &b in &self.old_edges[a] {
                    if self.new_to_old[b].is_none() && new.binary_search(&b).is_ok() {
                        let edge = self.edges.get_mut(&(a, b)).expect("registered edge");
                        edge.observe_call_edge(outgoing);
                        self.old_dirty.push(a);
                        self.new_dirty.push(b);
                    }
                }
            }
            return;
        }

        for &a in old {
            if self.old_to_new[a].is_some() {
                continue;
            }
            for &b in new {
                if self.new_to_old[b].is_some() || !self.before[a].permits_candidate(&self.after[b])
                {
                    continue;
                }
                if self.before[a].prototype != self.after[b].prototype
                    && !self.edges.contains_key(&(a, b))
                {
                    continue;
                }
                let edge = self.edges.entry((a, b)).or_insert_with(|| {
                    self.old_edges[a].push(b);
                    self.new_edges[b].push(a);
                    let mut edge = Evidence::default();
                    statically_check(
                        &mut edge,
                        &self.old[a],
                        &self.new[b],
                        &self.before[a],
                        &self.after[b],
                    );
                    edge
                });
                edge.observe_call_edge(outgoing);
                self.old_dirty.push(a);
                self.new_dirty.push(b);
            }
        }
    }

    fn infer(&mut self) -> Vec<Pair> {
        let mut pairs = Vec::new();
        let mut check_old = Vec::new();
        while !self.old_dirty.is_empty() || !self.new_dirty.is_empty() {
            self.old_dirty.sort_unstable();
            self.old_dirty.dedup();
            self.new_dirty.sort_unstable();
            self.new_dirty.dedup();
            for &j in &self.new_dirty {
                if self.new_to_old[j].is_none() {
                    self.new_choice[j] = self.rank(j, true);
                    if self.new_choice[j].clear() {
                        check_old.push(self.new_choice[j].target);
                    }
                }
            }
            for &i in &self.old_dirty {
                if self.old_to_new[i].is_none() {
                    self.old_choice[i] = self.rank(i, false);
                    check_old.push(i);
                }
            }
            self.old_dirty.clear();
            self.new_dirty.clear();
            check_old.sort_unstable();
            check_old.dedup();
            let start = pairs.len();
            for i in check_old.drain(..) {
                let left = self.old_choice[i];
                let j = left.target;
                if left.clear() && self.old_to_new[i].is_none() && self.new_to_old[j].is_none() {
                    let right = self.new_choice[j];
                    if right.clear() && right.target == i {
                        pairs.push(self.take((i, j)));
                    }
                }
            }
            for pair in &pairs[start..] {
                self.expand(pair.old, pair.new, true);
                self.expand(pair.old, pair.new, false);
            }
        }
        pairs.sort_unstable_by_key(|pair| pair.old);
        pairs
    }
}

// A helper that survives only inside a caller has strong body evidence for the
// new caller. Determine that relationship before either takes the new endpoint.
fn inspect_inline_candidates(search: &mut Search<'_>, old_identities: &[String]) {
    let mut reports = Vec::new();
    for (&(i, j), &edge) in &search.edges {
        if edge.exact || edge.anchors < 2 || search.before[i].size >= search.after[j].size {
            continue;
        }
        let (Some(caller), Some(target)) = (&search.old[i].body, &search.new[j].body) else {
            continue;
        };
        if !search.old[i].issues.is_empty() || !search.new[j].issues.is_empty() {
            continue;
        }
        let mut alignment = None;
        for &helper_index in &search.old_calls.callees[i] {
            if search
                .edges
                .get(&(helper_index, j))
                .is_none_or(|child| child.anchors == 0 || child.exact)
                || !search.old[helper_index].issues.is_empty()
            {
                continue;
            }
            let Some(helper) = &search.old[helper_index].body else {
                continue;
            };
            let Some(count) = inline::eligible(helper) else {
                continue;
            };
            let replaced = if helper.code[count].opcode == 0x0e {
                1
            } else {
                2
            };
            if target.code.len().saturating_sub(caller.code.len()) != count - replaced {
                continue;
            }
            for (invoke, op) in caller.code.iter().enumerate() {
                if !inline::is_call(op.opcode)
                    || caller.callee(invoke) != Some(old_identities[helper_index].as_str())
                {
                    continue;
                }
                let aligned =
                    alignment.get_or_insert_with(|| alignment::align(&caller.code, &target.code));
                if inline::find(caller, helper, target, invoke, aligned)
                    .is_some_and(|site| site.corroborates_outer)
                {
                    reports.push((i, helper_index, j));
                    break;
                }
            }
        }
    }
    for (caller, helper, target) in reports {
        search
            .edges
            .get_mut(&(caller, target))
            .expect("observed candidate")
            .inline_caller = true;
        search
            .edges
            .get_mut(&(helper, target))
            .expect("observed embedded fragment")
            .embedded = true;
    }
}

pub(super) fn compare(old: &[Method], new: &[Method]) -> Matching {
    let identities = (
        old.iter().map(Method::identity).collect::<Vec<_>>(),
        new.iter().map(Method::identity).collect::<Vec<_>>(),
    );
    let mut postings = Vec::new();
    let mut before = profile_and_postings(old, &identities.0, false, &mut postings);
    let mut after = profile_and_postings(new, &identities.1, true, &mut postings);
    let mut edges = candidates(postings, &before, &after, &identities);
    let mut old_edges = vec![Vec::new(); old.len()];
    let mut new_edges = vec![Vec::new(); new.len()];
    for (&(i, j), edge) in &mut edges {
        statically_check(edge, &old[i], &new[j], &before[i], &after[j]);
        old_edges[i].push(j);
        new_edges[j].push(i);
    }
    let old_calls = calls(old, &identities.0);
    let new_calls = calls(new, &identities.1);
    for (profile, callees) in before.iter_mut().zip(&old_calls.callees) {
        profile.independent_body &= callees.is_empty();
    }
    for (profile, callees) in after.iter_mut().zip(&new_calls.callees) {
        profile.independent_body &= callees.is_empty();
    }
    let mut search = Search {
        old,
        new,
        before,
        after,
        old_calls,
        new_calls,
        edges,
        old_edges,
        new_edges,
        old_dirty: (0..old.len()).collect(),
        new_dirty: (0..new.len()).collect(),
        old_choice: vec![Choice::default(); old.len()],
        new_choice: vec![Choice::default(); new.len()],
        old_to_new: vec![None; old.len()],
        new_to_old: vec![None; new.len()],
        old_owner: HashMap::new(),
        new_owner: HashMap::new(),
        anchors_only: true,
    };
    inspect_inline_candidates(&mut search, &identities.0);
    let mut pairs = search.infer();
    search.seed_owners(&pairs);
    search.anchors_only = false;
    pairs.extend(search.infer());
    pairs.sort_unstable_by_key(|pair| pair.old);
    let pending = search
        .edges
        .iter()
        .filter_map(|(&(i, j), &edge)| {
            (search.old_to_new[i].is_none()
                && search.new_to_old[j].is_none()
                && (!search.owner_evidence(i, j, edge).other_owner && edge.identity
                    || score(
                        search.owner_evidence(i, j, edge),
                        &search.before[i],
                        &search.after[j],
                        false,
                    ) > 0))
                .then_some((i, j))
        })
        .collect();
    let mut callees: HashMap<String, String> = pairs
        .iter()
        .filter(|pair| search.old_calls.unique[pair.old] && search.new_calls.unique[pair.new])
        .map(|pair| {
            (
                identities.0[pair.old].clone(),
                identities.1[pair.new].clone(),
            )
        })
        .collect();
    let owners: HashMap<_, _> = search
        .old_owner
        .iter()
        .filter_map(|(&a, &b)| {
            let b = b?;
            (search.new_owner.get(b).copied().flatten() == Some(a))
                .then(|| (a.to_owned(), b.to_owned()))
        })
        .collect();
    callees.extend(owners.iter().map(|(a, b)| (a.clone(), b.clone())));
    Matching {
        pairs,
        old_to_new: search.old_to_new,
        new_to_old: search.new_to_old,
        callees,
        candidate_pairs: search.edges.len(),
        pending,
        owners,
    }
}

struct Components {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl Components {
    fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
            rank: vec![0; count],
        }
    }

    fn find(&mut self, mut id: usize) -> usize {
        while id != self.parent[id] {
            self.parent[id] = self.parent[self.parent[id]];
            id = self.parent[id];
        }
        id
    }

    fn union(&mut self, a: usize, b: usize) {
        let (mut a, mut b) = (self.find(a), self.find(b));
        if a == b {
            return;
        }
        if self.rank[a] < self.rank[b] {
            std::mem::swap(&mut a, &mut b);
        }
        self.parent[b] = a;
        if self.rank[a] == self.rank[b] {
            self.rank[a] += 1;
        }
    }
}

impl Matching {
    pub(super) fn unmatched(&self, old: &Index, new: &Index) -> Vec<UnmatchedGroup> {
        let (old_count, new_count) = (old.methods.len(), new.methods.len());
        let mut components = Components::new(old_count + new_count);
        for &(i, j) in &self.pending {
            components.union(i, old_count + j);
        }

        // Keep a heavy group as two endpoint lists. This preserves the unresolved
        // possibilities without constructing its complete bipartite graph.
        let reverse: HashMap<_, _> = self
            .owners
            .iter()
            .map(|(a, b)| (b.as_str(), a.as_str()))
            .collect();
        let mut shapes: HashMap<_, (Vec<usize>, Vec<usize>)> = HashMap::new();
        for (is_new, index, mappings) in [
            (false, old, &self.old_to_new),
            (true, new, &self.new_to_old),
        ] {
            for (id, method) in index.methods.iter().enumerate() {
                if mappings[id].is_some() {
                    continue;
                }
                if let Some(body) = &method.body {
                    let mut hash = DefaultHasher::new();
                    prototype_shape(&method.prototype).hash(&mut hash);
                    body.code.iter().for_each(|op| op_shape(op).hash(&mut hash));
                    let owner = if is_new {
                        reverse.get(method.owner.as_str()).copied()
                    } else {
                        self.owners
                            .contains_key(method.owner.as_str())
                            .then_some(method.owner.as_str())
                    };
                    let bucket = shapes.entry((owner, hash.finish())).or_default();
                    if is_new {
                        bucket.1.push(old_count + id);
                    } else {
                        bucket.0.push(id);
                    }
                }
            }
        }
        for (a, b) in shapes.into_values() {
            if let (Some(&first), Some(_)) = (a.first(), b.first()) {
                for other in a.into_iter().chain(b) {
                    components.union(first, other);
                }
            }
        }

        let mut groups: BTreeMap<_, UnmatchedGroup> = BTreeMap::new();
        for (is_new, offset, index, opposite, mappings) in [
            (false, 0, old, new, &self.old_to_new),
            (true, old_count, new, old, &self.new_to_old),
        ] {
            for (id, method) in index.methods.iter().enumerate() {
                if mappings[id].is_some() {
                    continue;
                }
                let group = groups
                    .entry(components.find(offset + id))
                    .or_insert_with(|| UnmatchedGroup {
                        status: "unmatched",
                        old: Vec::new(),
                        new: Vec::new(),
                    });
                if !method.issues.is_empty() || !opposite.issues.is_empty() {
                    group.status = "unavailable";
                }
                if !is_new {
                    group.old.push(method.into());
                } else {
                    group.new.push(method.into());
                }
            }
        }
        let mut out: Vec<_> = groups.into_values().collect();
        for group in &mut out {
            if group.status != "unavailable" && !group.old.is_empty() && !group.new.is_empty() {
                group.status = "ambiguous";
            }
        }
        out.sort_unstable_by(|a, b| {
            (a.old.first().map(|x| &x.at), a.new.first().map(|x| &x.at))
                .cmp(&(b.old.first().map(|x| &x.at), b.new.first().map(|x| &x.at)))
        });
        out
    }
}
