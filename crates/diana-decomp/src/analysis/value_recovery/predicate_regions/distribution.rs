use super::atoms::{AtomClasses, AtomIdentity};
use crate::ir::semantic::ControlCompletion;
use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    bdd::{Bdd, BddContext, BddError},
    semantic::SemanticCompletion,
    BoolVariable, InstructionId, SemanticFoldError, SemanticNode, SemanticOperand,
    SemanticPredicate,
};

pub(super) struct GuardDistribution;

impl GuardDistribution {
    const MAX_COPIES: usize = 8;
    const MAX_TAIL_STATEMENTS: usize = 4;

    pub(super) fn apply(
        original: &SemanticNode,
        guard: &SemanticOperand<SemanticPredicate>,
        tail: &SemanticNode,
    ) -> Result<Option<SemanticNode>, SemanticFoldError> {
        if matches!(original, SemanticNode::Empty)
            || !super::PredicateRegionFormation::predicate_is_pure(guard)
            || Self::linear_tail_size(tail).is_none()
        {
            return Ok(None);
        }
        let guard_cost = Self::predicate_cost(guard);
        if guard_cost < 3 {
            return Ok(None);
        }
        let domain = PredicateDomain::collect(original, guard)?;
        let guard_domain = match domain.compile(guard) {
            Ok(domain) => domain,
            Err(error) if error.is_resource_limit() => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let plan = match PathInjector::new(&domain, guard_domain).plan(original) {
            Ok(plan) => plan,
            Err(error) if error.is_resource_limit() => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if !(1..=Self::MAX_COPIES).contains(&plan.copies) {
            return Ok(None);
        }
        let covered = match domain.bdd.equivalent(plan.covered, guard_domain) {
            Ok(covered) => covered,
            Err(error) if error.is_resource_limit() => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if !covered {
            return Ok(None);
        }

        let old_cost = Self::node_cost(original)
            .saturating_add(guard_cost)
            .saturating_add(Self::node_cost(tail))
            .saturating_add(1);
        let candidate = plan.materialize(tail)?;
        if Self::node_cost(&candidate) >= old_cost {
            return Ok(None);
        }
        let original_completion = SemanticCompletion::sequence([
            SemanticCompletion::analyze(original),
            SemanticCompletion::branch(guard, SemanticCompletion::analyze(tail), None),
        ]);
        if original_completion != SemanticCompletion::analyze(&candidate) {
            return Ok(None);
        }
        Ok(Some(candidate))
    }

    fn linear_tail_size(node: &SemanticNode) -> Option<usize> {
        let mut statements = 0usize;
        let mut pending = vec![node];
        while let Some(node) = pending.pop() {
            match node {
                SemanticNode::Empty => {}
                SemanticNode::BasicBlock(block) => {
                    statements = statements.saturating_add(block.statements.len());
                    if statements > Self::MAX_TAIL_STATEMENTS {
                        return None;
                    }
                }
                SemanticNode::Sequence(children) => pending.extend(children),
                _ => return None,
            }
        }
        Some(statements)
    }

    fn predicate_cost(predicate: &SemanticPredicate) -> usize {
        let (atom, negated) = predicate.without_negations();
        match atom {
            SemanticPredicate::Test(_) => return 1 + usize::from(negated),
            SemanticPredicate::True | SemanticPredicate::False => return 1,
            _ => {}
        }
        predicate
            .domain()
            .map(|value| value.node_count())
            .unwrap_or(usize::MAX / 4)
    }

    fn node_cost(node: &SemanticNode) -> usize {
        let mut cost = 0usize;
        let mut pending = vec![node];
        while let Some(node) = pending.pop() {
            cost = cost.saturating_add(1);
            match node {
                SemanticNode::Empty | SemanticNode::Leave(_) => {}
                SemanticNode::BasicBlock(block) => {
                    cost = cost.saturating_add(block.statements.len().saturating_mul(2));
                }
                SemanticNode::Sequence(children) => pending.extend(children),
                SemanticNode::If {
                    condition,
                    then_node,
                    else_node,
                } => {
                    cost = cost.saturating_add(Self::predicate_cost(condition));
                    pending.push(then_node);
                    pending.extend(else_node.as_deref());
                }
                _ => cost = cost.saturating_add(32),
            }
        }
        cost
    }
}

struct PredicateDomain {
    bdd: BddContext,
    symbols: BTreeMap<InstructionId, BoolVariable>,
}

impl PredicateDomain {
    fn collect<'a>(
        root: &'a SemanticNode,
        guard: &'a SemanticPredicate,
    ) -> Result<Self, SemanticFoldError> {
        let mut atoms = AtomClasses::new(AtomIdentity::Distribution);
        let mut symbols = BTreeMap::new();
        Self::collect_predicate(guard, &mut atoms, &mut symbols)?;
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            match node {
                SemanticNode::Sequence(children) => pending.extend(children),
                SemanticNode::If {
                    condition,
                    then_node,
                    else_node,
                } => {
                    Self::collect_predicate(condition, &mut atoms, &mut symbols)?;
                    pending.push(then_node);
                    pending.extend(else_node.as_deref());
                }
                _ => {}
            }
        }
        let bdd = BddContext::new(&symbols.values().cloned().collect::<BTreeSet<_>>());
        Ok(Self { bdd, symbols })
    }

    fn collect_predicate<'a>(
        predicate: &'a SemanticPredicate,
        atoms: &mut AtomClasses<'a>,
        symbols: &mut BTreeMap<InstructionId, BoolVariable>,
    ) -> Result<(), SemanticFoldError> {
        let mut pending = vec![predicate];
        while let Some(predicate) = pending.pop() {
            match predicate {
                SemanticPredicate::Test(instruction) => {
                    let index = atoms.intern(instruction);
                    let index =
                        u32::try_from(index).map_err(|_| SemanticFoldError::MalformedWorkStack)?;
                    symbols.insert(instruction.id, BoolVariable::Atom(index));
                }
                SemanticPredicate::Not(inner) => pending.push(inner),
                SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                    pending.extend(terms);
                }
                SemanticPredicate::True | SemanticPredicate::False => {}
            }
        }
        Ok(())
    }

    fn compile(&self, predicate: &SemanticPredicate) -> Result<Bdd, BddError> {
        let expression = predicate
            .domain_with(|id| self.symbols.get(&id).cloned())
            .map_err(|_| BddError::MalformedEvaluation)?;
        self.bdd.compile(&expression)
    }
}

