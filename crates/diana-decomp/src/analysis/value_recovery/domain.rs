//! Bounded proof domain for value availability.
//!
//! Control formulas are retained as a hash-consed Boolean DAG. ROBDD handles
//! are attached while the method-local proof budget permits exact implication.
//! Budget exhaustion makes subsequent proofs conservative without abandoning
//! value recovery or expanding formulas into trees.

use std::cell::{Cell, OnceCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::hash::BuildHasher;

use crate::ir::bdd::{Bdd, BddContext, BddError};
use crate::ir::{BoolExpr, BoolVariable};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct ControlDomain(u32);

impl ControlDomain {
    /// Arena-local ordinal, including the two constant domains.
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone)]
pub struct DomainError {
    kind: DomainFailure,
}

#[derive(Debug, Clone)]
enum DomainFailure {
    Invalid(ControlDomain),
    Capacity(usize),
    Boolean(BddError),
    MalformedEvaluation,
}

impl DomainError {
    fn invalid(domain: ControlDomain) -> Self {
        Self {
            kind: DomainFailure::Invalid(domain),
        }
    }

    fn capacity(capacity: usize) -> Self {
        Self {
            kind: DomainFailure::Capacity(capacity),
        }
    }

    fn malformed_evaluation() -> Self {
        Self {
            kind: DomainFailure::MalformedEvaluation,
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            DomainFailure::Invalid(domain) => {
                write!(formatter, "invalid control domain {}", domain.0)
            }
            DomainFailure::Capacity(capacity) => {
                write!(formatter, "control-domain arena exceeds {capacity} nodes")
            }
            DomainFailure::Boolean(source) => write!(formatter, "Boolean proof failed: {source}"),
            DomainFailure::MalformedEvaluation => {
                formatter.write_str("malformed control-domain evaluation stack")
            }
        }
    }
}

impl std::error::Error for DomainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            DomainFailure::Boolean(source) => Some(source),
            _ => None,
        }
    }
}

