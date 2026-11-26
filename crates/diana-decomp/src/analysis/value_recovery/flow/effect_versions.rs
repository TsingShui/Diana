//! Path-sensitive source-observable effect stability.

use std::cell::{Cell, OnceCell};

use crate::ir::analysis::{SemanticFlowPoint, SemanticReachability};

use super::{effects::EffectSummary, ControlDomain, EffectFact, ValueFlowGraph};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EffectStability {
    Stable,
    IncompleteFlow,
    DoesNotReach,
    Changed,
}

pub(super) struct EffectVersions<'a> {
    graph: &'a ValueFlowGraph<'a>,
    prior_query: Cell<bool>,
    uncharged_facts: OnceCell<bool>,
    order: OnceCell<EffectOrder>,
}

/// Keep occurrence identities for all points in a cycle. The interval is
/// only a candidate filter; exact reachability still decides membership.
struct EffectOrder(Vec<(usize, usize)>);

impl EffectOrder {
    fn new(reachability: &SemanticReachability, effects: &[EffectFact]) -> Self {
        let mut ranks = effects
            .iter()
            .enumerate()
            .filter_map(|(i, fact)| Some((reachability.topological_component(fact.point)?, i)))
            .collect::<Vec<_>>();
        ranks.sort_unstable();
        Self(ranks)
    }

    fn between(
        &self,
        reachability: &SemanticReachability,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
    ) -> &[(usize, usize)] {
        let (Some(first), Some(last)) = (
            reachability.topological_component(source),
            reachability.topological_component(target),
        ) else {
            return &[];
        };
        let end = self.0.partition_point(|&(rank, _)| rank <= last);
        let start = self.0[..end].partition_point(|&(rank, _)| rank < first);
        &self.0[start..end]
    }
}

impl<'a> EffectVersions<'a> {
    pub(super) fn new(graph: &'a ValueFlowGraph<'a>) -> Self {
        Self {
            graph,
            prior_query: Cell::new(false),
            uncharged_facts: OnceCell::new(),
            order: OnceCell::new(),
        }
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
        let conflicts = |fact: &EffectFact| {
            fact.point != source
                && fact.point != target
                && effect.conflicts_with(&fact.effect)
                && !self
                    .graph
                    .logic
                    .disjoint(domain, fact.domain)
                    .unwrap_or(false)
        };
        // A bounded scan, or a single query, costs no persistent index.
        // Slicing/reordering domain checks is exact only if none can charge
        // the BDD arena. Otherwise every original check executes in order.
        let effects = &self.graph.effects;
        let changed = if effects.len() > 8
            && self.prior_query.replace(true)
            && (self.graph.logic.disjoint_without_bdd_work(domain)
                || *self.uncharged_facts.get_or_init(|| {
                    effects
                        .iter()
                        .all(|fact| self.graph.logic.disjoint_without_bdd_work(fact.domain))
                })) {
            let reachability = flow.indexed_reachability();
            let order = self
                .order
                .get_or_init(|| EffectOrder::new(reachability, effects));
            let candidates = order
                .between(reachability, source, target)
                .iter()
                .filter_map(|&(_, i)| conflicts(&effects[i]).then_some(&effects[i].point));
            reachability.reaches_any_between(source, target, candidates)
        } else {
            let mut conflicting = effects
                .iter()
                .filter_map(|fact| conflicts(fact).then_some(&fact.point));
            let Some(first) = conflicting.next() else {
                return EffectStability::Stable;
            };
            let changed = flow.indexed_reachability().reaches_any_between(
                source,
                target,
                std::iter::once(first).chain(&mut conflicting),
            );
            // Reachability may stop at the first hit. The remaining domain
            // intersections still run and may exhaust the shared proof budget.
            conflicting.for_each(|_| {});
            changed
        };
        if changed {
            EffectStability::Changed
        } else {
            EffectStability::Stable
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/effect_intervals.rs"]
mod tests;
