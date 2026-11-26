//! Path-sensitive source-observable effect stability.

use std::cell::{Cell, OnceCell, RefCell};

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
    domain_runs: OnceCell<Option<EffectDomainRuns>>,
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

/// Consecutive effects in one control domain have the same intersection with
/// a movement's domain. The first conflicting occurrence charges the shared
/// proof arena. Every later one in that run reuses that exact intersection.
struct EffectDomainRuns {
    starts: Vec<usize>,
    by_effect: Vec<usize>,
    eligible: Vec<Cell<bool>>,
    previous: Cell<Option<(ControlDomain, bool)>>,
    charged: RefCell<smallvec::SmallVec<[u64; 1]>>,
}

impl EffectDomainRuns {
    fn grouped(effects: &[EffectFact]) -> Option<Self> {
        let starts = effects
            .iter()
            .enumerate()
            .filter_map(|(i, fact)| (i == 0 || fact.domain != effects[i - 1].domain).then_some(i))
            .collect::<Vec<_>>();
        // Two scans (run discovery and the topological interval) are useful
        // only when enough occurrences share their domain.
        if starts.is_empty() || starts.len() > effects.len() / 2 {
            return None;
        }
        let mut by_effect = Vec::with_capacity(effects.len());
        for (index, &end) in starts.iter().skip(1).enumerate() {
            by_effect.resize(end, index);
        }
        by_effect.resize(effects.len(), starts.len() - 1);
        let eligible = vec![Cell::new(false); starts.len()];
        Some(Self {
            starts,
            by_effect,
            eligible,
            previous: Cell::new(None),
            charged: RefCell::default(),
        })
    }

    // The most recent complete row already holds each eligibility bit.
    // Earlier complete rows need only one bit per arena domain: all their
    // intersections remain memoized for an interval query until saturation.
    fn charged_earlier(&self, movement: ControlDomain) -> bool {
        if self
            .previous
            .get()
            .is_some_and(|(domain, _)| domain == movement)
        {
            return false;
        }
        let index = movement.index();
        self.charged
            .borrow()
            .get(index / 64)
            .is_some_and(|word| word & (1 << (index % 64)) != 0)
    }

    fn prepare(
        &self,
        effects: &[EffectFact],
        movement: ControlDomain,
        conflicts: impl Fn(&EffectFact) -> bool,
        mut overlaps: impl FnMut(ControlDomain) -> bool,
        saturated: impl Fn() -> bool,
    ) -> bool {
        // Once every run charged its intersection, repeating the same domain
        // uses only memoized proofs. The interval still tests the new endpoints
        // and effects. Saturation can make earlier disjoint runs overlap.
        if let Some((domain, any_overlap)) = self.previous.get() {
            if domain == movement && !saturated() {
                return any_overlap;
            }
        }
        let mut all_checked = true;
        let mut any_overlap = false;
        for (i, &start) in self.starts.iter().enumerate() {
            let end = self.starts.get(i + 1).copied().unwrap_or(effects.len());
            let eligible = if effects[start..end].iter().any(&conflicts) {
                overlaps(effects[start].domain)
            } else {
                all_checked = false;
                false
            };
            self.eligible[i].set(eligible);
            any_overlap |= eligible;
        }
        if all_checked && !saturated() {
            self.previous.set(Some((movement, any_overlap)));
            let index = movement.index();
            let mut charged = self.charged.borrow_mut();
            if charged.len() <= index / 64 {
                charged.resize(index / 64 + 1, 0);
            }
            charged[index / 64] |= 1 << (index % 64);
        } else {
            self.previous.set(None);
        }
        any_overlap
    }

    fn eligible(&self, index: usize) -> bool {
        self.eligible[self.by_effect[index]].get()
    }
}

impl<'a> EffectVersions<'a> {
    pub(super) fn new(graph: &'a ValueFlowGraph<'a>) -> Self {
        Self {
            graph,
            prior_query: Cell::new(false),
            uncharged_facts: OnceCell::new(),
            order: OnceCell::new(),
            domain_runs: OnceCell::new(),
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
            fact.point != source && fact.point != target && effect.conflicts_with(&fact.effect)
        };
        let overlaps = |other| !self.graph.logic.disjoint(domain, other).unwrap_or(false);
        let effects = &self.graph.effects;
        let repeated = effects.len() > 8 && self.prior_query.replace(true);
        let uncharged = repeated
            && (self.graph.logic.disjoint_without_bdd_work(domain)
                || *self.uncharged_facts.get_or_init(|| {
                    effects
                        .iter()
                        .all(|fact| self.graph.logic.disjoint_without_bdd_work(fact.domain))
                }));
        let runs = if repeated && !uncharged {
            self.domain_runs
                .get_or_init(|| EffectDomainRuns::grouped(effects))
                .as_ref()
        } else {
            None
        };
        // An earlier complete row made every possible intersection for
        // this domain uncharged, including ones outside the current interval.
        let changed = if uncharged || runs.is_some_and(|runs| runs.charged_earlier(domain)) {
            self.interval_conflict(flow, source, target, |i| {
                conflicts(&effects[i]) && overlaps(effects[i].domain)
            })
        } else if let Some(runs) = runs {
            // Keep the full sequence of first domain checks, even after a
            // reachable conflict. Saturation can change a later run's answer.
            runs.prepare(effects, domain, conflicts, overlaps, || {
                self.graph.proof_budget_exhausted()
            }) && self.interval_conflict(flow, source, target, |i| {
                runs.eligible(i) && conflicts(&effects[i])
            })
        } else {
            let mut conflicting = effects.iter().filter_map(|fact| {
                (conflicts(fact) && overlaps(fact.domain)).then_some(&fact.point)
            });
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

    fn interval_conflict(
        &self,
        flow: &crate::ir::analysis::SemanticFlowGraph,
        source: SemanticFlowPoint,
        target: SemanticFlowPoint,
        candidate: impl Fn(usize) -> bool,
    ) -> bool {
        let effects = &self.graph.effects;
        let reachability = flow.indexed_reachability();
        let order = self
            .order
            .get_or_init(|| EffectOrder::new(reachability, effects));
        let points = order
            .between(reachability, source, target)
            .iter()
            .filter_map(|&(_, i)| candidate(i).then_some(&effects[i].point));
        reachability.reaches_any_between(source, target, points)
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/effect_intervals.rs"]
mod tests;
