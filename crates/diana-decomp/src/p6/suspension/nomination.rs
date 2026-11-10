//! Temporary owning-stage nominations. Recovery consumes these records once;
//! none is stored in a published RegionFacts or consumed downstream.
use super::*;

// One fixed-size construction scratch value; boxing would add an infallible
// allocation without reducing input-dependent storage or traversal depth.
#[allow(clippy::large_enum_variant)]
pub(in crate::p6) enum Nomination {
    Body(CoroutineBodyPlan),
    Flow(CoroutineFlowPlan),
}

#[derive(Debug)]
pub(in crate::p6) struct CoroutineBodyPlan {
    pub(in crate::p6) site: SuspensionPlan,
    pub(in crate::p6) root: RegionId,
    pub(in crate::p6) prefix: Vec<RegionId>,
    pub(in crate::p6) dispatch: RegionId,
    pub(in crate::p6) initial: RegionId,
    pub(in crate::p6) resume: RegionId,
    pub(in crate::p6) invalid: RegionId,
    pub(in crate::p6) entry_check: OpId,
    pub(in crate::p6) sequence: Vec<RegionId>,
    pub(in crate::p6) operations: Vec<ProtocolEvent>,
    pub(in crate::p6) implicit_locals: Vec<crate::p5::LocalId>,
}

/// Exact local correspondence used only while qualifying the closed body.
/// The selected protocol retains its calls, copies and original controls.
#[derive(Clone, Copy, Debug)]
pub(in crate::p6) struct SuspensionPlan {
    pub(in crate::p6) dispatch: RegionId,
    pub(in crate::p6) control: RegionId,
    pub(in crate::p6) header: RegionId,
    pub(in crate::p6) normal_body: RegionId,
    pub(in crate::p6) suspended_body: RegionId,
    pub(in crate::p6) state: i32,
    pub(in crate::p6) state_store: OpId,
    pub(in crate::p6) call: OpId,
    pub(in crate::p6) result: crate::p5::LocalId,
    pub(in crate::p6) merged: crate::p5::LocalId,
    pub(in crate::p6) continuation_argument: u32,
    pub(in crate::p6) marker: crate::p5::LocalId,
    pub(in crate::p6) predicate: crate::p2::BlockId,
    pub(in crate::p6) normal: LinkId,
    pub(in crate::p6) suspended: LinkId,
    pub(in crate::p6) marker_return: crate::p2::BlockId,
    pub(in crate::p6) normal_copy: crate::p5::PlanId,
    pub(in crate::p6) resume: RegionId,
    pub(in crate::p6) resume_check: OpId,
    pub(in crate::p6) resume_copy: crate::p5::PlanId,
}
/// Source order selected from the existing hierarchy for a closed acyclic
/// continuation protocol. Original controls and all ABI nodes remain owned.
#[derive(Debug)]
pub(in crate::p6) struct CoroutineFlowPlan {
    pub(in crate::p6) source: Vec<RegionId>,
    pub(in crate::p6) entry_check: OpId,
    pub(in crate::p6) forwarding: Option<CopyInterval>,
    pub(in crate::p6) abi: Vec<NodeId>,
    pub(in crate::p6) sites: Vec<CoroutineSite>,
    pub(in crate::p6) protocol_operations: Vec<OpId>,
    pub(in crate::p6) gotos: Vec<LinkId>,
    pub(in crate::p6) abi_entries: Vec<NodeId>,
    pub(in crate::p6) guards: Vec<CoroutineGuardControl>,
    pub(in crate::p6) completions: Vec<(RegionId, Option<NodeId>)>,
    pub(in crate::p6) abi_gotos: Vec<LinkId>,
    pub(in crate::p6) retired_transfers: Vec<LinkId>,
    pub(in crate::p6) cycle: Option<CoroutineCyclePlan>,
    pub(in crate::p6) explicit_predicates: Vec<NodeId>,
}
#[derive(Debug)]
pub(in crate::p6) struct CoroutineCyclePlan {
    pub(in crate::p6) condition: LoopCondition,
    pub(in crate::p6) initial: Vec<RegionId>,
    pub(in crate::p6) body: Vec<RegionId>,
    pub(in crate::p6) tail: Vec<RegionId>,
    pub(in crate::p6) implicit_latch: LinkId,
    pub(in crate::p6) initial_header: LinkId,
}
#[derive(Clone, Copy, Debug)]
pub(in crate::p6) struct CoroutineGuardControl {
    pub(in crate::p6) control: RegionId,
    pub(in crate::p6) header: RegionId,
    pub(in crate::p6) normal: RegionId,
}
#[derive(Clone, Debug)]
pub(in crate::p6) struct CoroutineSite {
    pub(in crate::p6) state: i32,
    pub(in crate::p6) call: OpId,
    pub(in crate::p6) continuation_argument: u32,
    pub(in crate::p6) guard: NodeId,
    pub(in crate::p6) normal: LinkId,
    pub(in crate::p6) suspended: LinkId,
    pub(in crate::p6) resume: Vec<NodeId>,
    pub(in crate::p6) spills: Vec<(OpId, OpId)>,
}