impl From<BddError> for DomainError {
    fn from(source: BddError) -> Self {
        Self {
            kind: DomainFailure::Boolean(source),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TermSpan {
    page: usize,
    // Shared writes start below SHARED_PAGE_TERMS. A dedicated page starts
    // at zero. A set has at most one term per nonterminal u32 domain ID.
    start: u16,
    len: u32,
    fingerprint: u64,
}

impl TermSpan {
    fn range(self) -> std::ops::Range<usize> {
        let start = usize::from(self.start);
        start..start + self.len as usize
    }
}

#[derive(Default)]
struct TermArena {
    pages: Vec<Vec<ControlDomain>>,
    shared: Option<usize>,
}

impl TermArena {
    const SHARED_PAGE_TERMS: usize = 16_384;

    fn terms(&self, span: TermSpan) -> &[ControlDomain] {
        &self.pages[span.page][span.range()]
    }

    fn dedicated(&mut self, terms: Vec<ControlDomain>, fingerprint: u64) -> TermSpan {
        let len = terms.len() as u32;
        let page = self.pages.len();
        self.pages.push(terms);
        TermSpan {
            page,
            start: 0,
            len,
            fingerprint,
        }
    }

    fn store(
        &mut self,
        terms: &[ControlDomain],
        operand: Option<TermSpan>,
        fingerprint: u64,
    ) -> TermSpan {
        if let Some(span) = operand {
            let size = span.len as usize;
            // The union contains every operand term in sorted order. An
            // unchanged last position therefore establishes the whole prefix.
            if self.terms(span).last() == terms.get(size - 1) {
                return self.extend(span, &terms[size..], fingerprint);
            }
        }
        self.store_shared(terms, fingerprint)
    }

    fn store_shared(&mut self, terms: &[ControlDomain], fingerprint: u64) -> TermSpan {
        let page = if let Some(page) = self
            .shared
            .filter(|&page| self.pages[page].len() < Self::SHARED_PAGE_TERMS)
        {
            page
        } else {
            let page = self.pages.len();
            self.pages.push(Vec::new());
            self.shared = Some(page);
            page
        };
        let start = self.pages[page].len() as u16;
        self.pages[page].extend_from_slice(terms);
        TermSpan {
            page,
            start,
            len: terms.len() as u32,
            fingerprint,
        }
    }

    fn extend(&mut self, span: TermSpan, suffix: &[ControlDomain], fingerprint: u64) -> TermSpan {
        let prefix = span.range();
        let page = &mut self.pages[span.page];
        let overlap = suffix.len().min(page.len() - prefix.end);
        let len = prefix.len() + suffix.len();
        if page[prefix.end..prefix.end + overlap] == suffix[..overlap] {
            page.extend_from_slice(&suffix[overlap..]);
            return TermSpan {
                len: len as u32,
                fingerprint,
                ..span
            };
        }

        let mut branch = Vec::with_capacity(len);
        branch.extend_from_slice(&page[prefix]);
        branch.extend_from_slice(suffix);
        if len >= 16 {
            // Diverging long branches can continue growing independently.
            self.dedicated(branch, fingerprint)
        } else {
            self.store_shared(&branch, fingerprint)
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.pages.iter().map(Vec::len).sum()
    }
}

#[derive(Debug)]
enum DomainNode {
    True,
    False,
    Symbol,
    Not(ControlDomain),
    AndPair([ControlDomain; 2]),
    OrPair([ControlDomain; 2]),
    And(TermSpan),
    Or(TermSpan),
}

impl DomainNode {
    fn span(&self, conjunction: bool) -> Option<TermSpan> {
        match (self, conjunction) {
            (Self::And(span), true) | (Self::Or(span), false) => Some(*span),
            _ => None,
        }
    }

    fn terms<'a>(&'a self, conjunction: bool, arena: &'a TermArena) -> Option<&'a [ControlDomain]> {
        match (self, conjunction) {
            (Self::AndPair(terms), true) | (Self::OrPair(terms), false) => Some(terms),
            (Self::And(range), true) | (Self::Or(range), false) => Some(arena.terms(*range)),
            _ => None,
        }
    }
}

struct DomainRecord {
    node: DomainNode,
    proof: Option<Bdd>,
    complement: Option<ControlDomain>,
}

/// A collector creates structural domains before the planner observes any
/// exact proof. These are the successful creating operands, in order. Repeated
/// structural lookups and terminal shortcuts leave the bounded arena untouched.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ProofStep {
    Symbol(usize),
    Not(ControlDomain),
    And([ControlDomain; 2]),
    Or([ControlDomain; 2]),
}

struct Collection {
    steps: Vec<ProofStep>,
    previous: Option<Box<CollectedProofs>>,
}

struct CollectedProofs {
    steps: Vec<ProofStep>,
    bdd: BddContext,
    roots: Vec<Option<Bdd>>,
    saturated: bool,
}

#[derive(Default)]
pub(super) struct DomainProofCache(std::sync::Mutex<Option<Box<CollectedProofs>>>);

impl fmt::Debug for DomainProofCache {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DomainProofCache")
            .finish_non_exhaustive()
    }
}

pub(super) struct DomainLogic {
    bdd: OnceCell<BddContext>,
    collection: Option<Box<Collection>>,
    nodes: Vec<DomainRecord>,
    symbols: BTreeMap<BoolVariable, ControlDomain>,
    joins: foldhash::HashMap<(ControlDomain, ControlDomain, bool), ControlDomain>,
    /// Larger canonical sets occupy one method-local arena. The table stores
    /// their creating domains and only accepts an exact ordered-term match.
    term_arena: TermArena,
    term_work: Vec<ControlDomain>,
    junction_index: hashbrown::HashTable<(u64, ControlDomain)>,
    // Each term is hashed separately before the fingerprints are added.
    // Avalanche adjacent ordinals before combining fixed-sum sets.
    term_hasher: foldhash::quality::RandomState,
    saturated: Cell<bool>,
}

impl DomainLogic {
    const MAX_COLLECTION_STEPS: usize = 65_536;
    const LONG_JUNCTION_OPERAND: usize = 64;
    const TRUE: ControlDomain = ControlDomain(0);
    const FALSE: ControlDomain = ControlDomain(1);

    pub(super) fn new(variables: &BTreeSet<BoolVariable>) -> Self {
        Self::ordered(variables.iter().cloned())
    }

    pub(super) fn ordered(variables: impl ExactSizeIterator<Item = BoolVariable>) -> Self {
        let bdd = if variables.len() == 0 {
            OnceCell::new()
        } else {
            OnceCell::from(BddContext::ordered(variables))
        };
        Self {
            collection: None,
            nodes: Vec::new(),
            symbols: BTreeMap::new(),
            joins: Default::default(),
            term_arena: TermArena::default(),
            term_work: Vec::new(),
            junction_index: hashbrown::HashTable::new(),
            term_hasher: Default::default(),
            bdd,
            saturated: Cell::new(false),
        }
    }

    // A guard-free method normally uses the two structural terminals only.
    // A later real proof or unknown symbol still uses the ordinary empty arena.
    fn bdd(&self) -> &BddContext {
        self.bdd
            .get_or_init(|| BddContext::ordered(std::iter::empty()))
    }

    pub(super) fn begin_collection(&mut self, cache: Option<&DomainProofCache>) {
        // Small Boolean arenas are cheaper to build directly. Only one
        // previous collection is retained by this method's recovery pipeline.
        let Some(cache) = cache else { return };
        let Some(bdd) = self
            .bdd
            .get()
            .filter(|b| (17..=Self::MAX_COLLECTION_STEPS).contains(&b.variable_order().len()))
        else {
            return;
        };
        let previous = cache
            .0
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .take()
            .filter(|p| p.bdd.variable_order() == bdd.variable_order());
        self.collection = Some(Box::new(Collection {
            steps: Vec::new(),
            previous,
        }));
    }

    pub(super) fn finish_collection(
        &mut self,
        cache: Option<&DomainProofCache>,
    ) -> Result<(), DomainError> {
        let Some(collection) = self.collection.take() else {
            return Ok(());
        };
        let Collection { steps, previous } = *collection;
        let snapshot = match previous {
            // Every creating step matched as it happened. A shorter prefix
            // has a different budget and must be built in its original order.
            Some(previous) if steps.len() == previous.steps.len() => {
                let (bdd, roots) = previous.bdd.fork(previous.roots.iter().copied())?;
                self.bdd = OnceCell::from(bdd);
                for (record, proof) in self.nodes.iter_mut().zip(roots) {
                    record.proof = proof;
                }
                self.saturated.set(previous.saturated);
                previous
            }
            different => {
                if different.is_some() {
                    drop(different);
                    self.materialize(&steps)?;
                }
                let Some(_) = cache else { return Ok(()) };
                let (bdd, roots) = self
                    .bdd()
                    .fork(self.nodes.iter().map(|record| record.proof))?;
                Box::new(CollectedProofs {
                    steps,
                    bdd,
                    roots,
                    saturated: self.saturated.get(),
                })
            }
        };
        if let Some(cache) = cache {
            *cache.0.lock().unwrap_or_else(|poison| poison.into_inner()) = Some(snapshot);
        }
        Ok(())
    }

    fn record(&mut self, step: ProofStep) -> Result<bool, DomainError> {
        let Some(mut collection) = self.collection.take() else {
            return Ok(false);
        };
        // Bound additional recipe space even for a hostile structural DAG much
        // larger than its method-local BDD budget. Only a previously completed
        // prefix can be deferred. A novel operation runs at its original site.
        let full = collection.steps.len() >= Self::MAX_COLLECTION_STEPS;
        if collection.previous.as_ref().is_some_and(|previous| {
            full || previous.steps.get(collection.steps.len()) != Some(&step)
        }) {
            collection.previous = None;
            self.materialize(&collection.steps)?;
        }
        if full {
            return Ok(false);
        }
        let reused = collection.previous.is_some();
        collection.steps.push(step);
        self.collection = Some(collection);
        Ok(reused)
    }

    fn flush_collection(&mut self) -> Result<(), DomainError> {
        if let Some(collection) = self.collection.take() {
            if collection.previous.is_some() {
                let Collection { steps, previous } = *collection;
                drop(previous);
                self.materialize(&steps)?;
            }
        }
        Ok(())
    }

    fn materialize(&mut self, steps: &[ProofStep]) -> Result<(), DomainError> {
        for (i, &step) in steps.iter().enumerate() {
            if self.saturated.get() {
                break;
            }
            let proof = match step {
                ProofStep::Symbol(index) => {
                    let bdd = self.bdd();
                    if index >= bdd.variable_order().len() {
                        return Err(DomainError::malformed_evaluation());
                    }
                    self.proof(bdd.compile_index(index, true))?
                }
                ProofStep::Not(domain) => match self.exact(domain) {
                    Some(proof) => self.proof(self.bdd().not(proof))?,
                    None => None,
                },
                ProofStep::And(pair) | ProofStep::Or(pair) => {
                    let conjunction = matches!(step, ProofStep::And(_));
                    let terms = self.nodes[i]
                        .node
                        .terms(conjunction, &self.term_arena)
                        .ok_or_else(DomainError::malformed_evaluation)?;
                    self.junction_proof(terms, conjunction, pair)?
                }
            };
            self.nodes[i].proof = proof;
        }
        Ok(())
    }

    pub(super) fn proof_budget_exhausted(&self) -> bool {
        self.saturated.get()
    }

    pub(super) fn truth(&self) -> ControlDomain {
        Self::TRUE
    }

    pub(super) fn falsity(&self) -> ControlDomain {
        Self::FALSE
    }

    pub(super) fn predicate(
        &mut self,
        predicate: &crate::ir::SemanticPredicate,
    ) -> Result<ControlDomain, super::ValueRecoveryError> {
        use crate::ir::SemanticPredicate;
        match predicate {
            SemanticPredicate::True => Ok(Self::TRUE),
            SemanticPredicate::False => Ok(Self::FALSE),
            SemanticPredicate::Test(instruction) => {
                Ok(self.symbol(BoolVariable::Instruction(instruction.id))?)
            }
            SemanticPredicate::Not(inner) => match inner.as_ref() {
                SemanticPredicate::True => Ok(Self::FALSE),
                SemanticPredicate::False => Ok(Self::TRUE),
                SemanticPredicate::Test(instruction) => {
                    let symbol = self.symbol(BoolVariable::Instruction(instruction.id))?;
                    Ok(self.not(symbol)?)
                }
                _ => Ok(self.compile(&predicate.domain()?)?),
            },
            _ => Ok(self.compile(&predicate.domain()?)?),
        }
    }

    pub(super) fn compile(&mut self, expression: &BoolExpr) -> Result<ControlDomain, DomainError> {
        match expression {
            BoolExpr::True => return Ok(Self::TRUE),
            BoolExpr::False => return Ok(Self::FALSE),
            BoolExpr::Symbol(symbol) => return self.symbol(symbol.clone()),
            _ => {}
        }
        let mut pending = vec![CompileTask::Visit(expression)];
        let mut results = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                CompileTask::Visit(BoolExpr::True) => results.push(Self::TRUE),
                CompileTask::Visit(BoolExpr::False) => results.push(Self::FALSE),
                CompileTask::Visit(BoolExpr::Symbol(symbol)) => {
                    results.push(self.symbol(symbol.clone())?);
                }
                CompileTask::Visit(BoolExpr::Not(inner)) => {
                    pending.push(CompileTask::Not);
                    pending.push(CompileTask::Visit(inner));
                }
                CompileTask::Visit(BoolExpr::And(terms)) => {
                    pending.push(CompileTask::And(terms.len()));
                    pending.extend(terms.iter().rev().map(CompileTask::Visit));
                }
                CompileTask::Visit(BoolExpr::Or(terms)) => {
                    pending.push(CompileTask::Or(terms.len()));
                    pending.extend(terms.iter().rev().map(CompileTask::Visit));
                }
                CompileTask::Not => {
                    let value = results
                        .pop()
                        .ok_or_else(DomainError::malformed_evaluation)?;
                    results.push(self.not(value)?);
                }
                CompileTask::And(count) => {
                    let value = self.combine(&mut results, count, true)?;
                    results.push(value);
                }
                CompileTask::Or(count) => {
                    let value = self.combine(&mut results, count, false)?;
                    results.push(value);
                }
            }
        }
        if results.len() != 1 {
            return Err(DomainError::malformed_evaluation());
        }
        results.pop().ok_or_else(DomainError::malformed_evaluation)
    }