struct PathInjector<'a> {
    domain: &'a PredicateDomain,
    guard: Bdd,
}

impl<'a> PathInjector<'a> {
    fn new(domain: &'a PredicateDomain, guard: Bdd) -> Self {
        Self { domain, guard }
    }

    fn plan<'node>(&self, root: &'node SemanticNode) -> Result<InjectionPlan<'node>, BddError> {
        let mut tasks = vec![InjectionTask::Visit {
            node: root,
            path: self.domain.bdd.truth(),
        }];
        let mut regions = Vec::new();
        let mut steps = Vec::new();
        let mut copies = 0usize;
        while let Some(task) = tasks.pop() {
            match task {
                InjectionTask::Visit { node, path } => match node {
                    SemanticNode::Sequence(children) if !children.is_empty() => {
                        let (last, prefix) =
                            children.split_last().ok_or(BddError::MalformedEvaluation)?;
                        tasks.push(InjectionTask::Sequence(prefix));
                        tasks.push(InjectionTask::Visit { node: last, path });
                    }
                    SemanticNode::If {
                        condition,
                        then_node,
                        else_node,
                    } => {
                        let condition_domain = self.domain.compile(condition)?;
                        let true_path = self.domain.bdd.and(path, condition_domain)?;
                        let false_path = self
                            .domain
                            .bdd
                            .and(path, self.domain.bdd.not(condition_domain)?)?;
                        tasks.push(InjectionTask::If(condition));
                        tasks.push(InjectionTask::Visit {
                            node: else_node.as_deref().unwrap_or(&SemanticNode::Empty),
                            path: false_path,
                        });
                        tasks.push(InjectionTask::Visit {
                            node: then_node,
                            path: true_path,
                        });
                    }
                    node => {
                        let append = ControlCompletion::normally_completes(node)
                            && self.domain.bdd.implies_bdd(path, self.guard)?;
                        regions.push(if append {
                            path
                        } else {
                            self.domain.bdd.falsity()
                        });
                        copies = copies.saturating_add(usize::from(append));
                        steps.push(InjectionStep::Leaf { node, append });
                    }
                },
                InjectionTask::Sequence(prefix) => steps.push(InjectionStep::Sequence(prefix)),
                InjectionTask::If(condition) => {
                    let when_false = regions.pop().ok_or(BddError::MalformedEvaluation)?;
                    let when_true = regions.pop().ok_or(BddError::MalformedEvaluation)?;
                    regions.push(self.domain.bdd.or(when_true, when_false)?);
                    steps.push(InjectionStep::If(condition));
                }
            }
        }
        let covered = regions.pop().ok_or(BddError::MalformedEvaluation)?;
        if !regions.is_empty() {
            return Err(BddError::MalformedEvaluation);
        }
        Ok(InjectionPlan {
            steps,
            covered,
            copies,
        })
    }
}

