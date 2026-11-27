//! Structured region formation from predicated semantic fragments.

use crate::ir::{
    semantic::SemanticCompletion, BlockId, BoolVariable, InstructionId, SemanticBlock,
    SemanticExpressionFacts, SemanticFoldError, SemanticFolder, SemanticNode, SemanticPredicate,
    SemanticStatement, SemanticVisitor,
};

use super::ValueRecoveryError;

mod atoms;
mod distribution;
mod guarded_assignment;

use atoms::{AtomClasses, AtomIdentity};
use distribution::GuardDistribution;
use guarded_assignment::GuardedAssignment;

pub(super) struct PredicateRegionFormation<'a> {
    queries: &'a mut PredicateQueries,
    changed: bool,
}

impl PredicateRegionFormation<'_> {
    #[cfg(test)]
    pub(super) fn apply(root: &mut SemanticNode) -> Result<bool, ValueRecoveryError> {
        Self::apply_with_queries(root, &mut PredicateQueries::default())
    }

    pub(super) fn apply_with_queries(
        root: &mut SemanticNode,
        queries: &mut PredicateQueries,
    ) -> Result<bool, ValueRecoveryError> {
        if crate::ir::trivial_early_returns() && !needs_predicate_regions(root) {
            return Ok(false);
        }
        let before = SemanticCompletion::analyze(root);
        let mut changed = false;
        loop {
            let mut formation = PredicateRegionFormation {
                changed: false,
                queries,
            };
            let body = std::mem::replace(root, SemanticNode::Empty);
            *root = formation.fold_node(body)?;
            if !formation.changed {
                break;
            }
            changed = true;
        }
        if changed {
            // Only a rewrite can change completion; verifying an untouched
            // tree would compare it against itself.
            let after = SemanticCompletion::analyze(root);
            if before != after {
                return Err(SemanticFoldError::CompletionChanged {
                    transform: "predicate-region-formation",
                }
                .into());
            }
        }
        Ok(changed)
    }

    fn form_sequence(
        &mut self,
        nodes: Vec<SemanticNode>,
    ) -> Result<SemanticNode, ValueRecoveryError> {
        if !has_guarded_tail(&nodes) {
            return Ok(SemanticNode::sequence(nodes));
        }
        let mut formed = Vec::<SemanticNode>::with_capacity(nodes.len());
        for mut node in nodes {
            if let (
                Some(SemanticNode::If {
                    condition,
                    then_node,
                    else_node: None,
                }),
                SemanticNode::If {
                    condition: next_condition,
                    then_node: next_then,
                    else_node: None,
                },
            ) = (formed.last_mut(), &mut node)
            {
                if self.can_extend(condition, then_node, next_condition)? {
                    let first = SemanticCompletion::analyze(then_node);
                    let second = SemanticCompletion::analyze(next_then);
                    let original = SemanticCompletion::sequence([
                        SemanticCompletion::branch(condition, first.clone(), None),
                        SemanticCompletion::branch(next_condition, second.clone(), None),
                    ]);
                    let merged = SemanticCompletion::branch(
                        condition,
                        SemanticCompletion::sequence([first, second]),
                        None,
                    );
                    if original == merged {
                        let first = std::mem::replace(then_node.as_mut(), SemanticNode::Empty);
                        let second = std::mem::replace(next_then.as_mut(), SemanticNode::Empty);
                        **then_node = SemanticNode::sequence([first, second]);
                        self.changed = true;
                        continue;
                    }
                }
            }

            if let (
                Some(last),
                SemanticNode::If {
                    condition,
                    then_node,
                    else_node: None,
                },
            ) = (formed.last_mut(), &node)
            {
                if let Some(distributed) = GuardDistribution::apply(last, condition, then_node)? {
                    *last = distributed;
                    self.changed = true;
                    continue;
                }
            }
            formed.push(node);
        }
        Ok(SemanticNode::sequence(formed))
    }

    /// Splits a block around every definition whose selection is guarded.
    ///
    /// The recovered regions are emitted into the enclosing sequence, where
    /// region formation can merge them with the neighbours that share their
    /// predicate instead of leaving each merge to restate it.
    fn form_block(&mut self, block: SemanticBlock) -> SemanticNode {
        if block.statements.is_empty() {
            return SemanticNode::Empty;
        }
        if !block
            .statements
            .iter()
            .any(GuardedAssignment::is_self_selecting)
        {
            return SemanticNode::BasicBlock(block);
        }
        let id = block.id;
        let mut formed = Vec::<SemanticNode>::new();
        let mut pending = Vec::<SemanticStatement>::new();
        for statement in block.statements {
            match GuardedAssignment::recover(statement, id) {
                Ok(region) => {
                    formed.push(Self::fragment(id, std::mem::take(&mut pending)));
                    formed.push(region);
                    self.changed = true;
                }
                Err(statement) => pending.push(statement),
            }
        }
        formed.push(Self::fragment(id, pending));
        SemanticNode::sequence(formed)
    }

    fn fragment(id: BlockId, statements: Vec<SemanticStatement>) -> SemanticNode {
        if statements.is_empty() {
            return SemanticNode::Empty;
        }
        SemanticNode::BasicBlock(SemanticBlock { id, statements })
    }

    fn can_extend(
        &mut self,
        condition: &SemanticPredicate,
        body: &SemanticNode,
        next_condition: &SemanticPredicate,
    ) -> Result<bool, ValueRecoveryError> {
        if !Self::predicate_is_pure(condition)
            || !self.queries.equivalent(condition, next_condition)?
        {
            return Ok(false);
        }
        let predicate = SemanticExpressionFacts::of_predicate(condition);
        let body = SemanticExpressionFacts::of_node(body);
        Ok(predicate
            .used_variables()
            .all(|variable| body.definition_count(variable) == 0)
            && predicate
                .used_ssa_variables()
                .all(|variable| body.ssa_definition_count(variable) == 0))
    }

    fn predicate_is_pure(predicate: &SemanticPredicate) -> bool {
        let (predicate, _) = predicate.without_negations();
        match predicate {
            SemanticPredicate::True | SemanticPredicate::False => return true,
            SemanticPredicate::Test(test) => return test.effects().without_control().is_pure(),
            _ => {}
        }
        let mut pending = vec![predicate];
        while let Some(predicate) = pending.pop() {
            match predicate {
                SemanticPredicate::Test(instruction) => {
                    if !instruction.effects().without_control().is_pure() {
                        return false;
                    }
                }
                SemanticPredicate::Not(inner) => pending.push(inner),
                SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                    pending.extend(terms);
                }
                SemanticPredicate::True | SemanticPredicate::False => {}
            }
        }
        true
    }
}

