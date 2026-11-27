//! Pattern-independent construction of acyclic semantic regions.
//!
//! The query stage computes exact reaching conditions and clusters nodes by
//! Boolean implication plus graph reachability. The construction stage is a
//! separate, mechanical translation from the resulting fact tree to semantic IR.

use foldhash::HashMap;
use smallvec::{smallvec, SmallVec};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use crate::ir::analysis::{
    DominanceError, DominatorTree, ReachingConditionError, ReachingConditions,
};
use crate::ir::bdd::Bdd;
use crate::ir::semantic::{ControlCompletion, SemanticFactory};
use crate::ir::{
    BlockId, BoolExpr, BoolVariable, EdgeKind, InsnType, SemanticBuildError, SemanticNode,
    SemanticPredicate, CFG,
};

pub(super) enum AcyclicStructure {
    Structured(SemanticNode),
    RequiresGraphReduction(AcyclicReductionReason),
}

#[derive(Debug)]
pub(super) enum AcyclicReductionReason {
    ReachingConditions(ReachingConditionError),
    ConditionExpression,
}

pub(super) trait AcyclicSeeds {
    fn body(&self, block: BlockId) -> Option<&SemanticNode>;
    fn take(&mut self, block: BlockId) -> Option<SemanticNode>;

    fn predicate(
        &self,
        semantic: &SemanticFactory<'_>,
        condition: BoolExpr,
    ) -> Result<SemanticPredicate, SemanticBuildError> {
        semantic.predicate(condition)
    }
}

impl AcyclicSeeds for BTreeMap<BlockId, SemanticNode> {
    fn body(&self, block: BlockId) -> Option<&SemanticNode> {
        self.get(&block)
    }

    fn take(&mut self, block: BlockId) -> Option<SemanticNode> {
        self.remove(&block)
    }
}

pub(super) struct AcyclicStructurer<'a, 'semantic, S> {
    cfg: &'a CFG,
    semantic: &'semantic SemanticFactory<'semantic>,
    seeded: &'a mut S,
}

impl<'a, 'semantic, S: AcyclicSeeds> AcyclicStructurer<'a, 'semantic, S> {
    pub(super) fn new(
        cfg: &'a CFG,
        semantic: &'semantic SemanticFactory<'semantic>,
        seeded: &'a mut S,
    ) -> Self {
        Self {
            cfg,
            semantic,
            seeded,
        }
    }

    /// An unconditional chain has one reaching condition (true) and its
    /// dominator tree is the chain itself. Construct the same ordered block
    /// facts directly, leaving ordinary lowering in charge of seeds and DEX
    /// statements. Bounded traversal also rejects cycles and detached blocks.
    pub(super) fn linear(&mut self) -> Result<Option<SemanticNode>, AcyclicStructureError> {
        let count = self.cfg.num_graph_nodes();
        let mut block = self.cfg.entry;
        let mut facts = Vec::new();
        loop {
            if facts.len() == count || !self.cfg.is_graph_node(block) {
                return Ok(None);
            }
            let Some(body) = self.cfg.block(block) else {
                return Ok(None);
            };
            if body
                .terminator()
                .is_some_and(|insn| matches!(insn.insn_type, InsnType::If | InsnType::Switch))
            {
                return Ok(None);
            }
            facts.push(AcyclicStructureFact::Block {
                block,
                prefix_only: false,
            });
            match self.cfg.successors_with_kind(block) {
                [] if facts.len() == count => break,
                [(target, EdgeKind::Normal)] => block = *target,
                _ => return Ok(None),
            }
        }
        let shape = AcyclicStructureFact::sequence(facts);
        AcyclicStructureLowering::new(self.semantic, self.seeded)
            .lower(shape)
            .map(Some)
    }

    pub(super) fn structure(&mut self) -> Result<AcyclicStructure, AcyclicStructureError> {
        if let Some(node) = self.linear()? {
            return Ok(AcyclicStructure::Structured(node));
        }
        self.structure_branches()
    }

