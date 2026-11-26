//! Sparse reaching-definition facts over occurrence-sensitive Semantic IR.
//!
//! Candidate selection is derived exclusively from the semantic control graph.
//! Effect-aware movement and source rewriting remain planner responsibilities.

use std::{cell::OnceCell, sync::Arc};

use super::{DefinitionFact, UseFact, UseSite, ValueFlowGraph, ValueIdentity};
use crate::analysis::value_recovery::ValueRecoveryError;
use crate::ir::analysis::{
    SemanticFlowGraph, SemanticFlowPoint, SemanticReachingValues, SemanticValueDefinition, SsaVar,
};

pub(super) struct SemanticReachingAnalysis<'a> {
    graph: &'a ValueFlowGraph<'a>,
    model: OnceCell<ReachingModel<'a>>,
}

enum ReachingModel<'a> {
    Ssa(&'a SemanticFlowGraph),
    Source {
        flow: &'a SemanticFlowGraph,
        reaching: OnceCell<SourceReaching>,
    },
    Incomplete,
}

struct SourceReaching {
    facts: Arc<SemanticReachingValues>,
}

impl SourceReaching {
    fn analyze(graph: &ValueFlowGraph<'_>, flow: &SemanticFlowGraph) -> Self {
        let definitions = graph
            .definitions
            .values()
            .enumerate()
            .flat_map(|(value, definitions)| {
                definitions
                    .iter()
                    .enumerate()
                    .filter_map(move |(ordinal, definition)| {
                        definition.site.map(|site| {
                            SemanticValueDefinition::new(
                                SemanticFlowPoint::after(site),
                                value,
                                ordinal,
                            )
                        })
                    })
            })
            .collect::<Vec<_>>();
        let targets = graph.movement_points();
        let facts =
            flow.cached_reaching_values(definitions, graph.definitions.values().len(), targets);
        Self { facts }
    }
}

impl<'a> SemanticReachingAnalysis<'a> {
    pub(super) fn new(graph: &'a ValueFlowGraph<'a>) -> Self {
        Self {
            graph,
            model: OnceCell::new(),
        }
    }

    fn model(&self) -> &ReachingModel<'a> {
        self.model.get_or_init(|| {
            let Some(flow) = self.graph.semantic_flow().filter(|flow| flow.is_complete()) else {
                return ReachingModel::Incomplete;
            };
            match self.graph.identity() {
                ValueIdentity::Ssa => ReachingModel::Ssa(flow),
                ValueIdentity::Source => ReachingModel::Source {
                    flow,
                    reaching: OnceCell::new(),
                },
            }
        })
    }

    pub(super) fn candidates(
        &self,
        key: SsaVar,
        point: SemanticFlowPoint,
    ) -> Result<Vec<usize>, ValueRecoveryError> {
        let mut output = Vec::new();
        self.candidates_into(key, point, &mut output)?;
        Ok(output)
    }

    fn candidates_into(
        &self,
        key: SsaVar,
        point: SemanticFlowPoint,
        output: &mut Vec<usize>,
    ) -> Result<(), ValueRecoveryError> {
        let definitions = self
            .graph
            .definitions
            .get(&key)
            .filter(|_| !self.graph.definitions_without_sites.contains(&key))
            .ok_or(ValueRecoveryError::MissingDefinitionSite(key))?;
        match self.model() {
            ReachingModel::Ssa(flow) => {
                let reachability = flow.indexed_reachability();
                output.clear();
                output.extend(definitions.iter().enumerate().filter_map(
                    |(ordinal, definition)| {
                        definition
                            .site
                            .map(SemanticFlowPoint::after)
                            .filter(|definition| reachability.reaches(*definition, point))
                            .map(|_| ordinal)
                    },
                ));
                Ok(())
            }
            ReachingModel::Source { flow, reaching } => {
                let reaching = reaching.get_or_init(|| SourceReaching::analyze(self.graph, flow));
                let value = self
                    .graph
                    .definitions
                    .row(&key)
                    .ok_or(ValueRecoveryError::MissingSemanticSite)?;
                let candidates = reaching.facts.at_into(value, point, output);
                candidates.ok_or(ValueRecoveryError::MissingReachingFact { value: key, point })
            }
            ReachingModel::Incomplete => Err(ValueRecoveryError::IncompleteSemanticFlow),
        }
    }

    pub(super) fn unchanged(
        &self,
        key: SsaVar,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
    ) -> bool {
        if !self.graph.definitions.contains_key(&key) {
            return true;
        }
        if self.graph.definitions_without_sites.contains(&key) {
            return false;
        }
        match self.model() {
            ReachingModel::Ssa(_) => true,
            ReachingModel::Source { flow, reaching } => {
                let reaching = reaching.get_or_init(|| SourceReaching::analyze(self.graph, flow));
                self.graph
                    .definitions
                    .row(&key)
                    .and_then(|value| reaching.facts.unchanged(value, source, target))
                    .unwrap_or(false)
            }
            ReachingModel::Incomplete => false,
        }
    }

    pub(super) fn supports(&self, source: SemanticFlowPoint, target: SemanticFlowPoint) -> bool {
        source != target && !matches!(self.model(), ReachingModel::Incomplete)
    }

    /// On a complete SSA graph, every input version is invariant once all
    /// definitions have sites. Source locals can have multiple versions.
    pub(super) fn operands_unchanged<I: IntoIterator<Item = SsaVar>>(
        &self,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
        values: impl FnOnce() -> I,
    ) -> bool {
        if !self.supports(source, target) {
            return false;
        }
        if matches!(self.model(), ReachingModel::Ssa(_))
            && self.graph.definitions_without_sites.is_empty()
        {
            return true;
        }
        values()
            .into_iter()
            .all(|value| self.unchanged(value, source, target))
    }
}

