//! Unified sparse facts consumed by value scheduling.
//!
//! Canonical values, value numbering, reaching definitions, and effect
//! versions are computed from one immutable value-flow graph. Scheduling owns
//! no independent analysis state.

use std::collections::{BTreeMap, BTreeSet};
use std::{cell::RefCell, rc::Rc};

use crate::analysis::value_recovery::{
    numbering::{SyntheticUse, ValueAvailability, ValueNumbering},
    ValueRecoveryError,
};
use crate::ir::{analysis::SsaVar, InsnArg};

use super::{
    effect_versions::{EffectStability, EffectVersions},
    effects::EffectSummary,
    reaching::{ReachingDefinitionFacts, ReachingDefinitions, SemanticReachingAnalysis},
    DefinitionFact, UseFact, ValueFlowGraph, ValueIdentity,
};

struct CachedRelations {
    key: SsaVar,
    saturated: bool,
    facts: Rc<ReachingDefinitionFacts>,
}

pub(super) struct SparseValueFacts<'a> {
    graph: &'a ValueFlowGraph<'a>,
    canonical: BTreeMap<SsaVar, InsnArg>,
    numbered_replacements: BTreeMap<SsaVar, InsnArg>,
    uses_with_aliases: BTreeMap<SsaVar, Vec<UseFact>>,
    required_phi_inputs: BTreeSet<SsaVar>,
    reaching: SemanticReachingAnalysis<'a>,
    last_relations: RefCell<Option<CachedRelations>>,
    effects: EffectVersions<'a>,
}

impl<'a> SparseValueFacts<'a> {
    pub(super) fn analyze(graph: &'a ValueFlowGraph<'a>) -> Result<Self, ValueRecoveryError> {
        let classes = graph.classes_by_last_definition();
        let canonical = graph.canonical_values(&classes);
        let reaching = SemanticReachingAnalysis::new(graph);
        let availability = (graph.identity() == ValueIdentity::Source)
            .then_some(&reaching as &dyn ValueAvailability);
        let numbering = ValueNumbering::analyze_ordered(
            graph.identity(),
            &graph.logic,
            &graph.scopes,
            classes
                .into_iter()
                .filter_map(|class| match class.definitions {
                    [definition] => Some(definition),
                    _ => None,
                }),
            &canonical,
            availability,
        )?;
        let uses_with_aliases = Self::uses_with_aliases(graph, &numbering.synthetic_uses);
        let required_phi_inputs = if graph.identity() == ValueIdentity::Ssa {
            graph.required_phi_inputs()
        } else {
            Default::default()
        };
        let effects = EffectVersions::new(graph);
        Ok(Self {
            graph,
            canonical,
            numbered_replacements: numbering.replacements,
            uses_with_aliases,
            required_phi_inputs,
            reaching,
            last_relations: RefCell::new(None),
            effects,
        })
    }

    /// Use order is the representative's own occurrences followed by the
    /// original occurrences of each eliminated value, in numbering order.
    fn uses_with_aliases(
        graph: &'a ValueFlowGraph<'a>,
        links: &[SyntheticUse],
    ) -> BTreeMap<SsaVar, Vec<UseFact>> {
        let mut uses = BTreeMap::new();
        for link in links {
            if let Some(eliminated) = graph.uses.get(&link.eliminated) {
                uses.entry(link.representative)
                    .or_insert_with(|| {
                        graph
                            .uses
                            .get(&link.representative)
                            .cloned()
                            .unwrap_or_default()
                    })
                    .extend(eliminated.iter().cloned());
            }
        }
        uses
    }

    pub(super) fn graph(&self) -> &'a ValueFlowGraph<'a> {
        self.graph
    }

    pub(super) fn canonical(&self) -> &BTreeMap<SsaVar, InsnArg> {
        &self.canonical
    }