    pub(super) fn structure_branches(&mut self) -> Result<AcyclicStructure, AcyclicStructureError> {
        let facts = match ReachingConditions::analyze(self.cfg) {
            Ok(facts) => facts,
            Err(error) if error.requires_graph_reduction() => {
                return Ok(AcyclicStructure::RequiresGraphReduction(
                    AcyclicReductionReason::ReachingConditions(error),
                ));
            }
            Err(error) => return Err(error.into()),
        };
        let closed = self
            .cfg
            .graph_nodes()
            .filter(|block| self.cfg.normal_successors(*block).next().is_none())
            .filter(|block| self.is_closed_block(*block))
            .collect();
        let structure = match AcyclicStructureAnalysis::new(self.cfg, &facts, closed).analyze() {
            Ok(structure) => structure,
            Err(AcyclicStructureError::ReachingCondition(error))
                if error.requires_graph_reduction() =>
            {
                return Ok(AcyclicStructure::RequiresGraphReduction(
                    AcyclicReductionReason::ReachingConditions(error),
                ));
            }
            Err(error) => return Err(error),
        };
        let Some(structure) = structure else {
            return Ok(AcyclicStructure::RequiresGraphReduction(
                AcyclicReductionReason::ConditionExpression,
            ));
        };
        Ok(AcyclicStructure::Structured(
            AcyclicStructureLowering::new(self.semantic, &mut *self.seeded).lower(structure)?,
        ))
    }

    fn is_closed_block(&self, block: BlockId) -> bool {
        if let Some(node) = self.seeded.body(block) {
            return !ControlCompletion::normally_completes(node);
        }
        self.cfg
            .block(block)
            .and_then(|body| body.terminator())
            .is_some_and(|terminator| {
                matches!(terminator.insn_type, InsnType::Return | InsnType::Throw)
            })
    }
}

struct AcyclicStructureAnalysis<'a> {
    cfg: &'a CFG,
    facts: &'a ReachingConditions,
    closed: BTreeSet<BlockId>,
    expression_budget: Cell<usize>,
}

impl<'a> AcyclicStructureAnalysis<'a> {
    fn new(cfg: &'a CFG, facts: &'a ReachingConditions, closed: BTreeSet<BlockId>) -> Self {
        let expression_budget = facts.order().len().saturating_mul(64).clamp(256, 32_768);
        Self {
            cfg,
            facts,
            closed,
            expression_budget: Cell::new(expression_budget),
        }
    }

    fn analyze(&self) -> Result<Option<AcyclicStructureFact>, AcyclicStructureError> {
        let dominators = DominatorTree::for_reaching_conditions(self.cfg, self.facts)?;
        let mut subtrees = BTreeMap::<BlockId, StructureItem>::new();
        let mut queries = BranchQueries::default();
        for block in dominators.postorder() {
            let assumption = ConditionDomain {
                domain: self
                    .facts
                    .condition(block)
                    .ok_or(AcyclicStructureError::MissingCondition(block))?
                    .domain(),
            };
            let mut children = dominators
                .children(block)
                .map(|child| {
                    subtrees
                        .remove(&child)
                        .ok_or(AcyclicStructureError::MissingDominatorSubtree(child))
                })
                .collect::<Result<Vec<_>, _>>()?;
            children.sort_by_key(|child| child.lexical_rank);
            let (descendants, mut terminals) = if children.is_empty() {
                (AcyclicStructureFact::Empty, BTreeSet::new())
            } else {
                let Some(structure) = self.refine(children, assumption.clone(), &mut queries)?
                else {
                    return Ok(None);
                };
                structure
            };
            if self.closed.contains(&block) {
                terminals.insert(block);
            }
            let block_structure = self.block_structure(block)?;
            let rank = self
                .facts
                .rank(block)
                .ok_or(AcyclicStructureError::MissingRank(block))?;
            subtrees.insert(
                block,
                StructureItem {
                    condition: assumption,
                    roots: smallvec![rank],
                    terminals,
                    lexical_rank: rank,
                    structure: AcyclicStructureFact::sequence(vec![block_structure, descendants]),
                },
            );
        }
        let root = subtrees.remove(&self.cfg.entry).ok_or(
            AcyclicStructureError::MissingDominatorSubtree(self.cfg.entry),
        )?;
        if !subtrees.is_empty() {
            return Err(AcyclicStructureError::MalformedDominatorTree);
        }
        Ok(Some(root.structure))
    }