fn has_guarded_tail(nodes: &[SemanticNode]) -> bool {
    nodes.iter().skip(1).any(|node| {
        matches!(
            node,
            SemanticNode::If {
                else_node: None,
                ..
            }
        )
    })
}

fn needs_predicate_regions(root: &SemanticNode) -> bool {
    #[derive(Default)]
    struct Sites {
        needs_fold: bool,
        has_if: bool,
        needs_normalization: bool,
    }
    impl SemanticVisitor for Sites {
        fn visits_values(&self) -> bool {
            false
        }

        fn enter_node(&mut self, node: &SemanticNode) {
            if self.needs_fold {
                return;
            }
            match node {
                SemanticNode::BasicBlock(block) => {
                    self.needs_normalization |= block.statements.is_empty();
                    self.needs_fold = block
                        .statements
                        .iter()
                        .any(GuardedAssignment::is_self_selecting);
                }
                SemanticNode::Sequence(children) => {
                    self.needs_fold = has_guarded_tail(children);
                    self.needs_normalization |= children.len() < 2
                        || children.iter().any(|child| {
                            matches!(child, SemanticNode::Empty | SemanticNode::Sequence(_))
                        });
                }
                SemanticNode::If { .. } => self.has_if = true,
                // The original pass also enters on a self-selecting loop
                // statement, then canonicalizes any blocks and sequences.
                SemanticNode::For { init, update, .. } => {
                    self.needs_fold = GuardedAssignment::is_self_selecting(init)
                        || GuardedAssignment::is_self_selecting(update);
                }
                _ => {}
            }
            // An isolated branch has no region pairing, but still activates
            // the existing normalization of empty blocks and nested sequences.
            self.needs_fold |= self.has_if && self.needs_normalization;
        }
    }
    let mut sites = Sites::default();
    sites.visit_node(root);
    sites.needs_fold
}

