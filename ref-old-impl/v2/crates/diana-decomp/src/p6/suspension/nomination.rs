//! Temporary owning-stage nominations. Recovery consumes these records once;
//! none is stored in a published RegionFacts or consumed downstream.
use super::*;

/// Source order and optional iteration selected from the original hierarchy.
/// Field and parameter entries share these records; all ABI nodes stay owned.
#[derive(Debug)]
pub(in crate::p6) struct ProtocolNomination {
    pub(in crate::p6) source: Vec<RegionId>,
    pub(in crate::p6) entry_check: OpId,
    pub(in crate::p6) forwarding: Option<CopyInterval>,
    pub(in crate::p6) abi: Vec<NodeId>,
    pub(in crate::p6) sites: Vec<CoroutineSite>,
    pub(in crate::p6) operations: Vec<ProtocolEvent>,
    pub(in crate::p6) hidden_locals: Vec<LocalId>,
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
    pub(in crate::p6) publication: Option<(crate::p5::PlanId, LocalId)>,
}