    fn refine(
        &self,
        items: Vec<StructureItem>,
        assumption: ConditionDomain,
        queries: &mut BranchQueries,
    ) -> Result<Option<Refined>, AcyclicStructureError> {
        let mut tasks = vec![RefineTask::Refine {
            items,
            assumption,
            first_variable: 0,
        }];
        let mut results = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                RefineTask::Refine {
                    items,
                    assumption,
                    first_variable,
                } => {
                    let variables = if assumption.domain == self.facts.truth() {
                        &mut queries.root
                    } else {
                        queries.assumed.entry(assumption.domain).or_default()
                    };
                    let Some(cluster) = self.cluster(
                        &items,
                        &assumption,
                        first_variable,
                        variables,
                        &mut queries.literals,
                    )?
                    else {
                        let Some(structure) = self.finish(items, &assumption)? else {
                            return Ok(None);
                        };
                        results.push(structure);
                        continue;
                    };
                    let condition = cluster.condition;
                    let condition_domain = self.facts.compile(&condition)?;
                    let negated_domain = self.facts.not(condition_domain)?;
                    let true_assumption = ConditionDomain {
                        domain: self.facts.and(assumption.domain, condition_domain)?,
                    };
                    let false_assumption = ConditionDomain {
                        domain: self.facts.and(assumption.domain, negated_domain)?,
                    };
                    let mut when_true = Vec::new();
                    let mut when_false = Vec::new();
                    let mut remaining = Vec::new();
                    for (item, class) in items.into_iter().zip(cluster.classes) {
                        match class {
                            BranchClass::True => when_true.push(item),
                            BranchClass::False => when_false.push(item),
                            BranchClass::Remaining => remaining.push(item),
                        }
                    }
                    for item in &mut when_true {
                        item.condition.domain = self
                            .facts
                            .and(item.condition.domain, true_assumption.domain)?;
                    }
                    for item in &mut when_false {
                        item.condition.domain = self
                            .facts
                            .and(item.condition.domain, false_assumption.domain)?;
                    }
                    tasks.push(RefineTask::Resume {
                        remaining,
                        insertion: cluster.insertion,
                        assumption: assumption.clone(),
                        condition,
                        roots: cluster.roots,
                        first_variable: cluster.first_variable,
                        lexical_rank: cluster.lexical_rank,
                        has_false_branch: !when_false.is_empty(),
                    });
                    tasks.push(RefineTask::Refine {
                        items: when_false,
                        assumption: false_assumption,
                        first_variable: 0,
                    });
                    tasks.push(RefineTask::Refine {
                        items: when_true,
                        assumption: true_assumption,
                        first_variable: 0,
                    });
                }
                RefineTask::Resume {
                    mut remaining,
                    insertion,
                    assumption,
                    condition,
                    roots,
                    first_variable,
                    lexical_rank,
                    has_false_branch,
                } => {
                    let (when_false, mut terminals) = results
                        .pop()
                        .ok_or(AcyclicStructureError::MalformedWorkStack)?;
                    let (when_true, other_terminals) = results
                        .pop()
                        .ok_or(AcyclicStructureError::MalformedWorkStack)?;
                    Self::merge_terminals(&mut terminals, other_terminals);
                    let branch = AcyclicStructureFact::If {
                        condition,
                        when_true: Box::new(when_true),
                        when_false: has_false_branch.then(|| Box::new(when_false)),
                    };
                    remaining.insert(
                        insertion,
                        StructureItem {
                            condition: assumption.clone(),
                            roots,
                            terminals,
                            lexical_rank,
                            structure: branch,
                        },
                    );
                    tasks.push(RefineTask::Refine {
                        items: remaining,
                        assumption,
                        first_variable,
                    });
                }
            }
        }
        if results.len() != 1 {
            return Err(AcyclicStructureError::MalformedWorkStack);
        }
        results
            .pop()
            .map(Some)
            .ok_or(AcyclicStructureError::MalformedWorkStack)
    }

    fn block_structure(
        &self,
        block: BlockId,
    ) -> Result<AcyclicStructureFact, AcyclicStructureError> {
        let body = self
            .cfg
            .block(block)
            .ok_or(AcyclicStructureError::MissingBlock(block))?;
        let prefix_only = body.terminator().is_some_and(|terminator| {
            matches!(terminator.insn_type, InsnType::If | InsnType::Switch)
        });
        Ok(AcyclicStructureFact::Block { block, prefix_only })
    }

    fn cluster(
        &self,
        items: &[StructureItem],
        assumption: &ConditionDomain,
        first_variable: usize,
        variables: &mut BranchVariables,
        literals: &mut Vec<Option<(Bdd, Bdd)>>,
    ) -> Result<Option<ConditionCluster>, AcyclicStructureError> {
        // A singleton cannot partition. Its first complete search still issues
        // all BDD queries in order, including those that charge cache entries.
        if let [item] = items {
            if variables.last_singleton == Some(item.condition.domain) {
                return Ok(None);
            }
        }
        if items.is_empty() && variables.tested == self.facts.branch_variables().len() {
            return Ok(None);
        }
        let mut classes = Vec::new();
        let mut next = first_variable;
        // After a partition, the remainder contains only old items and one
        // item at the enclosing assumption. An eligible variable already
        // tested that domain in both directions. A variable with fewer than
        // two members stays ineligible. An earlier incompatible ordering
        // can become valid as members leave.
        let mut retry_from = None;
        // These items have the enclosing assumption's domain. Eligibility
        // already issued both implication queries and found neither true.
        let unconditional = items
            .iter()
            .all(|item| item.condition.domain == assumption.domain);
        if unconditional {
            next = variables.available.len();
        }
        while let Some((condition, domain, negated)) =
            variables.candidate(next, assumption.domain, self.facts, literals)?
        {
            next += 1;
            if unconditional {
                continue;
            }
            let (positive, negative) =
                self.classify_items(items, assumption.domain, domain, negated, &mut classes)?;
            if positive + negative < 2 {
                continue;
            }
            if let Some(cluster) = self.partition(
                BoolExpr::block(condition),
                self.facts
                    .rank(condition)
                    .ok_or(AcyclicStructureError::MissingRank(condition))?,
                items,
                &mut classes,
                retry_from.unwrap_or(next),
            ) {
                return Ok(Some(cluster));
            }
            retry_from.get_or_insert(next - 1);
        }
        for (index, candidate) in items.iter().enumerate() {
            if items[..index]
                .iter()
                .any(|previous| previous.condition.domain == candidate.condition.domain)
            {
                continue;
            }
            let domain = candidate.condition.domain;
            let negated = self.facts.not(domain)?;
            if self.facts.implies(assumption.domain, domain)?
                || self.facts.implies(assumption.domain, negated)?
            {
                continue;
            }
            let (positive, negative) =
                self.classify_items(items, assumption.domain, domain, negated, &mut classes)?;
            if positive == 0 || negative == 0 {
                continue;
            }
            let remaining = self.expression_budget.get();
            let relative = self.facts.constrain(domain, assumption.domain)?;
            let Some((condition, nodes)) = self.facts.expression(relative, remaining)? else {
                continue;
            };
            if let Some(cluster) = self.partition(
                condition,
                candidate.lexical_rank,
                items,
                &mut classes,
                retry_from.unwrap_or(next),
            ) {
                self.expression_budget.set(remaining.saturating_sub(nodes));
                return Ok(Some(cluster));
            }
        }
        if let [item] = items {
            variables.last_singleton = Some(item.condition.domain);
        }
        Ok(None)
    }

    fn classify_items(
        &self,
        items: &[StructureItem],
        assumption: Bdd,
        condition: Bdd,
        negated: Bdd,
        classes: &mut Vec<BranchClass>,
    ) -> Result<(usize, usize), AcyclicStructureError> {
        classes.clear();
        classes.reserve(items.len());
        let mut positive = 0;
        let mut negative = 0;
        for item in items {
            let class = if item.condition.domain == assumption {
                BranchClass::Remaining
            } else if self.facts.implies(item.condition.domain, condition)? {
                positive += 1;
                BranchClass::True
            } else if self.facts.implies(item.condition.domain, negated)? {
                negative += 1;
                BranchClass::False
            } else {
                BranchClass::Remaining
            };
            classes.push(class);
        }
        Ok((positive, negative))
    }

    fn partition(
        &self,
        condition: BoolExpr,
        condition_rank: usize,
        items: &[StructureItem],
        classes: &mut Vec<BranchClass>,
        first_variable: usize,
    ) -> Option<ConditionCluster> {
        // Items are unions of complete sibling dominator subtrees. If a
        // member of A reaches one of B, a path to that source avoiding B
        // forces the suffix to pass through B's root. A's root reaches both.
        // It suffices to keep the sibling roots through every partition.
        let mut roots = SmallVec::new();
        let mut remaining = Vec::new();
        let mut lexical_rank = condition_rank;
        for (item, class) in items.iter().zip(classes.iter()) {
            if matches!(class, BranchClass::Remaining) {
                remaining.push(item);
            } else {
                roots.extend_from_slice(&item.roots);
                lexical_rank = lexical_rank.min(item.lexical_rank);
            }
        }
        // Each DAG path assigns independent literals at distinct blocks.
        // A path between opposite arms would satisfy both the predicate and
        // its negation.
        let mut lower = 0usize;
        let mut upper = remaining.len();
        for (index, item) in remaining.iter().enumerate() {
            if self.facts.any_rank_reaches(&item.roots, &roots) {
                lower = lower.max(index + 1);
            }
            if self.facts.any_rank_reaches(&roots, &item.roots) {
                upper = upper.min(index);
            }
        }
        if lower > upper {
            return None;
        }
        let desired = remaining
            .iter()
            .take_while(|item| item.lexical_rank < lexical_rank)
            .count();
        let insertion = desired.clamp(lower, upper);
        Some(ConditionCluster {
            condition,
            classes: std::mem::take(classes),
            insertion,
            roots,
            first_variable,
            lexical_rank,
        })
    }

    fn finish(
        &self,
        items: Vec<StructureItem>,
        assumption: &ConditionDomain,
    ) -> Result<Option<Refined>, AcyclicStructureError> {
        let mut terminals = BTreeSet::new();
        let mut structures = Vec::with_capacity(items.len());
        let mut active = assumption.domain;
        for item in items {
            let structure = if self.facts.implies(active, item.condition.domain)? {
                item.structure
            } else {
                let remaining = self.expression_budget.get();
                let relative = self.facts.constrain(item.condition.domain, active)?;
                let Some((condition, nodes)) = self.facts.expression(relative, remaining)? else {
                    return Ok(None);
                };
                self.expression_budget.set(remaining.saturating_sub(nodes));
                AcyclicStructureFact::Guard {
                    condition,
                    body: Box::new(item.structure),
                }
            };
            let terminal = item.terminals.iter().copied().try_fold(
                self.facts.not(self.facts.truth())?,
                |domain, block| -> Result<Bdd, AcyclicStructureError> {
                    let reached = self
                        .facts
                        .condition(block)
                        .ok_or(AcyclicStructureError::MissingCondition(block))?
                        .domain();
                    Ok(self.facts.or(domain, reached)?)
                },
            )?;
            active = self.facts.and(active, self.facts.not(terminal)?)?;
            structures.push(structure);
            Self::merge_terminals(&mut terminals, item.terminals);
        }
        Ok(Some((
            AcyclicStructureFact::sequence(structures),
            terminals,
        )))
    }

    fn merge_terminals(terminals: &mut BTreeSet<BlockId>, mut other: BTreeSet<BlockId>) {
        if other.len() > terminals.len() {
            std::mem::swap(terminals, &mut other);
        }
        terminals.extend(other);
    }
}