    pub(super) fn not(&mut self, domain: ControlDomain) -> Result<ControlDomain, DomainError> {
        match domain {
            Self::TRUE => Ok(Self::FALSE),
            Self::FALSE => Ok(Self::TRUE),
            ControlDomain(id) => {
                let index = (id - 2) as usize;
                let record = self
                    .nodes
                    .get(index)
                    .ok_or_else(|| DomainError::invalid(domain))?;
                if let Some(complement) = record.complement {
                    return Ok(complement);
                }
                let proof = if self.collection.is_some() && self.record(ProofStep::Not(domain))? {
                    None
                } else if !self.saturated.get() {
                    match self.nodes[index].proof {
                        Some(value) => self.proof(self.bdd().not(value))?,
                        None => None,
                    }
                } else {
                    None
                };
                let complement = self.push_node(DomainNode::Not(domain), proof, Some(domain))?;
                self.nodes[index].complement = Some(complement);
                Ok(complement)
            }
        }
    }

    fn combine(
        &mut self,
        results: &mut Vec<ControlDomain>,
        count: usize,
        conjunction: bool,
    ) -> Result<ControlDomain, DomainError> {
        let start = results
            .len()
            .checked_sub(count)
            .ok_or_else(DomainError::malformed_evaluation)?;
        let values = results.drain(start..);
        let mut value = if conjunction { Self::TRUE } else { Self::FALSE };
        for operand in values {
            value = if conjunction {
                self.and(value, operand)?
            } else {
                self.or(value, operand)?
            };
        }
        Ok(value)
    }