    pub(super) fn into_ssa_constants(mut self) -> BTreeMap<SsaVar, InsnArg> {
        if self.graph.identity() == ValueIdentity::Source {
            return BTreeMap::new();
        }
        self.canonical
            .retain(|_, value| ValueFlowGraph::is_constant(value));
        self.canonical
    }

    pub(super) fn replacement(&self, key: SsaVar) -> Option<&InsnArg> {
        self.canonical
            .get(&key)
            .or_else(|| self.numbered_replacements.get(&key))
    }

    pub(super) fn uses_of(&self, key: SsaVar) -> &[UseFact] {
        self.uses_with_aliases
            .get(&key)
            .or_else(|| self.graph.uses.get(&key))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(super) fn required_phi_input(&self, key: SsaVar) -> bool {
        self.required_phi_inputs.contains(&key)
    }

    pub(super) fn reaching_definitions(
        &self,
        definitions: &[DefinitionFact],
    ) -> Result<Rc<ReachingDefinitionFacts>, ValueRecoveryError> {
        let key = definitions
            .first()
            .ok_or(ValueRecoveryError::MissingSemanticSite)?
            .key;
        let saturated = self.graph.proof_budget_exhausted();
        if let Some(previous) = self.last_relations.borrow().as_ref() {
            if previous.key == key && previous.saturated == saturated {
                return Ok(Rc::clone(&previous.facts));
            }
        }
        let facts = Rc::new(
            ReachingDefinitions::new(self.graph, &self.reaching)
                .analyze(definitions, self.uses_of(key))?,
        );
        // Several rules inspect the same assignment class in succession. A
        // proof that crossed the BDD budget cannot describe a later query:
        // the earlier relations may include precise answers from before it.
        if self.graph.proof_budget_exhausted() == saturated {
            *self.last_relations.borrow_mut() = Some(CachedRelations {
                key,
                saturated,
                facts: Rc::clone(&facts),
            });
        }
        Ok(facts)
    }

    pub(super) fn reaching_candidates(
        &self,
        key: SsaVar,
        point: crate::ir::analysis::SemanticFlowPoint,
    ) -> Result<Vec<usize>, ValueRecoveryError> {
        self.reaching.candidates(key, point)
    }

    pub(super) fn reaching_unchanged(
        &self,
        key: SsaVar,
        source: crate::ir::analysis::SemanticFlowPoint,
        target: crate::ir::analysis::SemanticFlowPoint,
    ) -> bool {
        self.reaching.unchanged(key, source, target)
    }

    pub(super) fn reaching_supports(
        &self,
        source: crate::ir::analysis::SemanticFlowPoint,
        target: crate::ir::analysis::SemanticFlowPoint,
    ) -> bool {
        self.reaching.supports(source, target)
    }

    pub(super) fn effect_stability(
        &self,
        effect: &EffectSummary,
        domain: super::ControlDomain,
        source: crate::ir::analysis::SemanticFlowPoint,
        target: crate::ir::analysis::SemanticFlowPoint,
    ) -> EffectStability {
        self.effects.stability(effect, domain, source, target)
    }

    pub(super) fn effects_unchanged(
        &self,
        effect: &EffectSummary,
        domain: super::ControlDomain,
        source: crate::ir::analysis::SemanticFlowPoint,
        target: crate::ir::analysis::SemanticFlowPoint,
    ) -> bool {
        self.effects.unchanged(effect, domain, source, target)
    }
}

impl ValueAvailability for SemanticReachingAnalysis<'_> {
    fn supports(
        &self,
        source: crate::ir::analysis::SemanticFlowPoint,
        target: crate::ir::analysis::SemanticFlowPoint,
    ) -> bool {
        SemanticReachingAnalysis::supports(self, source, target)
    }

    fn unchanged(
        &self,
        value: SsaVar,
        source: crate::ir::analysis::SemanticFlowPoint,
        target: crate::ir::analysis::SemanticFlowPoint,
    ) -> bool {
        SemanticReachingAnalysis::unchanged(self, value, source, target)
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/value_fact_indices.rs"]
mod tests;