#[derive(Clone)]
struct ConditionDomain {
    domain: Bdd,
}

/// Reaching conditions and their ordered variables belong to one BDD arena.
#[derive(Default)]
struct BranchQueries {
    root: BranchVariables,
    assumed: HashMap<Bdd, BranchVariables>,
    literals: Vec<Option<(Bdd, Bdd)>>,
}

/// Each assumption keeps the eligible prefix that has already been examined.
/// Extend it lazily: first queries consume bounded BDD entries even for
/// rejected variables. Every eligible index already has two charged implication
/// pairs in the BDD cache. Its literals are shared by all assumptions.
#[derive(Default)]
struct BranchVariables {
    tested: usize,
    available: Vec<usize>,
    last_singleton: Option<Bdd>,
}

impl BranchVariables {
    fn candidate(
        &mut self,
        rank: usize,
        assumption: Bdd,
        facts: &ReachingConditions,
        literals: &mut Vec<Option<(Bdd, Bdd)>>,
    ) -> Result<Option<(BlockId, Bdd, Bdd)>, AcyclicStructureError> {
        let variables = facts.branch_variables();
        if let Some(&index) = self.available.get(rank) {
            let (domain, negated) =
                literals[index].ok_or(AcyclicStructureError::MissingCondition(variables[index]))?;
            return Ok(Some((variables[index], domain, negated)));
        }
        for (index, &condition) in variables.iter().enumerate().skip(self.tested) {
            self.tested += 1;
            let owner = facts
                .condition(condition)
                .ok_or(AcyclicStructureError::MissingCondition(condition))?;
            if !facts.implies(assumption, owner.domain())? {
                continue;
            }
            if index >= literals.len() {
                literals.resize(index + 1, None);
            }
            let (domain, negated) = match literals[index] {
                Some(literals) => literals,
                None => {
                    let domain = facts.compile(&BoolExpr::block(condition))?;
                    let negated = facts.not(domain)?;
                    literals[index] = Some((domain, negated));
                    (domain, negated)
                }
            };
            if facts.implies(assumption, domain)? || facts.implies(assumption, negated)? {
                continue;
            }
            self.available.push(index);
            return Ok(Some((condition, domain, negated)));
        }
        Ok(None)
    }
}

