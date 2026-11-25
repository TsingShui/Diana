//! Source-identity scheduling after source-variable allocation.
//!
//! Allocation removes phi obligations and assigns stable `code_var` identities.
//! The same control-domain and effect scheduler used for SSA can therefore run
//! again with source identities, without a second propagation or DCE algorithm.

use std::sync::Arc;

use crate::ir::{
    SemanticControlTopology, SemanticMethod, SemanticNode, SemanticSiteId, SemanticVisitor,
    SourceVariableContext,
};

#[cfg(debug_assertions)]
use crate::ir::SemanticFoldError;

use super::{
    flow::{RecoveryMode, ValueFlowGraph, ValueIdentity},
    initialization::SourceInitializationRecovery,
    loops::LoopInvariantMotion,
    predicate_regions::PredicateRegionFormation,
    schedule::ValueSchedule,
    ValueRecoveryError,
};

#[cfg(test)]
#[path = "../../../tests/internal/source_flow_cache.rs"]
mod tests;

#[derive(Debug, Clone)]
pub(super) struct SourceFlowCache {
    inputs: FlowInputs,
    flow: Arc<crate::ir::analysis::SemanticFlowGraph>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FlowInputs {
    topology: SemanticControlTopology,
    sites: Vec<(usize, Option<SemanticSiteId>, bool, Option<bool>)>,
    node: usize,
    protection_depth: usize,
}

impl FlowInputs {
    fn site(
        &mut self,
        site: Option<SemanticSiteId>,
        may_throw: impl FnOnce() -> bool,
        condition: Option<bool>,
    ) {
        self.sites.push((
            self.node,
            site,
            self.protection_depth > 0 && may_throw(),
            condition,
        ));
    }
}

impl SemanticVisitor for FlowInputs {
    fn visits_values(&self) -> bool {
        false
    }

    fn enter_node(&mut self, node: &SemanticNode) {
        self.node += 1;
        self.topology.enter_node(node);
        match node {
            SemanticNode::Try { .. } => self.protection_depth += 1,
            SemanticNode::BasicBlock(block) => {
                for statement in &block.statements {
                    self.site(statement.site, || statement.effects().may_throw(), None);
                }
            }
            SemanticNode::For {
                init,
                condition,
                update,
                ..
            } => {
                self.site(init.site, || init.effects().may_throw(), None);
                self.site(
                    condition.site,
                    || condition.effects().may_throw(),
                    condition.constant_value(),
                );
                self.site(update.site, || update.effects().may_throw(), None);
            }
            SemanticNode::Loop { test, .. } => self.site(
                test.condition.site,
                || test.condition.effects().may_throw(),
                test.condition.constant_value(),
            ),
            SemanticNode::If { condition, .. } => {
                self.site(condition.site, || condition.effects().may_throw(), None)
            }
            SemanticNode::ForEach { iterable, .. } => {
                self.site(iterable.site, || iterable.effects().may_throw(), None)
            }
            SemanticNode::Switch { selector, .. } => {
                self.site(selector.site, || selector.effects().may_throw(), None)
            }
            SemanticNode::Synchronized { lock, .. } => {
                self.site(lock.site, || lock.effects().may_throw(), None)
            }
            SemanticNode::Leave(leave) => self.site(
                leave.site,
                || {
                    leave
                        .value()
                        .is_some_and(|value| value.effects().may_throw())
                        || leave
                            .condition
                            .as_ref()
                            .is_some_and(|value| value.effects().may_throw())
                },
                None,
            ),
            SemanticNode::Empty | SemanticNode::Sequence(_) | SemanticNode::Label { .. } => {}
        }
    }

    fn exit_node(&mut self, node: &SemanticNode) {
        if matches!(node, SemanticNode::Try { .. }) {
            self.protection_depth -= 1;
        }
        self.topology.exit_node(node);
    }
}

impl SourceFlowCache {
    pub(super) fn get_or_analyze(
        cache: &mut Option<Self>,
        root: &SemanticNode,
    ) -> Arc<crate::ir::analysis::SemanticFlowGraph> {
        let mut inputs = FlowInputs::default();
        inputs.visit_node(root);
        if let Some(cached) = cache.as_ref() {
            if cached.inputs == inputs {
                return Arc::clone(&cached.flow);
            }
        }
        let flow = Arc::new(crate::ir::analysis::SemanticFlowGraph::analyze(root));
        *cache = Some(Self {
            inputs,
            flow: Arc::clone(&flow),
        });
        flow
    }
}

pub(super) struct SourceValueRecovery;

impl SourceValueRecovery {
    pub(super) fn recover<State: SourceVariableContext>(
        method: &mut SemanticMethod<State>,
        mode: RecoveryMode,
        bindings: &std::collections::BTreeSet<crate::ir::analysis::SsaVar>,
        cache: &mut Option<SourceFlowCache>,
    ) -> Result<bool, ValueRecoveryError> {
        method.normalize_source()?;
        let mut changed = false;
        loop {
            loop {
                let initialization = SourceInitializationRecovery::apply(method.body_mut())?;
                if initialization {
                    method.normalize_source_variables()?;
                    *cache = None;
                }
                crate::ir::SemanticSiteNumbering::assign(method.body_mut())?;
                let graph = ValueFlowGraph::build_source(method.body(), bindings, cache)?;
                let plan = graph.schedule(mode)?;
                #[cfg(debug_assertions)]
                let before_schedule =
                    crate::ir::semantic::SemanticControlTopology::analyze(method.body());
                let schedule = ValueSchedule::compile(plan.actions, ValueIdentity::Source)?;
                let source = schedule.apply(method.body_mut())?;
                #[cfg(debug_assertions)]
                Self::verify_topology(method, &before_schedule, "source-value-schedule")?;
                let motion = LoopInvariantMotion::apply(method.body_mut())?;
                if motion {
                    *cache = None;
                }
                changed |= initialization || source || motion;
                if !initialization && !source && !motion {
                    break;
                }
                method.normalize_source()?;
            }
            let predicates = PredicateRegionFormation::apply(method.body_mut())?;
            changed |= predicates;
            if !predicates {
                break;
            }
            *cache = None;
            method.normalize_source()?;
        }
        Ok(changed)
    }

    #[cfg(debug_assertions)]
    fn verify_topology<State: SourceVariableContext>(
        method: &SemanticMethod<State>,
        before: &crate::ir::semantic::SemanticControlTopology,
        transform: &'static str,
    ) -> Result<(), ValueRecoveryError> {
        let after = crate::ir::semantic::SemanticControlTopology::analyze(method.body());
        if before != &after {
            return Err(SemanticFoldError::ControlTopologyChanged { transform }.into());
        }
        Ok(())
    }
}
