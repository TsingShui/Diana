//! Path-sensitive source-observable effect stability.

use crate::ir::analysis::SemanticFlowPoint;

use super::{effects::EffectSummary, ControlDomain, ValueFlowGraph};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EffectStability {
    Stable,
    IncompleteFlow,
    DoesNotReach,
    Changed,
}

pub(super) struct EffectVersions<'a> {
    graph: &'a ValueFlowGraph<'a>,
}

impl<'a> EffectVersions<'a> {
    pub(super) fn new(graph: &'a ValueFlowGraph<'a>) -> Self {
        Self { graph }
    }

    pub(super) fn unchanged(
        &self,
        effect: &EffectSummary,
        domain: ControlDomain,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
    ) -> bool {
        self.stability(effect, domain, source, target) == EffectStability::Stable
    }

    pub(super) fn stability(
        &self,
        effect: &EffectSummary,
        domain: ControlDomain,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
    ) -> EffectStability {
        if effect.is_pure() {
            return EffectStability::Stable;
        }
        let Some(flow) = self.graph.semantic_flow().filter(|flow| flow.is_complete()) else {
            return EffectStability::IncompleteFlow;
        };
        if !flow.must_reach(source, target) {
            return EffectStability::DoesNotReach;
        }
        let conflicting = self
            .graph
            .effects
            .iter()
            .filter(|fact| {
                fact.point != source
                    && fact.point != target
                    && effect.conflicts_with(&fact.effect)
                    && !self
                        .graph
                        .logic
                        .disjoint(domain, fact.domain)
                        .unwrap_or(false)
            })
            .map(|fact| fact.point)
            .collect::<Vec<_>>();
        if !conflicting.is_empty()
            && flow
                .indexed_reachability()
                .reaches_any_between(source, target, &conflicting)
        {
            EffectStability::Changed
        } else {
            EffectStability::Stable
        }
    }
}