type Refined = (AcyclicStructureFact, BTreeSet<BlockId>);

struct StructureItem {
    condition: ConditionDomain,
    roots: SmallVec<[usize; 1]>,
    terminals: BTreeSet<BlockId>,
    lexical_rank: usize,
    structure: AcyclicStructureFact,
}

struct ConditionCluster {
    condition: BoolExpr,
    classes: Vec<BranchClass>,
    insertion: usize,
    roots: SmallVec<[usize; 1]>,
    first_variable: usize,
    lexical_rank: usize,
}

#[derive(Clone, Copy)]
enum BranchClass {
    True,
    False,
    Remaining,
}

#[derive(Debug)]
enum AcyclicStructureFact {
    Empty,
    Block {
        block: BlockId,
        prefix_only: bool,
    },
    Sequence(VecDeque<AcyclicStructureFact>),
    Guard {
        condition: BoolExpr,
        body: Box<AcyclicStructureFact>,
    },
    If {
        condition: BoolExpr,
        when_true: Box<AcyclicStructureFact>,
        when_false: Option<Box<AcyclicStructureFact>>,
    },
}

impl AcyclicStructureFact {
    fn sequence(structures: Vec<Self>) -> Self {
        let mut flattened = VecDeque::new();
        // Dominator lowering most often prepends one block to a long child
        // sequence. Keep the larger buffer when joining two sequences.
        for structure in structures.into_iter().rev() {
            match structure {
                Self::Empty => {}
                Self::Sequence(mut children) if children.len() > flattened.len() => {
                    children.extend(flattened);
                    flattened = children;
                }
                Self::Sequence(children) => {
                    flattened.reserve(children.len());
                    for child in children.into_iter().rev() {
                        flattened.push_front(child);
                    }
                }
                other => flattened.push_front(other),
            }
        }
        match flattened.len() {
            0 => Self::Empty,
            1 => flattened.pop_front().unwrap_or(Self::Empty),
            _ => Self::Sequence(flattened),
        }
    }
}