#[derive(Default, Debug)]
pub(super) struct PredicateQueries {
    answers: foldhash::HashMap<PredicatePair, bool>,
    tokens: usize,
}

impl PredicateQueries {
    fn equivalent(
        &mut self,
        left: &SemanticPredicate,
        right: &SemanticPredicate,
    ) -> Result<bool, ValueRecoveryError> {
        let (left_atom, left_negated) = left.without_negations();
        let (right_atom, right_negated) = right.without_negations();
        if let (SemanticPredicate::Test(first), SemanticPredicate::Test(second)) =
            (left_atom, right_atom)
        {
            // A pair of distinct instructions has at most two symbols. Their
            // equivalence is exactly the interner's operand comparison.
            if AtomIdentity::Source.equivalent(first, second) {
                return Ok(left_negated == right_negated);
            }
            if first.id != second.id {
                return Ok(false);
            }
        }
        let classes = PredicateEquivalence::new(left, right);
        if classes.literal_joins_differ(left, right) {
            return Ok(false);
        }
        let key = classes.query_key(left, right);
        if let Some(result) = key.as_ref().and_then(|key| self.answers.get(key)) {
            return Ok(*result);
        }
        let result = match classes.equivalent(left, right)? {
            Ok(equivalent) => equivalent,
            Err(error) if error.is_resource_limit() => false,
            Err(error) => return Err(SemanticFoldError::from(error).into()),
        };
        if let Some(key) = key {
            // The table belongs to this method. Eviction only requests a fresh
            // proof under the same local BDD limits.
            if self.tokens + key.len() > 8_192 {
                self.answers.clear();
                self.tokens = 0;
            }
            self.tokens += key.len();
            self.answers.insert(key, result);
        }
        Ok(result)
    }
}

type PredicatePair = smallvec::SmallVec<[PredicateToken; 12]>;

#[derive(Debug, Hash, PartialEq, Eq)]
enum PredicateToken {
    True,
    False,
    Atom(u32),
    Not,
    And(u16),
    Or(u16),
}

struct PredicateEquivalence {
    symbols: std::collections::BTreeMap<InstructionId, usize>,
    numeric: bool,
}