// The proof visits the original tree without duplicating any subtree. Its
// postfix steps retain only the pieces needed by the ordinary IR constructors.
struct InjectionPlan<'a> {
    steps: Vec<InjectionStep<'a>>,
    covered: Bdd,
    copies: usize,
}

impl InjectionPlan<'_> {
    fn materialize(self, tail: &SemanticNode) -> Result<SemanticNode, BddError> {
        let mut results = Vec::new();
        for step in self.steps {
            let node = match step {
                InjectionStep::Leaf { node, append } => {
                    if append {
                        SemanticNode::sequence([node.clone(), tail.clone()])
                    } else {
                        node.clone()
                    }
                }
                InjectionStep::Sequence(prefix) => {
                    let last = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    SemanticNode::sequence(prefix.iter().cloned().chain(std::iter::once(last)))
                }
                InjectionStep::If(condition) => {
                    let when_false = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    let when_true = results.pop().ok_or(BddError::MalformedEvaluation)?;
                    SemanticNode::branch(
                        condition.clone(),
                        when_true,
                        (!matches!(&when_false, SemanticNode::Empty)).then_some(when_false),
                    )
                }
            };
            results.push(node);
        }
        let result = results.pop().ok_or(BddError::MalformedEvaluation)?;
        if !results.is_empty() {
            return Err(BddError::MalformedEvaluation);
        }
        Ok(result)
    }
}

enum InjectionStep<'a> {
    Leaf {
        node: &'a SemanticNode,
        append: bool,
    },
    Sequence(&'a [SemanticNode]),
    If(&'a SemanticPredicate),
}

enum InjectionTask<'a> {
    Visit { node: &'a SemanticNode, path: Bdd },
    Sequence(&'a [SemanticNode]),
    If(&'a SemanticPredicate),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        BlockId, InsnNode, InsnType, InstructionId, RegionId, SemanticBlock, SemanticLeave,
        SemanticLeaveKind, SemanticOperand, SemanticOperation, SemanticStatement,
    };

    fn predicate(id: usize) -> SemanticPredicate {
        let mut instruction = InsnNode::new(InsnType::If, 0);
        instruction.id = InstructionId::new(id);
        SemanticPredicate::Test(SemanticOperation::from_instruction(instruction).unwrap())
    }

    fn return_node() -> SemanticNode {
        let region = RegionId::new(0);
        SemanticNode::Leave(SemanticLeave {
            site: None,
            condition: None,
            kind: SemanticLeaveKind::Return(None),
            edge: None,
            origin: None,
            source: region,
            destination: region,
            target: region,
            cleanup: Vec::new(),
        })
    }

    fn no_return_block() -> SemanticNode {
        let mut instruction = InsnNode::new(InsnType::Invoke, 0);
        instruction.id = InstructionId::new(3);
        instruction.payload.no_return = true;
        SemanticNode::BasicBlock(SemanticBlock {
            id: BlockId::new(3),
            statements: vec![SemanticStatement::instruction(instruction).unwrap()],
        })
    }

    #[test]
    fn rejects_distribution_that_changes_structural_completion() {
        let first = predicate(1);
        let second = predicate(2);
        let prefix = SemanticNode::guard(
            first.clone().negate(),
            SemanticNode::guard(second.clone(), return_node()),
        );
        let guard = SemanticPredicate::Or(vec![first, second.negate()]);
        let tail = no_return_block();

        let domain = PredicateDomain::collect(&prefix, &guard).unwrap();
        let guard_domain = domain.compile(&guard).unwrap();
        let injected = PathInjector::new(&domain, guard_domain)
            .plan(&prefix)
            .unwrap()
            .materialize(&tail)
            .unwrap();
        let original_region = SemanticNode::sequence([
            prefix.clone(),
            SemanticNode::guard(guard.clone(), tail.clone()),
        ]);

        assert!(ControlCompletion::normally_completes(&original_region));
        assert!(!ControlCompletion::normally_completes(&injected));
        assert!(
            GuardDistribution::node_cost(&injected) < {
                GuardDistribution::node_cost(&prefix)
                    + GuardDistribution::predicate_cost(&guard)
                    + GuardDistribution::node_cost(&tail)
                    + 1
            }
        );

        let result =
            GuardDistribution::apply(&prefix, &SemanticOperand::new(guard), &tail).unwrap();

        assert!(result.is_none());
        assert!(ControlCompletion::normally_completes(&prefix));
    }
}