enum RefineTask {
    Refine {
        items: Vec<StructureItem>,
        assumption: ConditionDomain,
        first_variable: usize,
    },
    Resume {
        remaining: Vec<StructureItem>,
        insertion: usize,
        assumption: ConditionDomain,
        condition: BoolExpr,
        roots: SmallVec<[usize; 1]>,
        first_variable: usize,
        lexical_rank: usize,
        has_false_branch: bool,
    },
}

struct AcyclicStructureLowering<'a, 'semantic, S> {
    semantic: &'semantic SemanticFactory<'semantic>,
    seeded: &'a mut S,
}

impl<'a, 'semantic, S: AcyclicSeeds> AcyclicStructureLowering<'a, 'semantic, S> {
    fn new(semantic: &'semantic SemanticFactory<'semantic>, seeded: &'a mut S) -> Self {
        Self { semantic, seeded }
    }

    fn lower(
        &mut self,
        structure: AcyclicStructureFact,
    ) -> Result<SemanticNode, AcyclicStructureError> {
        let mut tasks = vec![LowerTask::Visit(structure)];
        let mut results = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                LowerTask::Visit(AcyclicStructureFact::Empty) => results.push(SemanticNode::Empty),
                LowerTask::Visit(AcyclicStructureFact::Block { block, prefix_only }) => {
                    results.push(match self.seeded.take(block) {
                        Some(node) => node,
                        None => self.semantic.block(block, prefix_only)?,
                    });
                }
                LowerTask::Visit(AcyclicStructureFact::Sequence(structures)) => {
                    tasks.push(LowerTask::Sequence(structures.len()));
                    tasks.extend(structures.into_iter().rev().map(LowerTask::Visit));
                }
                LowerTask::Visit(AcyclicStructureFact::Guard { condition, body }) => {
                    tasks.push(LowerTask::Guard(condition));
                    tasks.push(LowerTask::Visit(*body));
                }
                LowerTask::Visit(AcyclicStructureFact::If {
                    condition,
                    when_true,
                    when_false,
                }) => {
                    tasks.push(LowerTask::If {
                        condition,
                        has_false_branch: when_false.is_some(),
                    });
                    if let Some(when_false) = when_false {
                        tasks.push(LowerTask::Visit(*when_false));
                    }
                    tasks.push(LowerTask::Visit(*when_true));
                }
                LowerTask::Sequence(count) => {
                    let start = results
                        .len()
                        .checked_sub(count)
                        .ok_or(AcyclicStructureError::MalformedWorkStack)?;
                    let children = results.drain(start..).collect::<Vec<_>>();
                    results.push(SemanticNode::sequence(children));
                }
                LowerTask::Guard(condition) => {
                    let body = results
                        .pop()
                        .ok_or(AcyclicStructureError::MalformedWorkStack)?;
                    results.push(SemanticNode::branch(
                        self.seeded.predicate(self.semantic, condition)?,
                        body,
                        None,
                    ));
                }
                LowerTask::If {
                    condition,
                    has_false_branch,
                } => {
                    let when_false = if has_false_branch {
                        Some(
                            results
                                .pop()
                                .ok_or(AcyclicStructureError::MalformedWorkStack)?,
                        )
                    } else {
                        None
                    };
                    let when_true = results
                        .pop()
                        .ok_or(AcyclicStructureError::MalformedWorkStack)?;
                    results.push(SemanticNode::branch(
                        self.seeded.predicate(self.semantic, condition)?,
                        when_true,
                        when_false,
                    ));
                }
            }
        }
        if results.len() != 1 {
            return Err(AcyclicStructureError::MalformedWorkStack);
        }
        results
            .pop()
            .ok_or(AcyclicStructureError::MalformedWorkStack)
    }
}