impl PredicateEquivalence {
    fn new(left: &SemanticPredicate, right: &SemanticPredicate) -> Self {
        let mut atoms = AtomClasses::new(AtomIdentity::Source);
        let mut symbols = std::collections::BTreeMap::new();
        for predicate in [left, right] {
            let mut pending = vec![predicate];
            while let Some(predicate) = pending.pop() {
                match predicate {
                    SemanticPredicate::Test(instruction) => {
                        symbols.insert(instruction.id, atoms.intern(instruction));
                    }
                    SemanticPredicate::Not(inner) => pending.push(inner),
                    SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                        pending.extend(terms);
                    }
                    SemanticPredicate::True | SemanticPredicate::False => {}
                }
            }
        }
        let count = atoms.len();
        let numeric = u32::try_from(count.saturating_sub(1)).is_ok();
        if numeric && count > 10 {
            let ranks = Self::decimal_ranks(count);
            for identity in symbols.values_mut() {
                *identity = ranks[*identity];
            }
        }
        Self { symbols, numeric }
    }

    // A consistent conjunction of literals is characterized by the assignment
    // it requires. Disjunction has the dual property. Distinct consistent
    // sets, or just one contradictory set, give an exact counterexample. An
    // equal pair still needs the ordinary proof under its local BDD limits.
    fn literal_joins_differ(&self, left: &SemanticPredicate, right: &SemanticPredicate) -> bool {
        let (left, right) = match (left, right) {
            (SemanticPredicate::And(left), SemanticPredicate::And(right))
            | (SemanticPredicate::Or(left), SemanticPredicate::Or(right)) => (left, right),
            _ => return false,
        };
        let mut signs = smallvec::SmallVec::<[u8; 16]>::from_elem(0, self.symbols.len());
        let mut contradictory = [false; 2];
        for (side, terms) in [left, right].into_iter().enumerate() {
            for term in terms {
                let (atom, negated) = term.without_negations();
                let SemanticPredicate::Test(operation) = atom else {
                    return false;
                };
                let Some(signs) = self
                    .symbols
                    .get(&operation.id)
                    .and_then(|id| signs.get_mut(*id))
                else {
                    return false;
                };
                let shift = 2 * side;
                *signs |= (1 << usize::from(negated)) << shift;
                contradictory[side] |= (*signs >> shift) & 3 == 3;
            }
        }
        contradictory[0] != contradictory[1]
            || (!contradictory[0] && signs.iter().any(|&row| row & 3 != row >> 2))
    }

    /// The existing local BDD namespace sorts numeric suffixes as strings:
    /// 1, 10, 11, ..., 2. Enumerate that decimal trie once so integer symbols
    /// have identical relative order and encounter the same proof budgets.
    fn decimal_ranks(count: usize) -> Vec<usize> {
        let mut ranks = vec![0; count];
        let mut number = 1;
        for rank in 1..count {
            ranks[number] = rank;
            if number <= (count - 1) / 10 {
                number *= 10;
            } else {
                while number % 10 == 9 || number == count - 1 {
                    number /= 10;
                }
                number += 1;
            }
        }
        ranks
    }

    /// A fresh proof consumes only these two formulas and their atom order.
    /// Prefix syntax and child counts uniquely encode both roots. Source
    /// operations, register facts, and instruction IDs stay out of the cache.
    fn query_key(
        &self,
        left: &SemanticPredicate,
        right: &SemanticPredicate,
    ) -> Option<PredicatePair> {
        if !self.numeric {
            return None;
        }
        const MAX_TOKENS: usize = 1_024;
        let mut pending: smallvec::SmallVec<[&SemanticPredicate; 8]> =
            smallvec::smallvec![right, left];
        let mut key = PredicatePair::new();
        while let Some(predicate) = pending.pop() {
            if key.len() == MAX_TOKENS {
                return None;
            }
            key.push(match predicate {
                SemanticPredicate::True => PredicateToken::True,
                SemanticPredicate::False => PredicateToken::False,
                SemanticPredicate::Test(operation) => {
                    PredicateToken::Atom(*self.symbols.get(&operation.id)? as u32)
                }
                SemanticPredicate::Not(child) => {
                    pending.push(child);
                    PredicateToken::Not
                }
                SemanticPredicate::And(children) | SemanticPredicate::Or(children) => {
                    if children.len() > MAX_TOKENS.saturating_sub(key.len() + pending.len() + 1) {
                        return None;
                    }
                    pending.extend(children.iter().rev());
                    let arity = children.len() as u16;
                    if matches!(predicate, SemanticPredicate::And(_)) {
                        PredicateToken::And(arity)
                    } else {
                        PredicateToken::Or(arity)
                    }
                }
            });
        }
        Some(key)
    }

    fn equivalent(
        &self,
        left: &SemanticPredicate,
        right: &SemanticPredicate,
    ) -> Result<Result<bool, crate::ir::bdd::BddError>, SemanticFoldError> {
        let symbol = |id| {
            self.symbols.get(&id).map(|&identity| {
                if self.numeric {
                    BoolVariable::Atom(identity as u32)
                } else {
                    BoolVariable::Named(format!("predicate-value:{identity}"))
                }
            })
        };
        let left = left.domain_with(symbol)?;
        let right = right.domain_with(symbol)?;
        Ok(left.equivalent(&right))
    }
}

impl SemanticFolder for PredicateRegionFormation<'_> {
    type Error = ValueRecoveryError;

    fn finish_node(&mut self, node: SemanticNode) -> Result<SemanticNode, Self::Error> {
        match node {
            SemanticNode::Sequence(nodes) => self.form_sequence(nodes),
            SemanticNode::BasicBlock(block) => Ok(self.form_block(block)),
            node => Ok(node),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/predicate_regions.rs"]
mod tests;