    pub(super) fn and(
        &mut self,
        left: ControlDomain,
        right: ControlDomain,
    ) -> Result<ControlDomain, DomainError> {
        self.junction(left, right, true)
    }

    pub(super) fn or(
        &mut self,
        left: ControlDomain,
        right: ControlDomain,
    ) -> Result<ControlDomain, DomainError> {
        self.junction(left, right, false)
    }

    pub(super) fn equivalent(
        &self,
        left: ControlDomain,
        right: ControlDomain,
    ) -> Result<bool, DomainError> {
        if left == right {
            return Ok(true);
        }
        match (self.exact(left), self.exact(right)) {
            (Some(left), Some(right)) => match self.bdd().equivalent(left, right) {
                Ok(result) => Ok(result),
                Err(error) if error.is_resource_limit() => {
                    self.saturated.set(true);
                    Ok(false)
                }
                Err(error) => Err(error.into()),
            },
            _ => Ok(false),
        }
    }

    pub(super) fn implies(
        &self,
        premise: ControlDomain,
        consequence: ControlDomain,
    ) -> Result<bool, DomainError> {
        if premise == consequence || consequence == Self::TRUE || premise == Self::FALSE {
            return Ok(true);
        }
        if self
            .node(premise)?
            .terms(true, &self.term_arena)
            .is_some_and(|terms| terms.binary_search(&consequence).is_ok())
            || self
                .node(consequence)?
                .terms(false, &self.term_arena)
                .is_some_and(|terms| terms.binary_search(&premise).is_ok())
        {
            return Ok(true);
        }
        if self.saturated.get() {
            return Ok(false);
        }
        let (Some(premise), Some(consequence)) = (self.exact(premise), self.exact(consequence))
        else {
            return Ok(false);
        };
        match self.bdd().implies_bdd(premise, consequence) {
            Ok(result) => Ok(result),
            Err(error) if error.is_resource_limit() => {
                self.saturated.set(true);
                Ok(false)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn covered_by(
        &self,
        premise: ControlDomain,
        consequences: impl IntoIterator<Item = ControlDomain>,
    ) -> Result<bool, DomainError> {
        let Some(premise) = self.exact(premise) else {
            return Ok(false);
        };
        let mut covered = self.bdd().falsity();
        for consequence in consequences {
            let Some(consequence) = self.exact(consequence) else {
                return Ok(false);
            };
            covered = match self.bdd().or(covered, consequence) {
                Ok(value) => value,
                Err(error) if error.is_resource_limit() => {
                    self.saturated.set(true);
                    return Ok(false);
                }
                Err(error) => return Err(error.into()),
            };
        }
        match self.bdd().implies_bdd(premise, covered) {
            Ok(result) => Ok(result),
            Err(error) if error.is_resource_limit() => {
                self.saturated.set(true);
                Ok(false)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn disjoint(
        &self,
        left: ControlDomain,
        right: ControlDomain,
    ) -> Result<bool, DomainError> {
        if left == Self::FALSE || right == Self::FALSE {
            return Ok(true);
        }
        if self.saturated.get() || (left == Self::TRUE && right == Self::TRUE) {
            return Ok(false);
        }
        let (Some(left), Some(right)) = (self.exact(left), self.exact(right)) else {
            return Ok(false);
        };
        // Terminal and equal operands are precisely the uncharged terminal
        // cases in BDD intersection. The handles came from this arena.
        if left.is_false() || right.is_false() {
            return Ok(true);
        }
        if left == right || left.is_true() || right.is_true() {
            return Ok(false);
        }
        match self.bdd().and(left, right) {
            Ok(intersection) => Ok(intersection.is_false()),
            Err(error) if error.is_resource_limit() => {
                self.saturated.set(true);
                Ok(false)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Disjointness with this operand cannot allocate an intersection or
    /// exhaust a proof budget, regardless of its other operand.
    pub(super) fn disjoint_without_bdd_work(&self, domain: ControlDomain) -> bool {
        domain == Self::TRUE
            || domain == Self::FALSE
            || self.saturated.get()
            || self
                .exact(domain)
                .is_none_or(|bdd| bdd.is_true() || bdd.is_false())
    }

    pub(super) fn expression_under(
        &self,
        value: ControlDomain,
        care: ControlDomain,
        node_limit: usize,
    ) -> Result<Option<BoolExpr>, DomainError> {
        if self.saturated.get() {
            return Ok(None);
        }
        let (Some(value), Some(care)) = (self.exact(value), self.exact(care)) else {
            return Ok(None);
        };
        let constrained = match self.bdd().constrain(value, care) {
            Ok(value) => value,
            Err(error) if error.is_resource_limit() => {
                self.saturated.set(true);
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        match self.bdd().expression(constrained, node_limit) {
            Ok(expression) => Ok(expression.map(|(expression, _)| expression)),
            Err(error) if error.is_resource_limit() => {
                self.saturated.set(true);
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn symbol(&mut self, symbol: BoolVariable) -> Result<ControlDomain, DomainError> {
        if let Some(domain) = self.symbols.get(&symbol).copied() {
            return Ok(domain);
        }
        let mut deferred = false;
        if self.collection.is_some() {
            if let Some(index) = self.bdd().variable_index(&symbol) {
                deferred = self.record(ProofStep::Symbol(index))?;
            } else {
                // An unknown variable is a construction error unless earlier
                // operations exhausted the arena. Determine that at this site.
                self.flush_collection()?;
            }
        }
        let proof = if deferred || self.saturated.get() {
            None
        } else {
            self.proof(self.bdd().compile_literal(&symbol, true))?
        };
        let domain = self.push_node(DomainNode::Symbol, proof, None)?;
        self.symbols.insert(symbol, domain);
        Ok(domain)
    }

    fn junction(
        &mut self,
        left: ControlDomain,
        right: ControlDomain,
        conjunction: bool,
    ) -> Result<ControlDomain, DomainError> {
        let absorbing = if conjunction { Self::FALSE } else { Self::TRUE };
        let identity = if conjunction { Self::TRUE } else { Self::FALSE };
        if left == absorbing || right == absorbing {
            return Ok(absorbing);
        }
        if left == identity {
            return Ok(right);
        }
        if right == identity || left == right {
            return Ok(left);
        }

        let a = self.junction_terms(&left, conjunction)?;
        let b = self.junction_terms(&right, conjunction)?;
        let input_pair = (left.min(right), left.max(right), conjunction);
        // Two-term junctions are interned by their only producing pair.
        // Wider sets have their own exact term index. Keep a second key
        // only when recovering that set would scan a long operand.
        let remember_pair =
            (a.len() == 1 && b.len() == 1) || a.len().max(b.len()) >= Self::LONG_JUNCTION_OPERAND;
        if remember_pair {
            if let Some(&domain) = self.joins.get(&input_pair) {
                return Ok(domain);
            }
        }
        let (shorter, longer, longer_domain) = if a.len() <= b.len() {
            (a, b, right)
        } else {
            (b, a, left)
        };
        // Each interned junction already contains distinct, sorted terms
        // without complements. Only cross-operand pairs can cancel. When a
        // guard just repeats existing terms its canonical domain already
        // owns the exact proof and needs no union allocation or BDD work.
        let mut contained = true;
        for &term in shorter {
            let complement = self.nodes[(term.0 - 2) as usize].complement;
            if complement.is_some_and(|other| longer.binary_search(&other).is_ok()) {
                return Ok(absorbing);
            }
            contained = contained && longer.binary_search(&term).is_ok();
        }
        if contained {
            return Ok(longer_domain);
        }
        let domain = if a.len() == 1 && b.len() == 1 {
            // After the containment check a two-term union can only be
            // formed from its two singleton operands, in either order.
            let pair = [input_pair.0, input_pair.1];
            let proof = self.new_junction_proof(&pair, conjunction, [left, right])?;
            let node = if conjunction {
                DomainNode::AndPair(pair)
            } else {
                DomainNode::OrPair(pair)
            };
            self.push_node(node, proof, None)?
        } else {
            let prefix = self.node(longer_domain)?.span(conjunction);
            // A commutative fingerprint makes a single added term independent
            // of its sorted position. Candidates still require exact ordered
            // equality, including on a collision.
            let single_hash = match shorter {
                [term] => Some(
                    prefix
                        .map_or_else(
                            || self.fingerprint(longer, conjunction),
                            |span| span.fingerprint,
                        )
                        .wrapping_add(self.term_hasher.hash_one(term)),
                ),
                _ => None,
            };
            let appended = match (prefix, shorter, single_hash) {
                (Some(span), [term], Some(hash)) if longer.last() < Some(term) => {
                    Some((span, *term, hash))
                }
                _ => None,
            };
            let (range, proof, hash) = if let Some((span, term, hash)) = appended {
                if let Some(&(_, existing)) = self.junction_index.find(hash, |&(_, domain)| {
                    self.nodes[(domain.0 - 2) as usize]
                        .node
                        .terms(conjunction, &self.term_arena)
                        .and_then(|terms| terms.split_last())
                        == Some((&term, longer))
                }) {
                    return Ok(existing);
                }
                let operands = [left, right];
                let proof = if self.defer_junction(conjunction, operands)? || self.saturated.get() {
                    None
                } else {
                    match (self.exact(longer_domain), self.exact(term)) {
                        (Some(prefix), Some(term)) => self.apply_term(prefix, term, conjunction)?,
                        _ => None,
                    }
                };
                (
                    self.term_arena
                        .extend(span, std::slice::from_ref(&term), hash),
                    proof,
                    hash,
                )
            } else {
                let mut terms = std::mem::take(&mut self.term_work);
                Self::union_terms(
                    &mut terms,
                    self.junction_terms(&left, conjunction)?,
                    self.junction_terms(&right, conjunction)?,
                );
                let hash = single_hash.unwrap_or_else(|| self.fingerprint(&terms, conjunction));
                if let Some(&(_, existing)) = self.junction_index.find(hash, |&(_, domain)| {
                    self.nodes[(domain.0 - 2) as usize]
                        .node
                        .terms(conjunction, &self.term_arena)
                        == Some(terms.as_slice())
                }) {
                    self.term_work = terms;
                    return Ok(existing);
                }
                let proof = self.new_junction_proof(&terms, conjunction, [left, right])?;
                let range = self.term_arena.store(&terms, prefix, hash);
                self.term_work = terms;
                (range, proof, hash)
            };
            let node = if conjunction {
                DomainNode::And(range)
            } else {
                DomainNode::Or(range)
            };
            let domain = self.push_node(node, proof, None)?;
            self.junction_index
                .insert_unique(hash, (hash, domain), |entry| entry.0);
            domain
        };
        if remember_pair {
            self.joins.insert(input_pair, domain);
        }
        Ok(domain)
    }

    fn fingerprint(&self, terms: &[ControlDomain], conjunction: bool) -> u64 {
        terms.iter().fold(u64::from(conjunction), |hash, term| {
            hash.wrapping_add(self.term_hasher.hash_one(term))
        })
    }

    fn junction_terms<'a>(
        &'a self,
        domain: &'a ControlDomain,
        conjunction: bool,
    ) -> Result<&'a [ControlDomain], DomainError> {
        Ok(self
            .node(*domain)?
            .terms(conjunction, &self.term_arena)
            .unwrap_or(std::slice::from_ref(domain)))
    }

    fn union_terms(
        result: &mut Vec<ControlDomain>,
        mut left: &[ControlDomain],
        mut right: &[ControlDomain],
    ) {
        result.clear();
        result.reserve(left.len() + right.len());
        if left.last() < right.first() {
            result.extend_from_slice(left);
            result.extend_from_slice(right);
            return;
        }
        if right.last() < left.first() {
            result.extend_from_slice(right);
            result.extend_from_slice(left);
            return;
        }
        let (shorter, longer) = if left.len() < right.len() {
            (left, right)
        } else {
            (right, left)
        };
        if let [single] = shorter {
            let position = longer.partition_point(|term| term < single);
            result.extend_from_slice(&longer[..position]);
            if longer.get(position) != Some(single) {
                result.push(*single);
            }
            result.extend_from_slice(&longer[position..]);
            return;
        }
        while let ([a, more_left @ ..], [b, more_right @ ..]) = (left, right) {
            result.push((*a).min(*b));
            if a <= b {
                left = more_left;
            }
            if b <= a {
                right = more_right;
            }
        }
        result.extend_from_slice(left);
        result.extend_from_slice(right);
    }

    fn new_junction_proof(
        &mut self,
        terms: &[ControlDomain],
        conjunction: bool,
        operands: [ControlDomain; 2],
    ) -> Result<Option<Bdd>, DomainError> {
        if self.defer_junction(conjunction, operands)? {
            return Ok(None);
        }
        self.junction_proof(terms, conjunction, operands)
    }

    fn defer_junction(
        &mut self,
        conjunction: bool,
        operands: [ControlDomain; 2],
    ) -> Result<bool, DomainError> {
        if self.collection.is_none() {
            return Ok(false);
        }
        self.record(if conjunction {
            ProofStep::And(operands)
        } else {
            ProofStep::Or(operands)
        })
    }

    fn apply_term(
        &self,
        prefix: Bdd,
        term: Bdd,
        conjunction: bool,
    ) -> Result<Option<Bdd>, DomainError> {
        self.proof(if conjunction {
            self.bdd().and(prefix, term)
        } else {
            self.bdd().or(prefix, term)
        })
    }

    fn junction_proof(
        &self,
        terms: &[ControlDomain],
        conjunction: bool,
        operands: [ControlDomain; 2],
    ) -> Result<Option<Bdd>, DomainError> {
        if self.saturated.get() {
            return Ok(None);
        }
        // A previously interned junction was folded in this same sorted
        // order. Its ROBDD already contains exactly the prefix results that
        // replaying those binary operations would find in the apply cache.
        let mut start = 1;
        let Some(mut value) = self.exact(terms[0]) else {
            return Ok(None);
        };
        for operand in operands {
            let Some(prefix) = self.node(operand)?.terms(conjunction, &self.term_arena) else {
                continue;
            };
            if prefix.len() > start && terms.starts_with(prefix) {
                let Some(proof) = self.exact(operand) else {
                    return Ok(None);
                };
                start = prefix.len();
                value = proof;
            }
        }
        for term in &terms[start..] {
            let Some(operand) = self.exact(*term) else {
                return Ok(None);
            };
            let Some(exact) = self.apply_term(value, operand, conjunction)? else {
                return Ok(None);
            };
            value = exact;
        }
        Ok(Some(value))
    }

    fn node(&self, domain: ControlDomain) -> Result<&DomainNode, DomainError> {
        match domain {
            Self::TRUE => Ok(&DomainNode::True),
            Self::FALSE => Ok(&DomainNode::False),
            ControlDomain(id) => self
                .nodes
                .get((id - 2) as usize)
                .map(|record| &record.node)
                .ok_or_else(|| DomainError::invalid(domain)),
        }
    }

    fn exact(&self, domain: ControlDomain) -> Option<Bdd> {
        match domain {
            Self::TRUE => Some(self.bdd().truth()),
            Self::FALSE => Some(self.bdd().falsity()),
            ControlDomain(id) => self
                .nodes
                .get((id - 2) as usize)
                .and_then(|record| record.proof),
        }
    }

    fn push_node(
        &mut self,
        node: DomainNode,
        proof: Option<Bdd>,
        complement: Option<ControlDomain>,
    ) -> Result<ControlDomain, DomainError> {
        let len = self.nodes.len().saturating_add(2);
        let id = u32::try_from(len).map_err(|_| DomainError::capacity(len))?;
        self.nodes.push(DomainRecord {
            node,
            proof,
            complement,
        });
        Ok(ControlDomain(id))
    }

    fn proof(&self, result: Result<Bdd, BddError>) -> Result<Option<Bdd>, DomainError> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.is_resource_limit() => {
                self.saturated.set(true);
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }
}

enum CompileTask<'a> {
    Visit(&'a BoolExpr),
    Not,
    And(usize),
    Or(usize),
}

#[cfg(test)]
#[path = "../../../tests/internal/control_domain_joins.rs"]
mod tests;