pub(super) fn lower_mapped_predicate<'a>(
    condition: BoolExpr,
    predicate: impl Fn(BlockId) -> Option<&'a SemanticPredicate>,
) -> Result<SemanticPredicate, SemanticBuildError> {
    let mut pending = vec![PredicateTask::Visit(condition)];
    let mut results = Vec::new();
    while let Some(task) = pending.pop() {
        match task {
            PredicateTask::Visit(condition) => match condition {
                BoolExpr::True => results.push(SemanticPredicate::True),
                BoolExpr::False => results.push(SemanticPredicate::False),
                BoolExpr::Symbol(BoolVariable::Block(block)) => {
                    results.push(
                        predicate(block)
                            .cloned()
                            .ok_or(SemanticBuildError::MissingCondition(block))?,
                    );
                }
                BoolExpr::Symbol(variable) => {
                    return Err(SemanticBuildError::InvalidVariable(variable));
                }
                BoolExpr::Not(inner) => {
                    pending.push(PredicateTask::Not);
                    pending.push(PredicateTask::Visit(*inner));
                }
                BoolExpr::And(terms) => {
                    let count = terms.len();
                    pending.push(PredicateTask::Junction {
                        count,
                        conjunction: true,
                    });
                    pending.extend(terms.into_iter().rev().map(PredicateTask::Visit));
                }
                BoolExpr::Or(terms) => {
                    let count = terms.len();
                    pending.push(PredicateTask::Junction {
                        count,
                        conjunction: false,
                    });
                    pending.extend(terms.into_iter().rev().map(PredicateTask::Visit));
                }
            },
            PredicateTask::Not => {
                let inner = results
                    .pop()
                    .ok_or(SemanticBuildError::MalformedPredicate)?;
                results.push(inner.negate());
            }
            PredicateTask::Junction { count, conjunction } => {
                let start = results
                    .len()
                    .checked_sub(count)
                    .ok_or(SemanticBuildError::MalformedPredicate)?;
                let terms = results.drain(start..).collect();
                results.push(if conjunction {
                    SemanticPredicate::And(terms)
                } else {
                    SemanticPredicate::Or(terms)
                });
            }
        }
    }
    if results.len() != 1 {
        return Err(SemanticBuildError::MalformedPredicate);
    }
    results.pop().ok_or(SemanticBuildError::MalformedPredicate)
}

