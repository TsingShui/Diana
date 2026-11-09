use super::{RegionError, build};
use crate::p2::{BlockId, EdgeId, ExceptionCompletion, ExitId, OpId};
use crate::p5::{CopyLocation, GroupId, PlanId, StorageGap, StorageRead, ThrowSite};

/// Closed control roles shared with P7; original operations remain P6 leaves.
pub type ControlLayout = crate::recovery::ControlLayout;

macro_rules! ids { ($($n:ident),*) => {$(#[derive(Clone,Copy,Debug,Eq,Hash,Ord,PartialEq,PartialOrd,serde::Serialize)] #[serde(transparent)] pub struct $n(pub u32);)*}; }
ids!(
    NodeId,
    LinkId,
    RegionId,
    LoopId,
    ChainId,
    ConditionId,
    TryId,
    SyncId,
    LabelId,
    ThrowSiteId
);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelKind {
    BlockEntry,
    Handler,
    Continuation,
}
#[derive(Clone, Copy, Debug)]
pub struct LabelPlan {
    pub(super) id: LabelId,
    pub(super) node: NodeId,
    pub(super) block: BlockId,
    pub(super) kind: LabelKind,
    pub(super) owner: RegionId,
}
impl LabelPlan {
    pub fn id(&self) -> LabelId {
        self.id
    }
    pub fn node(&self) -> NodeId {
        self.node
    }
    pub fn block(&self) -> BlockId {
        self.block
    }
    pub fn kind(&self) -> LabelKind {
        self.kind
    }
    pub fn owner(&self) -> RegionId {
        self.owner
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonitorRole {
    Acquire,
    Release,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationOwner {
    Ordinary {
        node: NodeId,
        region: RegionId,
    },
    Construction {
        group: GroupId,
        role: crate::p5::ConstructionRole,
    },
    CatchCapture(TryId),
    Synchronized {
        scope: SyncId,
        role: MonitorRole,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalOwner {
    Ordinary(RegionId),
    IfCondition(RegionId),
    LoopCondition(LoopId),
    SwitchSelector(RegionId),
    SynchronizedExit(SyncId),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Continuation {
    None,
    Parent,
    Next(NodeId),
    Goto(LabelId),
    Break(RegionId),
    Continue(LoopId),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SiteProtection {
    Explicit,
    Natural,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkOrigin {
    WithinBlock(BlockId),
    Edge(EdgeId),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NormalLink {
    pub(super) id: LinkId,
    pub(super) from: NodeId,
    pub(super) to: NodeId,
    pub(super) origin: LinkOrigin,
}
impl NormalLink {
    pub fn id(&self) -> LinkId {
        self.id
    }
    pub fn from(&self) -> NodeId {
        self.from
    }
    pub fn to(&self) -> NodeId {
        self.to
    }
    pub fn origin(&self) -> LinkOrigin {
        self.origin
    }
}
/// An ordered portion of one original block. Splits isolate monitor and protection
/// boundaries; they add no original edge, instruction, effect or SSA definition.
#[derive(Clone, Debug)]
pub struct Node {
    pub(super) id: NodeId,
    pub(super) block: BlockId,
    pub(super) operations: std::ops::Range<u32>,
    pub(super) terminal: bool,
    pub(super) reachable: bool,
}
impl Node {
    pub fn id(&self) -> NodeId {
        self.id
    }
    pub fn block(&self) -> BlockId {
        self.block
    }
    pub fn operations(&self) -> std::ops::Range<u32> {
        self.operations.clone()
    }
    pub fn has_terminal(&self) -> bool {
        self.terminal
    }
    pub fn reachable(&self) -> bool {
        self.reachable
    }
}
#[derive(Debug)]
pub struct BlockPlan {
    pub(super) block: BlockId,
    pub(super) parts: Vec<NodeId>,
}
impl BlockPlan {
    pub fn block(&self) -> BlockId {
        self.block
    }
    pub fn parts(&self) -> &[NodeId] {
        &self.parts
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoopForm {
    While(LoopCondition),
    DoWhile(LoopCondition),
    Infinite,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoopCondition {
    pub(super) node: NodeId,
    pub(super) repeat: LinkId,
    pub(super) exit: LinkId,
}
impl LoopCondition {
    pub fn node(self) -> NodeId {
        self.node
    }
    pub fn repeat(self) -> LinkId {
        self.repeat
    }
    pub fn exit(self) -> LinkId {
        self.exit
    }
}
#[derive(Debug)]
pub struct LoopPlan {
    pub(super) id: LoopId,
    pub(super) header: NodeId,
    pub(super) members: Vec<NodeId>,
    pub(super) evaluation: Vec<NodeId>,
    pub(super) lexical_members: Vec<NodeId>,
    pub(super) private_exits: Vec<LinkId>,
    pub(super) private_exit_corridors: Vec<PrivateLoopExit>,
    pub(super) latches: Vec<LinkId>,
    pub(super) exits: Vec<LinkId>,
    pub(super) follow: Option<NodeId>,
    pub(super) form: LoopForm,
}
/// P6-private ownership evidence from the single lexical-exit construction.
/// The public exit index remains unchanged; branch admission needs the complete
/// corridor and its original stopping target, not just that index's existence.
#[derive(Debug)]
pub(super) struct PrivateLoopExit {
    pub(super) exit: LinkId,
    pub(super) nodes: Vec<NodeId>,
    pub(super) follow: Option<NodeId>,
}
impl LoopPlan {
    pub fn id(&self) -> LoopId {
        self.id
    }
    pub fn header(&self) -> NodeId {
        self.header
    }
    /// Established cycle core, including internally entered handler rejoin
    /// corridors when normal-dominance roots alone cannot establish the loop.
    pub fn members(&self) -> &[NodeId] {
        &self.members
    }
    /// Original ordered predicate evaluation, separate from repeated body work.
    pub fn evaluation(&self) -> &[NodeId] {
        &self.evaluation
    }
    /// Backedge core plus proved private lexical exit content.
    pub fn lexical_members(&self) -> &[NodeId] {
        &self.lexical_members
    }
    /// Original exits whose closed private corridors are lexical body content.
    pub fn private_exits(&self) -> &[LinkId] {
        &self.private_exits
    }
    pub fn latches(&self) -> &[LinkId] {
        &self.latches
    }
    pub fn exits(&self) -> &[LinkId] {
        &self.exits
    }
    pub fn follow(&self) -> Option<NodeId> {
        self.follow
    }
    pub fn condition(&self) -> Option<NodeId> {
        match self.form {
            LoopForm::While(c) | LoopForm::DoWhile(c) => Some(c.node),
            LoopForm::Infinite => None,
        }
    }
    /// Original tail predicate or head. Other targets, including shared
    /// body updates, remain explicit; no source form retargets a link.
    pub fn continue_target(&self) -> NodeId {
        match self.form {
            LoopForm::DoWhile(c) => c.node,
            _ => self.header,
        }
    }
    pub fn form(&self) -> LoopForm {
        self.form
    }
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CatchTarget {
    Handler(NodeId),
    Uncaught,
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HandlerClause {
    pub order: u32,
    pub catch: Option<crate::TypeId>,
    pub target: CatchTarget,
}
#[derive(Debug)]
pub struct HandlerChain {
    pub(super) id: ChainId,
    pub(super) clauses: Vec<HandlerClause>,
}
impl HandlerChain {
    pub fn id(&self) -> ChainId {
        self.id
    }
    pub fn clauses(&self) -> &[HandlerClause] {
        &self.clauses
    }
}
#[derive(Debug)]
pub struct ThrowPlan {
    pub(super) id: ThrowSiteId,
    pub(super) site: ThrowSite,
    pub(super) node: NodeId,
    pub(super) completion: ExceptionCompletion,
    pub(super) chain: ChainId,
    pub(super) edges: Vec<EdgeId>,
    pub(super) snapshots: Vec<PlanId>,
    pub(super) construction: Option<GroupId>,
    pub(super) protection: SiteProtection,
}
impl ThrowPlan {
    pub fn protection(&self) -> SiteProtection {
        self.protection
    }
    pub fn id(&self) -> ThrowSiteId {
        self.id
    }
    pub fn site(&self) -> ThrowSite {
        self.site
    }
    pub fn node(&self) -> NodeId {
        self.node
    }
    pub fn completion(&self) -> ExceptionCompletion {
        self.completion
    }
    pub fn chain(&self) -> ChainId {
        self.chain
    }
    pub fn edges(&self) -> &[EdgeId] {
        &self.edges
    }
    pub fn snapshots(&self) -> &[PlanId] {
        &self.snapshots
    }
    pub fn construction(&self) -> Option<GroupId> {
        self.construction
    }
}
#[derive(Debug)]
pub struct HandlerEntry {
    pub(super) node: NodeId,
    pub(super) capture: Option<OpId>,
    pub(super) receives: Vec<PlanId>,
}
impl HandlerEntry {
    pub fn node(&self) -> NodeId {
        self.node
    }
    pub fn capture(&self) -> Option<OpId> {
        self.capture
    }
    pub fn receives(&self) -> &[PlanId] {
        &self.receives
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferAction {
    Goto(LabelId),
    Next(NodeId),
    IfArm { region: RegionId, taken: bool },
    SwitchArm { region: RegionId },
    LoopCondition { region: RegionId, repeat: bool },
    Break(RegionId),
    Continue(LoopId),
    EnterSynchronized(RegionId),
    LeaveSynchronized { region: RegionId, release: OpId },
    SynchronizedReleased { region: RegionId, release: OpId },
}
#[derive(Debug)]
pub struct LinkPlan {
    pub(super) link: LinkId,
    pub(super) action: TransferAction,
    pub(super) copies: Vec<PlanId>,
}
impl LinkPlan {
    pub fn link(&self) -> LinkId {
        self.link
    }
    pub fn action(&self) -> TransferAction {
        self.action
    }
    pub fn copies(&self) -> &[PlanId] {
        &self.copies
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExceptionDelivery {
    Explicit,
    Catch(TryId),
    ImpossibleRelease(OpId),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EdgeRoute {
    Normal(LinkId),
    Exception {
        site: ThrowSiteId,
        order: u32,
        delivery: ExceptionDelivery,
    },
}
#[derive(Clone, Debug)]
pub struct EdgePlan {
    pub(super) edge: EdgeId,
    pub(super) route: EdgeRoute,
}
impl EdgePlan {
    pub fn edge(&self) -> EdgeId {
        self.edge
    }
    pub fn route(&self) -> EdgeRoute {
        self.route
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopyOwner {
    Original,
    Construction(GroupId),
    SynchronizedExit(SyncId),
}
#[derive(Clone, Debug)]
pub struct CopyPlacement {
    pub plan: PlanId,
    pub location: CopyLocation,
    pub owner: CopyOwner,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryReason {
    Irreducible,
    MultipleEntry,
    Overlap,
    ProtectionBoundary,
    LockBoundary,
    NoCommonFollow,
    ConstructionSplit,
    Unstructured,
}
/// One original predicate of a condition region. `select` is the delivered link
/// that reaches the shared result arm directly or through supplied empty-goto
/// routing; `other` leaves the test. The zone between
/// the previous test and this predicate stays in the region as its own part.
#[derive(Clone, Copy, Debug)]
pub struct ConditionTest {
    pub(super) predicate: NodeId,
    pub(super) select: LinkId,
    pub(super) other: LinkId,
}
impl ConditionTest {
    pub fn predicate(&self) -> NodeId {
        self.predicate
    }
    pub fn select(&self) -> LinkId {
        self.select
    }
    pub fn other(&self) -> LinkId {
        self.other
    }
}
/// Ordered short-circuit tests. Every `select` reaches the shared arm directly
/// or through original links in the optional final routing child;
/// the last `other` link reaches the other result arm, common follow or an
/// enclosing completion boundary. A no-body boundary remains a real outlet.
#[derive(Debug)]
pub struct ConditionPlan {
    pub(super) id: ConditionId,
    pub(super) tests: Vec<ConditionTest>,
    /// Original empty-goto links owned by the optional final routing child.
    pub(super) routing: Vec<LinkId>,
    pub(super) shared: NodeId,
    pub(super) else_arm: bool,
    /// The final other edge leaves at a context boundary, not the common follow.
    pub(super) other_exit: bool,
    pub(super) explicit: Option<ConditionSchedule>,
}
/// Explicit alternative of an accepted condition. Child interiors, entry and
/// abrupt-exit scopes retain their source plans. `completions` supplies every
/// child's normal delivery under this order; `gotos` supplies original links
/// whose implicit delivery crosses the changed child boundaries.
#[derive(Debug)]
pub struct ConditionSchedule {
    pub(super) children: Vec<RegionId>,
    pub(super) completions: Vec<ConditionCompletion>,
    pub(super) gotos: Vec<LinkId>,
}
/// Normal completion of an intact child under the explicit parent layout.
/// This carries no edge copies: those remain on their original selected links.
#[derive(Clone, Copy, Debug)]
pub struct ConditionCompletion {
    pub(super) child: RegionId,
    pub(super) continuation: Continuation,
}
impl ConditionCompletion {
    pub fn child(&self) -> RegionId {
        self.child
    }
    pub fn continuation(&self) -> Continuation {
        self.continuation
    }
}
impl ConditionSchedule {
    pub fn children(&self) -> &[RegionId] {
        &self.children
    }
    pub fn completions(&self) -> &[ConditionCompletion] {
        &self.completions
    }
    pub fn gotos(&self) -> &[LinkId] {
        &self.gotos
    }
}
impl ConditionPlan {
    pub fn id(&self) -> ConditionId {
        self.id
    }
    pub fn tests(&self) -> &[ConditionTest] {
        &self.tests
    }
    pub fn shared(&self) -> NodeId {
        self.shared
    }
    pub fn routing(&self) -> &[LinkId] {
        &self.routing
    }
    /// Present for every selected Condition region; rejected proposals have no
    /// layout. Entry, follow and exact predicate portions retain their source IDs.
    pub fn explicit(&self) -> Option<&ConditionSchedule> {
        self.explicit.as_ref()
    }
    /// Whether the last test's other link reaches a second result arm rather
    /// than leaving the region directly.
    pub fn else_arm(&self) -> bool {
        self.else_arm
    }
    /// No body is owned for this arm, but its original boundary transfer must
    /// execute instead of falling through the condition's normal completion.
    pub fn other_exit(&self) -> bool {
        self.other_exit
    }
}
#[derive(Debug)]
pub enum RegionKind {
    Block(NodeId),
    Sequence,
    If {
        true_link: LinkId,
        false_link: LinkId,
    },
    /// Ordered tests whose selected edges share one result arm. Each test keeps
    /// its original predicate, links and evaluation order.
    Condition(ConditionId),
    Loop(LoopId),
    Switch {
        arms: Vec<LinkId>,
        /// One child per group, in lexical fallthrough order. Every original
        /// selector link occurs once; None retains the explicit shared body.
        cases: Option<Vec<Vec<LinkId>>>,
    },
    LabelledBlock {
        target: NodeId,
    },
    Try(TryId),
    Synchronized(SyncId),
    Graph {
        reason: RecoveryReason,
        entries: Vec<NodeId>,
        exits: Vec<LinkId>,
    },
}
/// Children are positional: If=(header,true,false),
/// Loop=(body,predicate?,update?),
/// Switch=(header,body) or (header,case-groups...), Try=(body,handlers...),
/// Sync=(enter,body,cleanup...).
/// A display alternative over selected If regions. The source tree stays intact.
#[derive(Debug, serde::Serialize)]
pub struct DecisionPlan {
    pub(super) tests: Vec<DecisionTest>,
    pub(super) default: RegionId,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct DecisionTest {
    pub(super) region: RegionId,
    pub(super) header: RegionId,
    pub(super) predicate: NodeId,
    pub(super) argument: u32,
    pub(super) key: i32,
    pub(super) selected: LinkId,
    pub(super) remaining: LinkId,
    pub(super) body: RegionId,
}
impl DecisionPlan {
    pub fn tests(&self) -> &[DecisionTest] {
        &self.tests
    }
    pub fn default(&self) -> RegionId {
        self.default
    }
}
impl DecisionTest {
    pub fn region(&self) -> RegionId {
        self.region
    }
    pub fn header(&self) -> RegionId {
        self.header
    }
    pub fn predicate(&self) -> NodeId {
        self.predicate
    }
    pub fn argument(&self) -> u32 {
        self.argument
    }
    pub fn key(&self) -> i32 {
        self.key
    }
    pub fn selected(&self) -> LinkId {
        self.selected
    }
    pub fn remaining(&self) -> LinkId {
        self.remaining
    }
    pub fn body(&self) -> RegionId {
        self.body
    }
}
#[derive(Debug)]
pub struct Region {
    pub(super) decision: Option<DecisionPlan>,
    pub(super) id: RegionId,
    pub(super) kind: RegionKind,
    pub(super) entry: Option<NodeId>,
    pub(super) follow: Option<NodeId>,
    pub(super) children: Vec<RegionId>,
    pub(super) member_range: Option<std::ops::Range<usize>>,
    pub(super) member_count: usize,
    pub(super) continuation: Continuation,
}
impl Region {
    pub fn decision(&self) -> Option<&DecisionPlan> {
        self.decision.as_ref()
    }
    pub fn continuation(&self) -> Continuation {
        self.continuation
    }
    pub fn id(&self) -> RegionId {
        self.id
    }
    pub fn kind(&self) -> &RegionKind {
        &self.kind
    }
    pub fn entry(&self) -> Option<NodeId> {
        self.entry
    }
    pub fn follow(&self) -> Option<NodeId> {
        self.follow
    }
    pub fn children(&self) -> &[RegionId] {
        &self.children
    }
}
/// One cleanup spelling with its original normal/exceptional instruction pairs.
/// The original capture, rethrow and normal duplicate remain owned source facts.
#[derive(Debug)]
pub struct FinallyPlan {
    pub(super) normal: Vec<NodeId>,
    pub(super) operations: Vec<(OpId, OpId)>,
    pub(super) predicates: Vec<(NodeId, NodeId)>,
    pub(super) capture: OpId,
    pub(super) rethrows: Vec<NodeId>,
    /// Explicit cleanup exits, distinct from the implicit pending rethrows.
    pub(super) terminals: Vec<(NodeId, NodeId)>,
}
impl FinallyPlan {
    pub fn normal(&self) -> &[NodeId] {
        &self.normal
    }
    pub fn operations(&self) -> &[(OpId, OpId)] {
        &self.operations
    }
    pub fn predicates(&self) -> &[(NodeId, NodeId)] {
        &self.predicates
    }
    pub fn capture(&self) -> OpId {
        self.capture
    }
    pub fn rethrows(&self) -> &[NodeId] {
        &self.rethrows
    }
    pub fn terminals(&self) -> &[(NodeId, NodeId)] {
        &self.terminals
    }
}
#[derive(Debug)]
pub struct TryPlan {
    pub(super) id: TryId,
    pub(super) chain: ChainId,
    pub(super) clause_range: std::ops::Range<u32>,
    pub(super) outer_sync: Option<SyncId>,
    pub(super) entry: NodeId,
    pub(super) body: Vec<NodeId>,
    pub(super) handlers: Vec<NodeId>,
    pub(super) handler_bodies: Vec<Vec<NodeId>>,
    pub(super) handler_clauses: Vec<std::ops::Range<u32>>,
    // Owning protection proof: no selected handler resumes shared normal code.
    pub(super) abrupt_handlers: bool,
    pub(super) bindings: Vec<Option<OpId>>,
    pub(super) finally: Option<FinallyPlan>,
    pub(super) members: Vec<NodeId>,
    pub(super) follow: Option<NodeId>,
}
impl TryPlan {
    pub fn id(&self) -> TryId {
        self.id
    }
    pub fn chain(&self) -> ChainId {
        self.chain
    }
    pub fn clauses(&self) -> std::ops::Range<u32> {
        self.clause_range.clone()
    }
    pub fn outer_sync(&self) -> Option<SyncId> {
        self.outer_sync
    }
    pub fn entry(&self) -> NodeId {
        self.entry
    }
    pub fn body(&self) -> &[NodeId] {
        &self.body
    }
    pub fn handlers(&self) -> &[NodeId] {
        &self.handlers
    }
    pub fn handler_bodies(&self) -> &[Vec<NodeId>] {
        &self.handler_bodies
    }
    /// Ordered contiguous original clauses delivered to each unique handler body.
    pub fn handler_clauses(&self) -> &[std::ops::Range<u32>] {
        &self.handler_clauses
    }
    pub fn bindings(&self) -> &[Option<OpId>] {
        &self.bindings
    }
    pub fn finally(&self) -> Option<&FinallyPlan> {
        self.finally.as_ref()
    }
    pub fn members(&self) -> &[NodeId] {
        &self.members
    }
    /// Lexical completion at an actual original target. A guarded footprint
    /// can have other explicit outlets; this does not assert postdomination.
    pub fn follow(&self) -> Option<NodeId> {
        self.follow
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockGap {
    Storage(StorageGap),
    Merge(NodeId),
    NoLocalAcquire(OpId),
    Reference { enter: OpId, exit: OpId },
    RepeatedAcquire(OpId),
}
#[derive(Debug, Eq, PartialEq)]
pub enum LockState {
    Unreachable,
    Known(Vec<OpId>),
    Unknown(LockGap),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockSeparation {
    Read(StorageGap),
    State(LockGap),
    Unreachable,
    ReenteredAcquire,
    NotHeld(NodeId),
    AbruptExit(NodeId),
    EscapingException(ThrowSiteId),
    IncompatibleContinuation { release: OpId, target: NodeId },
    PostReleaseCopy(PlanId),
    LoopContext(LoopId),
    ExternalEntry,
    Region(RecoveryReason),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonitorMode {
    Natural(SyncId),
    Explicit(LockSeparation),
}
#[derive(Clone, Copy, Debug)]
pub struct MonitorPlan {
    pub(super) enter: OpId,
    pub(super) mode: MonitorMode,
}
impl MonitorPlan {
    pub fn enter(&self) -> OpId {
        self.enter
    }
    pub fn mode(&self) -> MonitorMode {
        self.mode
    }
}
#[derive(Debug)]
pub struct LockEvent {
    pub(super) operation: OpId,
    pub(super) receiver: Result<StorageRead, StorageGap>,
    pub(super) before: LockState,
    pub(super) normal: LockState,
    pub(super) exception: LockState,
    pub(super) matched: Option<OpId>,
    pub(super) release_cannot_throw: bool,
}
impl LockEvent {
    pub fn operation(&self) -> OpId {
        self.operation
    }
    pub fn receiver(&self) -> Result<StorageRead, StorageGap> {
        self.receiver
    }
    pub fn before(&self) -> &LockState {
        &self.before
    }
    pub fn normal(&self) -> &LockState {
        &self.normal
    }
    pub fn exception(&self) -> &LockState {
        &self.exception
    }
    pub fn matched(&self) -> Option<OpId> {
        self.matched
    }
    pub fn release_cannot_throw(&self) -> bool {
        self.release_cannot_throw
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncLoopTransfer {
    Break,
    Continue,
}
/// The outlet's original continuation and supplied source role. Selected loop
/// context can complete Follow(update) as Continue without changing its target
/// or phases. Abrupt intents carry no new dispatcher state; a non-representable
/// continuation keeps the scope explicit instead.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncExitKind {
    Follow(NodeId),
    Loop {
        loop_id: LoopId,
        kind: SyncLoopTransfer,
        target: NodeId,
    },
    ReturnVoid {
        terminal: NodeId,
    },
    Rethrow {
        terminal: NodeId,
        /// Original exception-value provenance, not an ambient/current handler token.
        capture: OpId,
        /// The exact original terminal read; do not substitute the capture local.
        exception: StorageRead,
    },
}
#[derive(Debug)]
pub struct SyncExit {
    pub(super) release: OpId,
    pub(super) link: LinkId,
    pub(super) target: NodeId,
    pub(super) kind: SyncExitKind,
    pub(super) routing: Vec<NodeId>,
    pub(super) before: Vec<PlanId>,
    pub(super) after: Vec<PlanId>,
}
impl SyncExit {
    pub fn release(&self) -> OpId {
        self.release
    }
    pub fn release_link(&self) -> LinkId {
        self.link
    }
    pub fn target(&self) -> NodeId {
        self.target
    }
    pub fn kind(&self) -> SyncExitKind {
        self.kind
    }
    pub fn routing(&self) -> &[NodeId] {
        &self.routing
    }
    pub fn before_release(&self) -> &[PlanId] {
        &self.before
    }
    pub fn after_release(&self) -> &[PlanId] {
        &self.after
    }
}
/// Selected compiler cleanup that a natural synchronized boundary represents.
#[derive(Debug)]
pub struct ImplicitMonitor {
    pub(super) cleanups: Vec<(TryId, u32)>,
    pub(super) captures: Vec<OpId>,
    pub(super) rethrows: Vec<NodeId>,
}
impl ImplicitMonitor {
    pub fn cleanups(&self) -> &[(TryId, u32)] {
        &self.cleanups
    }
    pub fn captures(&self) -> &[OpId] {
        &self.captures
    }
    pub fn rethrows(&self) -> &[NodeId] {
        &self.rethrows
    }
}
/// Body nodes include syntax-owned exit anchors. `held_nodes` separately records
/// original pre-release execution; exit routing must not become ordinary locked work.
#[derive(Debug)]
pub struct SyncPlan {
    pub(super) id: SyncId,
    pub(super) enter: OpId,
    pub(super) enter_link: LinkId,
    pub(super) lock: StorageRead,
    pub(super) held: Vec<NodeId>,
    pub(super) body: Vec<NodeId>,
    pub(super) exits: Vec<SyncExit>,
    pub(super) members: Vec<NodeId>,
    pub(super) follow: Option<NodeId>,
    pub(super) separation: Option<LockSeparation>,
    pub(super) implicit: Option<ImplicitMonitor>,
}
impl SyncPlan {
    pub fn implicit(&self) -> Option<&ImplicitMonitor> {
        self.implicit.as_ref()
    }
    pub fn id(&self) -> SyncId {
        self.id
    }
    pub fn enter(&self) -> OpId {
        self.enter
    }
    pub fn enter_link(&self) -> LinkId {
        self.enter_link
    }
    pub fn lock(&self) -> StorageRead {
        self.lock
    }
    pub fn held_nodes(&self) -> &[NodeId] {
        &self.held
    }
    pub fn body(&self) -> &[NodeId] {
        &self.body
    }
    pub fn exits(&self) -> &[SyncExit] {
        &self.exits
    }
    pub fn members(&self) -> &[NodeId] {
        &self.members
    }
    /// The shared normal follow, not a target invented for distinct outlets.
    /// None permits closed explicit outlet continuations with no fallthrough.
    pub fn follow(&self) -> Option<NodeId> {
        self.follow
    }
    pub fn separation(&self) -> Option<LockSeparation> {
        self.separation
    }
}
#[derive(Clone, Debug)]
pub enum PlanningIssue {
    Graph {
        entry: NodeId,
        reason: RecoveryReason,
    },
    Storage {
        operation: Option<OpId>,
        block: BlockId,
        gap: StorageGap,
    },
    Lock {
        operation: OpId,
        gap: LockGap,
    },
    Protection {
        site: ThrowSite,
        reason: RecoveryReason,
    },
}
/// A local continuation protocol. The dispatch and resume/error bodies remain
/// original code; only the selected call/marker/normal-copy position is combined.
#[derive(Clone, Copy, Debug)]
pub struct SuspensionPlan {
    pub(super) dispatch: RegionId,
    pub(super) control: RegionId,
    pub(super) header: RegionId,
    pub(super) normal_body: RegionId,
    pub(super) suspended_body: RegionId,
    pub(super) state: i32,
    pub(super) state_store: OpId,
    pub(super) call: OpId,
    pub(super) result: crate::p5::LocalId,
    pub(super) merged: crate::p5::LocalId,
    pub(super) continuation_argument: u32,
    pub(super) marker: crate::p5::LocalId,
    pub(super) predicate: BlockId,
    pub(super) normal: LinkId,
    pub(super) suspended: LinkId,
    pub(super) marker_return: BlockId,
    pub(super) normal_copy: PlanId,
    pub(super) resume: RegionId,
    pub(super) resume_check: OpId,
    pub(super) resume_link: LinkId,
    pub(super) resume_copy: PlanId,
}
impl SuspensionPlan {
    pub fn call(&self) -> OpId {
        self.call
    }
    pub fn control(&self) -> RegionId {
        self.control
    }
    pub fn header(&self) -> RegionId {
        self.header
    }
    pub fn bodies(&self) -> [RegionId; 2] {
        [self.normal_body, self.suspended_body]
    }
    pub fn dispatch(&self) -> RegionId {
        self.dispatch
    }
    pub fn state(&self) -> i32 {
        self.state
    }
    pub fn state_store(&self) -> OpId {
        self.state_store
    }
    pub fn result(&self) -> crate::p5::LocalId {
        self.result
    }
    pub fn merged(&self) -> crate::p5::LocalId {
        self.merged
    }
    pub fn continuation_argument(&self) -> u32 {
        self.continuation_argument
    }
    pub fn marker(&self) -> crate::p5::LocalId {
        self.marker
    }
    pub fn predicate(&self) -> BlockId {
        self.predicate
    }
    pub fn links(&self) -> [LinkId; 2] {
        [self.normal, self.suspended]
    }
    pub fn marker_return(&self) -> BlockId {
        self.marker_return
    }
    pub fn normal_copy(&self) -> PlanId {
        self.normal_copy
    }
    pub fn resume(&self) -> RegionId {
        self.resume
    }
    pub fn resume_check(&self) -> OpId {
        self.resume_check
    }
    pub fn resume_link(&self) -> LinkId {
        self.resume_link
    }
    pub fn resume_copy(&self) -> PlanId {
        self.resume_copy
    }
}
/// Source-body order over an established suspension. ABI-only code remains
/// explicitly attached, so it is neither treated as pure nor erased as dead code.
#[derive(Clone, Copy, Debug)]
pub enum CoroutineOperationRole {
    Marker,
    LabelRead,
    Literal,
    PublishState,
    ResumeCheck,
    InvalidState,
}
#[derive(Clone, Copy, Debug)]
pub struct CoroutineOperation {
    pub(super) operation: OpId,
    pub(super) role: CoroutineOperationRole,
}
impl CoroutineOperation {
    pub fn operation(&self) -> OpId {
        self.operation
    }
    pub fn role(&self) -> CoroutineOperationRole {
        self.role
    }
}
#[derive(Debug)]
pub struct CoroutineBodyPlan {
    pub(super) root: RegionId,
    pub(super) prefix: RegionId,
    pub(super) dispatch: RegionId,
    pub(super) initial: RegionId,
    pub(super) resume: RegionId,
    pub(super) invalid: RegionId,
    pub(super) entry_check: OpId,
    pub(super) sequence: Vec<RegionId>,
    pub(super) operations: Vec<CoroutineOperation>,
    pub(super) implicit_locals: Vec<crate::p5::LocalId>,
}
/// A closed normal-run loop with one continuation reentry. The original node
/// partition remains intact; this is a selected presentation over those nodes.
#[derive(Debug)]
pub struct CoroutineLoopPlan {
    pub(super) initial: Vec<NodeId>,
    pub(super) body: Vec<NodeId>,
    pub(super) tail: Vec<NodeId>,
    pub(super) abi: Vec<NodeId>,
    pub(super) abi_entries: Vec<NodeId>,
    pub(super) condition: LoopCondition,
    pub(super) call: OpId,
    pub(super) continuation_argument: u32,
    pub(super) state: i32,
    pub(super) protocol_operations: Vec<OpId>,
    pub(super) normal_links: Vec<LinkId>,
    pub(super) spills: Vec<(OpId, OpId)>,
}
/// Source order selected from the existing hierarchy for a closed acyclic
/// continuation protocol. Original controls and all ABI nodes remain owned.
#[derive(Debug)]
pub struct CoroutineFlowPlan {
    pub(super) source: Vec<RegionId>,
    pub(super) abi: Vec<NodeId>,
    pub(super) source_nodes: Vec<NodeId>,
    pub(super) sites: Vec<CoroutineSite>,
    pub(super) protocol_operations: Vec<OpId>,
    pub(super) gotos: Vec<LinkId>,
    pub(super) abi_entries: Vec<NodeId>,
    pub(super) guards: Vec<CoroutineGuardControl>,
    pub(super) completions: Vec<(RegionId, Option<NodeId>)>,
    pub(super) abi_gotos: Vec<LinkId>,
    pub(super) retired_transfers: Vec<LinkId>,
    pub(super) cycle: Option<CoroutineCyclePlan>,
    pub(super) explicit_predicates: Vec<NodeId>,
}
#[derive(Debug)]
pub struct CoroutineCyclePlan {
    pub(super) header: NodeId,
    pub(super) condition: LoopCondition,
    pub(super) initial: Vec<RegionId>,
    pub(super) body: Vec<RegionId>,
    pub(super) tail: Vec<RegionId>,
    pub(super) implicit_latch: LinkId,
    pub(super) initial_header: LinkId,
}
impl CoroutineCyclePlan {
    pub fn header(&self) -> NodeId {
        self.header
    }
    pub fn condition(&self) -> LoopCondition {
        self.condition
    }
    pub fn initial(&self) -> &[RegionId] {
        &self.initial
    }
    pub fn body(&self) -> &[RegionId] {
        &self.body
    }
    pub fn tail(&self) -> &[RegionId] {
        &self.tail
    }
    pub fn implicit_latch(&self) -> LinkId {
        self.implicit_latch
    }
    pub fn initial_header(&self) -> LinkId {
        self.initial_header
    }
}
#[derive(Clone, Copy, Debug)]
pub struct CoroutineGuardControl {
    pub(super) control: RegionId,
    pub(super) header: RegionId,
    pub(super) normal: RegionId,
}
impl CoroutineGuardControl {
    pub fn control(&self) -> RegionId {
        self.control
    }
    pub fn header(&self) -> RegionId {
        self.header
    }
    pub fn normal(&self) -> RegionId {
        self.normal
    }
}
#[derive(Clone, Debug)]
pub struct CoroutineSite {
    pub(super) state: i32,
    pub(super) call: OpId,
    pub(super) continuation_argument: u32,
    pub(super) guard: NodeId,
    pub(super) normal: LinkId,
    pub(super) suspended: LinkId,
    pub(super) resume: Vec<NodeId>,
    pub(super) spills: Vec<(OpId, OpId)>,
}
impl CoroutineFlowPlan {
    pub fn cycle(&self) -> Option<&CoroutineCyclePlan> {
        self.cycle.as_ref()
    }
    pub fn explicit_predicates(&self) -> &[NodeId] {
        &self.explicit_predicates
    }
    pub fn guards(&self) -> &[CoroutineGuardControl] {
        &self.guards
    }
    pub fn completions(&self) -> &[(RegionId, Option<NodeId>)] {
        &self.completions
    }
    pub fn abi_gotos(&self) -> &[LinkId] {
        &self.abi_gotos
    }
    pub fn retired_transfers(&self) -> &[LinkId] {
        &self.retired_transfers
    }
    pub fn source(&self) -> &[RegionId] {
        &self.source
    }
    pub fn abi(&self) -> &[NodeId] {
        &self.abi
    }
    pub fn source_nodes(&self) -> &[NodeId] {
        &self.source_nodes
    }
    pub fn sites(&self) -> &[CoroutineSite] {
        &self.sites
    }
    pub fn protocol_operations(&self) -> &[OpId] {
        &self.protocol_operations
    }
    pub fn gotos(&self) -> &[LinkId] {
        &self.gotos
    }
    pub fn abi_entries(&self) -> &[NodeId] {
        &self.abi_entries
    }
}
impl CoroutineSite {
    pub fn state(&self) -> i32 {
        self.state
    }
    pub fn call(&self) -> OpId {
        self.call
    }
    pub fn continuation_argument(&self) -> u32 {
        self.continuation_argument
    }
    pub fn guard(&self) -> NodeId {
        self.guard
    }
    pub fn normal(&self) -> LinkId {
        self.normal
    }
    pub fn suspended(&self) -> LinkId {
        self.suspended
    }
    pub fn resume(&self) -> &[NodeId] {
        &self.resume
    }
    pub fn spills(&self) -> &[(OpId, OpId)] {
        &self.spills
    }
}
impl CoroutineLoopPlan {
    pub fn initial(&self) -> &[NodeId] {
        &self.initial
    }
    pub fn body(&self) -> &[NodeId] {
        &self.body
    }
    pub fn tail(&self) -> &[NodeId] {
        &self.tail
    }
    pub fn abi(&self) -> &[NodeId] {
        &self.abi
    }
    pub fn abi_entries(&self) -> &[NodeId] {
        &self.abi_entries
    }
    pub fn condition(&self) -> LoopCondition {
        self.condition
    }
    pub fn call(&self) -> OpId {
        self.call
    }
    pub fn continuation_argument(&self) -> u32 {
        self.continuation_argument
    }
    pub fn state(&self) -> i32 {
        self.state
    }
    pub fn protocol_operations(&self) -> &[OpId] {
        &self.protocol_operations
    }
    pub fn normal_links(&self) -> &[LinkId] {
        &self.normal_links
    }
    pub fn spills(&self) -> &[(OpId, OpId)] {
        &self.spills
    }
}
impl CoroutineBodyPlan {
    pub fn root(&self) -> RegionId {
        self.root
    }
    pub fn prefix(&self) -> RegionId {
        self.prefix
    }
    pub fn dispatch(&self) -> RegionId {
        self.dispatch
    }
    pub fn initial(&self) -> RegionId {
        self.initial
    }
    pub fn resume(&self) -> RegionId {
        self.resume
    }
    pub fn invalid(&self) -> RegionId {
        self.invalid
    }
    pub fn entry_check(&self) -> OpId {
        self.entry_check
    }
    pub fn sequence(&self) -> &[RegionId] {
        &self.sequence
    }
    pub fn operations(&self) -> &[CoroutineOperation] {
        &self.operations
    }
    pub fn implicit_locals(&self) -> &[crate::p5::LocalId] {
        &self.implicit_locals
    }
}
/// Lock emptiness is retained from the one owning lock-flow computation.
/// Missing computation is not an empty lock stack.
#[derive(Debug)]
pub(super) enum LockEntries {
    Unestablished,
    NoMonitorOperations,
    Analysed(Vec<bool>),
}
impl LockEntries {
    pub fn empty_at(&self, node: NodeId) -> bool {
        match self {
            Self::Unestablished => false,
            Self::NoMonitorOperations => true,
            Self::Analysed(entries) => entries.get(node.0 as usize).copied() == Some(true),
        }
    }
}
pub struct RegionFacts {
    pub(super) controls: Vec<ControlLayout>,
    pub(super) lock_entries: LockEntries,
    pub(super) suspensions: Vec<SuspensionPlan>,
    pub(super) coroutine_body: Option<CoroutineBodyPlan>,
    pub(super) coroutine_loop: Option<CoroutineLoopPlan>,
    pub(super) coroutine_flow: Option<CoroutineFlowPlan>,
    pub(super) nodes: Vec<Node>,
    pub(super) blocks: Vec<BlockPlan>,
    pub(super) links: Vec<NormalLink>,
    pub(super) link_plans: Vec<LinkPlan>,
    pub(super) edges: Vec<EdgePlan>,
    pub(super) loops: Vec<LoopPlan>,
    pub(super) regions: Vec<Region>,
    pub(super) member_nodes: Vec<NodeId>,
    pub(super) member_positions: Vec<usize>,
    pub(super) root: Option<RegionId>,
    pub(super) parents: Vec<Option<RegionId>>,
    pub(super) chains: Vec<HandlerChain>,
    pub(super) throws: Vec<ThrowPlan>,
    pub(super) handlers: Vec<HandlerEntry>,
    pub(super) tries: Vec<TryPlan>,
    pub(super) conditions: Vec<ConditionPlan>,
    pub(super) locks: Vec<LockEvent>,
    pub(super) monitors: Vec<MonitorPlan>,
    pub(super) synchronized: Vec<SyncPlan>,
    pub(super) copies: Vec<CopyPlacement>,
    pub(super) issues: Vec<PlanningIssue>,
    pub(super) exits: Vec<ExitId>,
    pub(super) labels: Vec<LabelPlan>,
    pub(super) operation_owners: Vec<OperationOwner>,
    pub(super) terminal_owners: Vec<TerminalOwner>,
}
impl RegionFacts {
    pub fn controls(&self) -> &[ControlLayout] {
        &self.controls
    }
    pub fn coroutine_body(&self) -> Option<&CoroutineBodyPlan> {
        self.coroutine_body.as_ref()
    }
    pub fn coroutine_loop(&self) -> Option<&CoroutineLoopPlan> {
        self.coroutine_loop.as_ref()
    }
    pub fn coroutine_flow(&self) -> Option<&CoroutineFlowPlan> {
        self.coroutine_flow.as_ref()
    }
    pub fn suspensions(&self) -> &[SuspensionPlan] {
        &self.suspensions
    }
    pub(super) fn empty() -> Self {
        Self {
            controls: Vec::new(),
            suspensions: Vec::new(),
            coroutine_body: None,
            coroutine_loop: None,
            coroutine_flow: None,
            lock_entries: LockEntries::Unestablished,
            nodes: Vec::new(),
            blocks: Vec::new(),
            links: Vec::new(),
            link_plans: Vec::new(),
            edges: Vec::new(),
            loops: Vec::new(),
            regions: Vec::new(),
            member_nodes: Vec::new(),
            member_positions: Vec::new(),
            root: None,
            parents: Vec::new(),
            chains: Vec::new(),
            throws: Vec::new(),
            handlers: Vec::new(),
            tries: Vec::new(),
            conditions: Vec::new(),
            locks: Vec::new(),
            monitors: Vec::new(),
            synchronized: Vec::new(),
            copies: Vec::new(),
            issues: Vec::new(),
            exits: Vec::new(),
            labels: Vec::new(),
            operation_owners: Vec::new(),
            terminal_owners: Vec::new(),
        }
    }
    pub fn labels(&self) -> &[LabelPlan] {
        &self.labels
    }
    pub fn operation_owners(&self) -> &[OperationOwner] {
        &self.operation_owners
    }
    pub fn terminal_owners(&self) -> &[TerminalOwner] {
        &self.terminal_owners
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn blocks(&self) -> &[BlockPlan] {
        &self.blocks
    }
    pub fn links(&self) -> &[NormalLink] {
        &self.links
    }
    pub fn link_plans(&self) -> &[LinkPlan] {
        &self.link_plans
    }
    pub fn edges(&self) -> &[EdgePlan] {
        &self.edges
    }
    pub fn loops(&self) -> &[LoopPlan] {
        &self.loops
    }
    pub fn regions(&self) -> &[Region] {
        &self.regions
    }
    /// Original membership in child traversal order, separate from execution.
    /// A partial construction without the shared index has no member slice.
    pub fn region_nodes(&self, region: RegionId) -> Option<&[NodeId]> {
        if self.member_nodes.len() != self.nodes.len() {
            return None;
        }
        self.member_nodes
            .get(self.regions.get(region.0 as usize)?.member_range.clone()?)
    }
    pub(super) fn members(&self, region: RegionId) -> Result<&[NodeId], RegionError> {
        self.region_nodes(region)
            .ok_or(build::bug("region membership is not established"))
    }
    pub(super) fn membership(
        &self,
        region: RegionId,
    ) -> Result<impl Fn(NodeId) -> bool + '_, RegionError> {
        let range = self
            .regions
            .get(region.0 as usize)
            .and_then(|region| region.member_range.as_ref())
            .ok_or(build::bug("region membership is not established"))?;
        if self.member_positions.len() != self.nodes.len() {
            return Err(build::bug("region member positions are not established"));
        }
        Ok(move |node: NodeId| range.contains(&self.member_positions[node.0 as usize]))
    }
    pub fn root(&self) -> Option<RegionId> {
        self.root
    }
    pub fn parent(&self, id: RegionId) -> Option<RegionId> {
        self.parents.get(id.0 as usize).copied().flatten()
    }
    pub fn handler_chains(&self) -> &[HandlerChain] {
        &self.chains
    }
    pub fn throws(&self) -> &[ThrowPlan] {
        &self.throws
    }
    pub fn handler_entries(&self) -> &[HandlerEntry] {
        &self.handlers
    }
    pub fn tries(&self) -> &[TryPlan] {
        &self.tries
    }
    pub fn conditions(&self) -> &[ConditionPlan] {
        &self.conditions
    }
    pub fn locks(&self) -> &[LockEvent] {
        &self.locks
    }
    pub fn monitor_plans(&self) -> &[MonitorPlan] {
        &self.monitors
    }
    pub fn synchronized(&self) -> &[SyncPlan] {
        &self.synchronized
    }
    pub fn copy_placements(&self) -> &[CopyPlacement] {
        &self.copies
    }
    pub fn issues(&self) -> &[PlanningIssue] {
        &self.issues
    }
    pub fn exits(&self) -> &[ExitId] {
        &self.exits
    }
}
pub struct PlannedMethod<'a> {
    pub(super) storage: crate::p5::StorageMethod<'a>,
    pub(super) facts: RegionFacts,
}
impl<'a> PlannedMethod<'a> {
    pub fn facts(&self) -> &RegionFacts {
        &self.facts
    }
    pub fn storage_facts(&self) -> &crate::p5::StorageFacts {
        self.storage.facts()
    }
    pub fn type_facts(&self) -> &crate::p4::TypeFacts {
        self.storage.type_facts()
    }
    pub fn execution(&self) -> crate::p2::ExecutionFacts<'_, 'a> {
        self.storage.execution()
    }
    pub fn evidence(&self) -> crate::p2::MethodEvidence<'_, 'a> {
        self.storage.evidence()
    }
    pub fn analysis_evidence(&self) -> crate::p5::AnalysisEvidence<'_, 'a> {
        self.storage.analysis_evidence()
    }
    pub fn storage_evidence(&self) -> crate::p5::StorageEvidence<'_, 'a> {
        self.storage.storage_evidence()
    }
    pub fn method_id(&self) -> crate::MethodId {
        self.storage.method_id()
    }
    pub fn class_id(&self) -> crate::ClassDefId {
        self.storage.class_id()
    }
}
pub struct RegionStop<'a> {
    pub(super) storage: crate::p5::StorageMethod<'a>,
    pub(super) error: super::RegionError,
    pub(super) partial: RegionPartial,
}
impl<'a> RegionStop<'a> {
    pub fn error(&self) -> &super::RegionError {
        &self.error
    }
    pub fn partial(&self) -> &RegionPartial {
        &self.partial
    }
    pub fn storage_facts(&self) -> &crate::p5::StorageFacts {
        self.storage.facts()
    }
    pub fn type_facts(&self) -> &crate::p4::TypeFacts {
        self.storage.type_facts()
    }
    pub fn execution(&self) -> crate::p2::ExecutionFacts<'_, 'a> {
        self.storage.execution()
    }
    pub fn evidence(&self) -> crate::p2::MethodEvidence<'_, 'a> {
        self.storage.evidence()
    }
    pub fn analysis_evidence(&self) -> crate::p5::AnalysisEvidence<'_, 'a> {
        self.storage.analysis_evidence()
    }
    pub fn storage_evidence(&self) -> crate::p5::StorageEvidence<'_, 'a> {
        self.storage.storage_evidence()
    }
}
pub struct RegionPartial {
    pub(super) facts: RegionFacts,
}
impl RegionPartial {
    pub fn nodes(&self) -> &[Node] {
        &self.facts.nodes
    }
    pub fn regions(&self) -> &[Region] {
        &self.facts.regions
    }
    pub fn issues(&self) -> &[PlanningIssue] {
        &self.facts.issues
    }
}