pub(super) struct ReachingDefinitionFacts {
    pub(super) uses: Vec<ReachingUse>,
    candidates: Vec<usize>,
    /// Number of uses for which each definition remains a feasible reaching
    /// value, including uses whose path relation cannot select one definition.
    pub(super) candidate_uses: Vec<usize>,
}

impl ReachingDefinitionFacts {
    pub(super) fn candidates(&self, usage: &ReachingUse) -> &[usize] {
        &self.candidates[usage.candidates.clone()]
    }
}

#[derive(Clone)]
pub(super) struct ReachingUse {
    candidates: std::ops::Range<usize>,
    pub(super) selected: Option<usize>,
    pub(super) control: bool,
}

pub(super) struct ReachingDefinitions<'a, 'graph> {
    graph: &'graph ValueFlowGraph<'graph>,
    analysis: &'a SemanticReachingAnalysis<'graph>,
}

impl<'a, 'graph> ReachingDefinitions<'a, 'graph> {
    pub(super) fn new(
        graph: &'graph ValueFlowGraph<'graph>,
        analysis: &'a SemanticReachingAnalysis<'graph>,
    ) -> Self {
        Self { graph, analysis }
    }

    pub(super) fn analyze(
        &self,
        definitions: &[DefinitionFact],
        uses: &[UseFact],
    ) -> Result<ReachingDefinitionFacts, ValueRecoveryError> {
        let key = definitions
            .first()
            .map(|definition| definition.key)
            .ok_or(ValueRecoveryError::MissingSemanticSite)?;
        let mut candidate_uses = vec![0usize; definitions.len()];
        let mut relations: Vec<ReachingUse> = Vec::with_capacity(uses.len());
        let mut candidates = Vec::new();
        let mut indices = Vec::new();
        let mut next_indices = Vec::new();
        let mut previous_point = None;
        let mut previous_query = None;

        for usage in uses {
            let Some(point) = usage.location.point() else {
                return Err(ValueRecoveryError::MissingUsePoint(key));
            };
            let same_indices = if previous_point == Some(point) {
                true
            } else {
                self.analysis
                    .candidates_into(key, point, &mut next_indices)?;
                let same = previous_point.is_some() && next_indices == indices;
                std::mem::swap(&mut indices, &mut next_indices);
                previous_point = Some(point);
                same
            };
            let control = matches!(
                usage.location.site(),
                Some(
                    UseSite::Predicate(_) | UseSite::SelectedPredicate(_, _) | UseSite::Argument(_)
                )
            );
            let saturated = self.graph.proof_budget_exhausted();
            let query = (usage.domain, control, saturated);
            let relation = if same_indices && previous_query == Some(query) {
                // The original successful proofs are cached by the same BDD arena.
                // A relation which crossed its proof limit is never replayed.
                relations.last().expect("previous reaching query").clone()
            } else {
                let start = candidates.len();
                for &index in &indices {
                    // A prior iteration can reach the use even when the two
                    // control domains within a single iteration are disjoint.
                    if definitions[index].repetitive
                        || !self
                            .graph
                            .logic
                            .disjoint(usage.domain, definitions[index].domain)?
                    {
                        candidates.push(index);
                    }
                }
                let feasible = &candidates[start..];
                let mut selected = None;
                if control {
                    // Every other feasible definition already defeated exclusion:
                    // it is repetitive or its intersection was nonempty. Saturation
                    // can only make the latter disjointness proof more conservative.
                    // Implications can still consume the arena budget in this order.
                    for &index in feasible {
                        if self
                            .graph
                            .logic
                            .implies(usage.domain, definitions[index].domain)?
                            && feasible.len() == 1
                        {
                            selected = Some(index);
                        }
                    }
                } else if let [only] = feasible {
                    selected = Some(*only);
                }
                previous_query =
                    (saturated == self.graph.proof_budget_exhausted()).then_some(query);
                ReachingUse {
                    candidates: start..candidates.len(),
                    selected,
                    control,
                }
            };
            relations.push(relation);
        }
        for run in relations.chunk_by(|left, right| left.candidates == right.candidates) {
            for &index in &candidates[run[0].candidates.clone()] {
                candidate_uses[index] += run.len();
            }
        }

        Ok(ReachingDefinitionFacts {
            uses: relations,
            candidates,
            candidate_uses,
        })
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/reaching_partitions.rs"]
mod partitions;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        ArgType, BlockId, InsnArg, InsnNode, InsnType, InstructionId, LiteralArg, RegionId,
        RegisterArg, SemanticBlock, SemanticExpression, SemanticLeave, SemanticLeaveKind,
        SemanticLoopControl, SemanticNode, SemanticOperand, SemanticPredicate,
        SemanticSiteNumbering, SemanticStatement,
    };
    use std::collections::BTreeSet;