enum PredicateTask {
    Visit(BoolExpr),
    Not,
    Junction { count: usize, conjunction: bool },
}

enum LowerTask {
    Visit(AcyclicStructureFact),
    Sequence(usize),
    Guard(BoolExpr),
    If {
        condition: BoolExpr,
        has_false_branch: bool,
    },
}

#[derive(Debug)]
pub enum AcyclicStructureError {
    MissingBlock(BlockId),
    MissingCondition(BlockId),
    MissingRank(BlockId),
    MissingDominatorSubtree(BlockId),
    MalformedDominatorTree,
    MalformedWorkStack,
    Dominance(DominanceError),
    ReachingCondition(ReachingConditionError),
    Semantic(SemanticBuildError),
}

impl fmt::Display for AcyclicStructureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBlock(block) => write!(formatter, "missing acyclic block {block}"),
            Self::MissingCondition(block) => {
                write!(formatter, "block {block} has no reaching condition")
            }
            Self::MissingRank(block) => write!(formatter, "block {block} has no lexical rank"),
            Self::MissingDominatorSubtree(block) => {
                write!(formatter, "dominator subtree for block {block} is missing")
            }
            Self::MalformedDominatorTree => formatter.write_str("malformed dominator tree"),
            Self::MalformedWorkStack => {
                formatter.write_str("malformed acyclic structuring work stack")
            }
            Self::Dominance(source) => write!(formatter, "dominance analysis failed: {source}"),
            Self::ReachingCondition(source) => {
                write!(formatter, "reaching-condition analysis failed: {source}")
            }
            Self::Semantic(source) => write!(formatter, "semantic construction failed: {source}"),
        }
    }
}

impl std::error::Error for AcyclicStructureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Dominance(source) => Some(source),
            Self::ReachingCondition(source) => Some(source),
            Self::Semantic(source) => Some(source),
            _ => None,
        }
    }
}

impl From<DominanceError> for AcyclicStructureError {
    fn from(source: DominanceError) -> Self {
        Self::Dominance(source)
    }
}

impl From<ReachingConditionError> for AcyclicStructureError {
    fn from(source: ReachingConditionError) -> Self {
        Self::ReachingCondition(source)
    }
}

impl From<SemanticBuildError> for AcyclicStructureError {
    fn from(source: SemanticBuildError) -> Self {
        Self::Semantic(source)
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/acyclic_members.rs"]
mod tests;
