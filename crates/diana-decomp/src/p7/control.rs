mod build;
mod mapping;
mod protection;
mod regions;
mod synchronized;
mod transfers;

pub(super) use build::construct;

use super::*;
use crate::p2::{EdgeRole, NormalEdge, OpId};
use crate::p5::ThrowSite;
use crate::p6::{self, LinkOrigin, RegionKind, SyncExitKind, SyncLoopTransfer, TransferAction};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Label {
    pub id: LabelId,
    pub name: String,
    pub node: p6::NodeId,
    pub block: crate::p2::BlockId,
    pub owner: ControlId,
    #[serde(serialize_with = "super::debug")]
    pub kind: p6::LabelKind,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferKind {
    Goto(LabelId),
    Next(LabelId),
    Break(ControlId),
    Continue(ControlId),
    IfArm {
        control: ControlId,
        taken: bool,
    },
    SwitchArm(ControlId),
    LoopCondition {
        control: ControlId,
        repeat: bool,
    },
    EnterSync(ControlId),
    LeaveSync {
        control: ControlId,
        release: StatementId,
    },
    ReleasedSync {
        control: ControlId,
        release: StatementId,
    },
}
#[derive(Debug, Serialize)]
pub struct Transfer {
    pub id: TransferId,
    pub from: p6::NodeId,
    pub destination: LabelId,
    pub edge: Option<crate::p2::EdgeId>,
    #[serde(serialize_with = "super::debug")]
    pub role: Option<NormalEdge>,
    pub kind: TransferKind,
    pub copies: Vec<CopyPlanId>,
    pub construction: Option<ConstructionId>,
}
#[derive(Debug, Serialize)]
pub struct CatchType {
    pub original: crate::TypeId,
    pub descriptor: Result<TypeUseId, SourceFailure>,
    pub use_issue: Option<SourceFailure>,
}
#[derive(Debug, Serialize)]
pub struct Clause {
    pub order: u32,
    pub catch: Option<CatchType>,
    pub target: Option<LabelId>,
}
#[derive(Debug, Serialize)]
pub struct ExceptionChain {
    pub id: p6::ChainId,
    pub clauses: Vec<Clause>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    Explicit,
    Catch(ControlId),
    ImpossibleRelease(StatementId),
}
#[derive(Debug, Serialize)]
pub struct ExceptionRoute {
    pub edge: crate::p2::EdgeId,
    pub order: u32,
    pub delivery: Delivery,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionSyntax {
    Natural,
    Explicit,
    Propagate,
    ImpossibleRelease,
}
#[derive(Debug, Serialize)]
pub struct ExceptionSite {
    pub id: SiteId,
    #[serde(serialize_with = "super::debug")]
    pub original: ThrowSite,
    #[serde(serialize_with = "super::debug")]
    pub completion: crate::p2::ExceptionCompletion,
    pub chain: p6::ChainId,
    pub routes: Vec<ExceptionRoute>,
    pub syntax: ExceptionSyntax,
    pub construction: Option<ConstructionId>,
    pub source_snapshots: Vec<CopyPlanId>,
}
#[derive(Debug, Serialize)]
pub struct Handler {
    pub node: p6::NodeId,
    pub capture: HandlerCapture,
    /// Exception arrivals only, after the original capture when present.
    pub receives: Vec<CopyPlanId>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerCapture {
    None,
    /// The explicit body retains the original capture statement. The handler
    /// label does not perform a second local binding on exceptional arrival.
    Body(StatementId),
    Catch {
        statement: StatementId,
        control: ControlId,
    },
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopSyntax {
    Infinite,
    While {
        terminal: crate::p2::BlockId,
        repeat: TransferId,
        exit: TransferId,
    },
    DoWhile {
        terminal: crate::p2::BlockId,
        repeat: TransferId,
        exit: TransferId,
    },
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Resume {
    Follow(LabelId),
    Break {
        control: ControlId,
        target: LabelId,
    },
    Continue {
        control: ControlId,
        target: LabelId,
    },
    ReturnVoid {
        terminal: crate::p2::BlockId,
    },
    Rethrow {
        terminal: crate::p2::BlockId,
        exception: ExprId,
        capture: OpId,
    },
}
#[derive(Debug, Serialize)]
pub struct SyncOutlet {
    pub entry: LabelId,
    pub release: StatementId,
    pub transfer: TransferId,
    pub destination: LabelId,
    pub resume: Resume,
    pub routing: Vec<p6::NodeId>,
    pub before_operand: Vec<CopyPlanId>,
    pub snapshots: Vec<CopyPlanId>,
    /// Original obligations, referenced by `completion`; do not emit twice.
    pub after_release: Vec<CopyPlanId>,
    pub completion: Vec<OutletStep>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutletStep {
    Copies(Vec<CopyPlanId>),
    /// Select the original link, then execute only this selected link's copies.
    Transfer {
        transfer: TransferId,
        copies: Vec<CopyPlanId>,
    },
    /// Before-terminal copies precede operand reads; snapshots follow those
    /// reads and precede the terminal event. `Resume` is not a second event.
    Terminal {
        terminal: crate::p2::BlockId,
        before_operand: Vec<CopyPlanId>,
        snapshots: Vec<CopyPlanId>,
    },
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    Block {
        node: p6::NodeId,
    },
    Sequence,
    If {
        terminal: crate::p2::BlockId,
        when_true: TransferId,
        when_false: TransferId,
    },
    /// Ordered short-circuit tests delivered by one P6 condition region. The
    /// children are the test runs, shared arm, optional other arm and optional
    /// routing child; each test keeps its terminal, links and evaluation order.
    Condition {
        tests: Vec<ConditionTest>,
        routing: Vec<TransferId>,
        else_arm: bool,
        other_exit: bool,
        explicit: ConditionSchedule,
    },
    Loop {
        syntax: LoopSyntax,
        continue_target: LabelId,
    },
    Switch {
        terminal: crate::p2::BlockId,
        arms: Vec<TransferId>,
        cases: Option<Vec<Vec<TransferId>>>,
    },
    LabelledBlock {
        target: LabelId,
    },
    Try {
        bindings: Vec<Option<CatchBinding>>,
        chain: p6::ChainId,
        clauses: std::ops::Range<u32>,
        handler_clauses: Vec<std::ops::Range<u32>>,
        handlers: Vec<p6::NodeId>,
        planning_scope: Option<p6::SyncId>,
        finally: Option<Finally>,
    },
    Synchronized {
        implicit: Option<ImplicitSync>,
        acquire: StatementId,
        lock: ExprId,
        enter: TransferId,
        held: Vec<p6::NodeId>,
        outlets: Vec<SyncOutlet>,
    },
    Graph {
        #[serde(serialize_with = "super::debug")]
        reason: p6::RecoveryReason,
        entries: Vec<LabelId>,
        exits: Vec<TransferId>,
    },
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Continuation {
    None,
    Parent,
    Next(LabelId),
    Goto(LabelId),
    Break(ControlId),
    Continue(ControlId),
}
/// One delivered predicate of a short-circuit condition. `select` is the
/// original edge to the shared arm, so it also fixes the printed polarity.
#[derive(Debug, Serialize)]
pub struct ConditionTest {
    pub predicate: p6::NodeId,
    pub select: TransferId,
    pub other: TransferId,
}
/// P6's explicit alternative, lowered without changing source child roles.
#[derive(Debug, Serialize)]
pub struct ConditionSchedule {
    pub children: Vec<ControlId>,
    pub completions: Vec<ConditionCompletion>,
    pub gotos: Vec<TransferId>,
}
#[derive(Debug, Serialize)]
pub struct ConditionCompletion {
    pub child: ControlId,
    pub continuation: Continuation,
}
/// P6's selected integer decision chain, retaining the original If tree.
#[derive(Debug, Serialize)]
pub struct Decision {
    pub arms: Vec<DecisionArm>,
    pub default: ControlId,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct DecisionArm {
    pub source: ControlId,
    pub header: ControlId,
    pub terminal: crate::p2::BlockId,
    pub predicate: p6::NodeId,
    pub selector: ExprId,
    pub key_expression: Option<ExprId>,
    pub key: i32,
    pub selected: TransferId,
    pub remaining: TransferId,
    pub body: ControlId,
}
#[derive(Debug, Serialize)]
pub struct Finally {
    pub capture: OpId,
    pub rethrows: Vec<crate::p2::BlockId>,
    pub terminals: Vec<(crate::p2::BlockId, crate::p2::BlockId)>,
    pub operations: Vec<(OpId, OpId)>,
    pub predicates: Vec<(crate::p2::BlockId, crate::p2::BlockId)>,
}
#[derive(Debug, Serialize)]
pub struct Control {
    pub decision: Option<Decision>,
    pub id: ControlId,
    pub kind: ControlKind,
    pub children: Vec<ControlId>,
    pub entry: Option<LabelId>,
    pub follow: Option<LabelId>,
    pub continuation: Continuation,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CatchBinding {
    pub operation: OpId,
    pub statement: StatementId,
    pub local: StorageId,
}
#[derive(Debug, Serialize)]
pub struct ImplicitSync {
    pub cleanups: Vec<(ControlId, u32)>,
    pub captures: Vec<OpId>,
    pub rethrows: Vec<crate::p2::BlockId>,
}