    fn predicate(id: usize) -> SemanticPredicate {
        let mut instruction = InsnNode::new(InsnType::If, 0);
        instruction.id = InstructionId::new(id);
        SemanticPredicate::Test(
            crate::ir::SemanticOperation::from_instruction(instruction).unwrap(),
        )
    }

    fn value_predicate(id: usize, register: u32, variable: u32) -> SemanticPredicate {
        let mut instruction = InsnNode::new(InsnType::If, 0);
        instruction.id = InstructionId::new(id);
        SemanticPredicate::Test(crate::ir::SemanticOperation::from_parts(
            instruction,
            vec![SemanticExpression::Register(local(register, variable))],
            None,
        ))
    }

    fn local(register: u32, variable: u32) -> RegisterArg {
        let mut result = RegisterArg::new(register, ArgType::INT);
        result.code_var = Some(variable);
        result
    }

    fn move_statement(
        register: u32,
        variable: u32,
        value: i64,
        edge_copy: bool,
    ) -> SemanticStatement {
        let mut instruction = InsnNode::mov(
            local(register, variable),
            InsnArg::Lit(LiteralArg::new(value, ArgType::INT)),
        );
        instruction.payload.edge_copy = edge_copy;
        SemanticStatement::instruction(instruction).unwrap()
    }

    fn nop_statement() -> SemanticStatement {
        SemanticStatement::instruction(InsnNode::new(InsnType::Nop, 0)).unwrap()
    }

    #[test]
    fn repetitive_definition_reaches_a_disjoint_loop_exit_domain() {
        let region = RegionId::new(1);
        let variable = 2;
        let mut root = SemanticNode::sequence([
            SemanticNode::BasicBlock(SemanticBlock {
                id: BlockId::new(0),
                statements: vec![move_statement(0, variable, 0, true)],
            }),
            SemanticNode::For {
                control: SemanticLoopControl::Region(region),
                init: Box::new(nop_statement()),
                condition: SemanticOperand::new(predicate(1)),
                update: Box::new(nop_statement()),
                body: Box::new(SemanticNode::guard(
                    predicate(2),
                    SemanticNode::BasicBlock(SemanticBlock {
                        id: BlockId::new(1),
                        statements: vec![move_statement(0, variable, 7, true)],
                    }),
                )),
            },
            SemanticNode::guard(
                value_predicate(3, 0, variable),
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
                }),
            ),
        ]);
        SemanticSiteNumbering::assign(&mut root).unwrap();
        let graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();
        let key = SsaVar::new(variable, 0);
        let definitions = &graph.definitions[&key];
        let uses = &graph.uses[&key];
        let analysis = SemanticReachingAnalysis::new(&graph);

        let facts = ReachingDefinitions::new(&graph, &analysis)
            .analyze(definitions, uses)
            .unwrap();
        let repetitive = definitions
            .iter()
            .position(|definition| definition.repetitive)
            .expect("loop definition");

        assert_eq!(uses.len(), 1);
        assert!(facts.uses[0].control);
        assert!(facts.candidates(&facts.uses[0]).contains(&repetitive));
        assert_eq!(facts.uses[0].selected, None);
        assert_eq!(facts.candidate_uses[repetitive], 1);
    }
}
