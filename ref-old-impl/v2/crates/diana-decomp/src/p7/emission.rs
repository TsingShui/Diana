//! Final syntax placement. No graph, type or copy solver runs here: every role,
//! target and storage location is already delivered by P5/P6 and owned by P7.
mod blocks;
mod build;
mod controls;
mod initialize;
mod phases;

pub(super) use build::construct;

use super::*;
use crate::p2::{BlockId, OpId};
use serde::Serialize;

#[derive(Default, Debug, Serialize)]
pub struct EvaluationPhases {
    pub before: Vec<CopyPlanId>,
    pub snapshots: Vec<CopyPlanId>,
    pub after: Vec<CopyPlanId>,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub enum BodyAction {
    /// An original protocol event, retained as an ordered barrier and printed
    /// as an ABI-only attachment rather than a second executable statement.
    AbiOperation {
        operation: OpId,
        statement: StatementId,
    },
    Statement {
        operation: OpId,
        statement: StatementId,
    },
    /// This syntax owns the original matched release. It closes only the named
    /// lock scope, not lexical try scopes; subsequent nodes stay at their origins.
    Release {
        operation: OpId,
        statement: StatementId,
        scope: ControlId,
    },
    /// Executed only on exceptional arrival, after capture when present.
    Receive { handler: u32 },
}
#[derive(Debug, Serialize)]
pub enum BlockEnd {
    None,
    Terminal(BlockId),
    Transfers(Vec<TransferId>),
    Annotated {
        terminal: BlockId,
        mode: SourceMode,
        normal: TransferId,
        suspended: TransferId,
    },
}
/// The exact printed carrier scope that shows an original `check-cast` instead
/// of its own statement. A check writes no register, so only a read of the
/// checked value can carry it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum CheckCarrier {
    /// One step of one original copy bundle. Only this step's right-hand side
    /// reads the value; the bundle's other steps and gap reports keep their own
    /// text at their own positions.
    Copy { bundle: CopyPlanId, step: u32 },
    /// The store that produced the checked value. The declaration names the
    /// check's type and its initializer prints the cast, so no read repeats it.
    Definition(StatementId),
}
/// A `check-cast` printed at the first use of its value instead of as its own
/// statement. The check and its operation keep their provenance there.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct InlineCheck {
    pub check: StatementId,
    pub operation: OpId,
    pub pc: crate::CodeUnitOffset,
    pub value: StorageId,
    pub reference: ReferenceId,
    pub carrier: CheckCarrier,
    /// The one printed expression occurrence that reads `value` for this
    /// carrier: the definition's checked read or the copy step's right-hand side.
    /// P9 matches it exactly; another read of the same storage stays plain.
    pub occurrence: ExprId,
}
/// A `check-cast` whose own display action cannot add behavior: P4 established
/// from pre-check facts that every reaching transfer already proves the exact
/// membership this check would resolve. The statement, operation, route and any
/// copy keep their original facts and positions; only its text disappears, and
/// its origins stay as zero-length anchors at that position.
#[derive(Clone, Debug, Serialize)]
pub struct OmittedCheck {
    pub check: StatementId,
    pub operation: OpId,
    pub pc: crate::CodeUnitOffset,
    pub value: StorageId,
    pub reference: ReferenceId,
    /// The original establishing events P4 relied on, for this product's reader.
    pub sources: Vec<crate::p4::CheckSource>,
}
/// A payload printed as the allocation's own array literal. The allocation, the
/// fill statement and the payload keep their original facts; only the spelling
/// and the omitted fill statement change.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ArrayLiteral {
    pub payload: PayloadId,
    /// The fill statement whose text the literal states; it prints nothing.
    pub fill: StatementId,
    pub operation: OpId,
    pub pc: crate::CodeUnitOffset,
}
/// Whether the composed producer replaces its only read or still commits its
/// original local before the surrounding expression consumes that value.
#[derive(Debug, Serialize)]
pub struct EmissionBlock {
    /// An unreferenced entry remains a zero-length source anchor.
    pub show_label: bool,
    pub actions: Vec<BodyAction>,
    pub end: BlockEnd,
}
/// The single printed form of one original branch terminal. The variants are
/// mutually exclusive, so no two display decisions can contradict each other.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum IfDisplay {
    /// `if (c) { yes } else { no }`
    Both,
    /// `if (c) { yes }`; the false arm prints nothing and keeps its anchors.
    NoElse,
    /// `if (!c) { no }`; the true arm prints nothing and keeps its anchors.
    Inverted,
    /// Nothing prints; both arms keep zero-length anchors.
    Omitted,
}
pub use crate::recovery::IfShape;
pub type PredicateEmission = crate::recovery::Predicate;
pub type LoopPlacement = crate::recovery::LoopPlacement;
pub type ConditionElse = crate::recovery::ConditionElse;
pub type EmissionLayout = crate::recovery::ControlLayout;
pub use crate::recovery::SourceMode;
pub type FinallyEmission = crate::recovery::Finally;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ControlCompletion {
    Goto(LabelId),
    Break(ControlId),
    Continue(ControlId),
    BreakHere(ControlId),
    ContinueHere(ControlId),
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum TransferSpelling {
    Goto(LabelId),
    Fallthrough,
    Break(ControlId),
    Continue(ControlId),
    /// The target is the innermost enclosing scope, so no label is named.
    BreakHere(ControlId),
    ContinueHere(ControlId),
    Selected,
}
#[derive(Debug, Serialize)]
pub struct EmissionTransfer {
    pub copies: Vec<CopyPlanId>,
    pub spelling: TransferSpelling,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub enum LocalSpelling {
    Named(TypeUseId),
    Machine(#[serde(serialize_with = "super::debug")] crate::p5::StorageType),
}
/// Sole execution layout consumed by P9. Source arenas and outlet-completion
/// records remain evidence, not additional schedules to emit alongside this one.
#[derive(Debug, Serialize)]
pub struct EmissionPlan {
    pub(super) protocol: Option<Protocol>,
    /// Unselected original containers retain identities without executing children.
    pub(super) control_origins: Vec<ControlId>,
    pub(super) local_types: Vec<LocalSpelling>,
    pub(super) boolean_uses: Vec<bool>,
    pub(super) boolean_locals: Vec<bool>,
    /// A declaration attached to its existing assignment; absence keeps the header form.
    pub(super) local_initializers: Vec<Option<StatementId>>,
    pub(super) blocks: Vec<EmissionBlock>,
    pub(super) show_chains: Vec<bool>,
    /// Whether a route annotation still adds information beyond the printed
    /// chains and catch scopes.
    pub(super) site_annotations: Vec<bool>,
    /// A field reference prints its declared type only when the delivered
    /// references would otherwise be indistinguishable.
    pub(super) field_qualifications: Vec<bool>,
    pub(super) field_receiver_owners: Vec<Option<TypeUseId>>,
    /// One structural placement record per original operation index. Every
    /// statement-keyed display decision lives here, so the plan keeps one index
    /// space instead of a parallel vector per decision.
    pub(super) placements: Vec<Placement>,
    pub(super) inline_checks: Vec<InlineCheck>,
    /// Indices into `inline_checks`, by the copy bundle that carries the check.
    pub(super) copy_check_uses: Vec<Vec<u32>>,
    /// Indices into `inline_checks`, by the statement whose stored value the
    /// check names.
    pub(super) definition_check_uses: Vec<Vec<u32>>,
    /// By the statement whose allocation prints the payload.
    /// By the statement whose check prints nothing because P4 established it
    /// cannot add behavior at that position.
    /// By the statement whose value prints only where it is used: every output
    /// materializes as a literal, so the statement itself states nothing. Its
    /// operation, PC, result commit and expression keep zero-length anchors.
    /// A `nop` operation states nothing a reader can execute: the plan keeps its
    /// statement, operation and PC as zero-length anchors instead of text.
    /// By the statement that allocates an object without constructing it yet.
    /// It prints the allocated type and its local where that allocation happens.
    /// By the local whose declaration is that allocation statement.
    pub(super) allocation_locals: Vec<bool>,
    pub(super) hidden_locals: Vec<bool>,
    pub(super) deferred_controls: Vec<bool>,
    pub(super) operations: Vec<EvaluationPhases>,
    pub(super) terminals: Vec<EvaluationPhases>,
    /// The one If display choice per original branch terminal.
    pub(super) ifs: Vec<IfDisplay>,
    /// Display-only omission of a method body's closing void return, whose
    /// closing brace performs the same exit.
    pub(super) omitted_returns: Vec<bool>,
    pub(super) transfers: Vec<EmissionTransfer>,
    pub(super) initial: Vec<CopyPlanId>,
    pub(super) origin_only_terminals: Vec<bool>,
}
/// Every statement-keyed display decision for one original operation.
#[derive(Clone, Debug, Default, Serialize)]
pub(super) struct Placement {
    /// A store prints as an ordinary assignment only when reordering its
    /// operands to Java's left-to-right order cannot be observed.
    pub(super) natural_store: bool,
    pub(super) array_literal: Option<ArrayLiteral>,
    pub(super) omitted_check: Option<OmittedCheck>,
    pub(super) omitted_literal: bool,
    pub(super) omitted_nop: bool,
    pub(super) allocation_declaration: bool,
    pub(super) origin_only: Option<OriginOnly>,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) enum OriginOnly {
    Attachment,
    /// A self-move is silent only while its operand is still a plain read.
    Identity(ExprId),
}
impl EmissionPlan {
    pub fn field_receiver_owner(&self, reference: ReferenceId) -> Option<TypeUseId> {
        self.field_receiver_owners
            .get(reference.0 as usize)
            .copied()
            .flatten()
    }
    /// The check a statement's stored value was given, when the declaration and
    /// its initializer print it instead of a later read.
    pub fn definition_check(&self, statement: StatementId) -> Option<InlineCheck> {
        let uses = self.definition_check_uses.get(statement.0 as usize)?;
        let index = *uses.first()?;
        self.inline_checks.get(index as usize).copied()
    }
    pub fn boolean_use(&self, expression: ExprId) -> bool {
        self.boolean_uses
            .get(expression.0 as usize)
            .copied()
            .unwrap_or(false)
    }
    pub fn boolean_local(&self, local: StorageId) -> bool {
        self.boolean_locals
            .get(local.0 as usize)
            .copied()
            .unwrap_or(false)
    }
    pub fn origin_only_terminal(&self, block: BlockId) -> bool {
        self.origin_only_terminals
            .get(block.0 as usize)
            .copied()
            .unwrap_or(false)
    }
    pub fn kotlin_continuation_view(&self) -> bool {
        self.protocol.is_some()
    }
    pub fn protocol(&self) -> Option<&Protocol> {
        self.protocol.as_ref()
    }
    pub fn implicit_continuation(&self, expression: ExprId) -> Option<(u32, ExprId)> {
        self.protocol
            .as_ref()?
            .calls
            .iter()
            .find(|p| p.expression == expression)
            .map(|p| (p.continuation_argument, p.continuation))
    }
    pub fn suspension(&self, op: OpId) -> Option<&Suspension> {
        self.protocol.as_ref()?.calls.iter().find(|p| p.call == op)
    }
    pub fn control_origins(&self) -> &[ControlId] {
        &self.control_origins
    }
    pub(super) fn empty() -> Self {
        Self {
            protocol: None,
            control_origins: Vec::new(),
            local_types: Vec::new(),
            boolean_uses: Vec::new(),
            boolean_locals: Vec::new(),
            local_initializers: Vec::new(),
            blocks: Vec::new(),
            show_chains: Vec::new(),
            site_annotations: Vec::new(),
            field_qualifications: Vec::new(),
            field_receiver_owners: Vec::new(),
            placements: Vec::new(),
            inline_checks: Vec::new(),
            copy_check_uses: Vec::new(),
            definition_check_uses: Vec::new(),
            allocation_locals: Vec::new(),
            hidden_locals: Vec::new(),
            deferred_controls: Vec::new(),
            operations: Vec::new(),
            terminals: Vec::new(),
            ifs: Vec::new(),
            omitted_returns: Vec::new(),
            transfers: Vec::new(),
            initial: Vec::new(),
            origin_only_terminals: Vec::new(),
        }
    }
    pub fn local_initializer(&self, local: StorageId) -> Option<StatementId> {
        self.local_initializers
            .get(local.0 as usize)
            .copied()
            .flatten()
    }
    pub fn local_types(&self) -> &[LocalSpelling] {
        &self.local_types
    }
    pub fn blocks(&self) -> &[EmissionBlock] {
        &self.blocks
    }
    pub fn show_chains(&self) -> &[bool] {
        &self.show_chains
    }
    /// Every check-cast this plan prints at a use instead of its own statement.
    pub fn inline_checks(&self) -> &[InlineCheck] {
        &self.inline_checks
    }
    /// The check-cast carried by this exact printed occurrence for a read of
    /// `value`. Another read of the same storage never prints the check.
    pub fn inline_check(
        &self,
        carrier: CheckCarrier,
        occurrence: ExprId,
        value: StorageId,
    ) -> Option<InlineCheck> {
        let uses = match carrier {
            CheckCarrier::Copy { bundle, .. } => self.copy_check_uses.get(bundle.0 as usize)?,
            CheckCarrier::Definition(statement) => {
                self.definition_check_uses.get(statement.0 as usize)?
            }
        };
        for &index in uses {
            let check = self.inline_checks.get(index as usize)?;
            if check.carrier == carrier && check.occurrence == occurrence && check.value == value {
                return Some(*check);
            }
        }
        None
    }
    pub fn natural_store(&self, id: StatementId) -> bool {
        self.placements
            .get(id.0 as usize)
            .is_some_and(|placement| placement.natural_store)
    }
    pub fn field_qualification(&self, id: ReferenceId) -> bool {
        self.field_qualifications
            .get(id.0 as usize)
            .copied()
            .unwrap_or(true)
    }
    pub fn site_annotation(&self, id: SiteId) -> bool {
        self.site_annotations
            .get(id.0 as usize)
            .copied()
            .unwrap_or(true)
    }
    pub fn array_literal(&self, id: StatementId) -> Option<ArrayLiteral> {
        self.placements.get(id.0 as usize)?.array_literal
    }
    /// The original check this statement's own display omits, if P4 established
    /// it cannot add behavior at this position.
    /// Whether this statement is a `nop` the plan prints without text.
    pub fn omitted_nop(&self, id: StatementId) -> bool {
        self.placements
            .get(id.0 as usize)
            .is_some_and(|placement| placement.omitted_nop)
    }
    pub fn omitted_literal(&self, id: StatementId) -> bool {
        self.placements
            .get(id.0 as usize)
            .is_some_and(|placement| placement.omitted_literal)
    }
    pub fn omitted_check(&self, id: StatementId) -> Option<&OmittedCheck> {
        self.placements.get(id.0 as usize)?.omitted_check.as_ref()
    }
    pub fn hidden_local(&self, id: StorageId) -> bool {
        self.hidden_locals
            .get(id.0 as usize)
            .copied()
            .unwrap_or(false)
    }
    pub fn allocation_declaration(&self, id: StatementId) -> bool {
        self.placements
            .get(id.0 as usize)
            .is_some_and(|placement| placement.allocation_declaration)
    }
    pub fn allocation_local(&self, id: StorageId) -> bool {
        self.allocation_locals
            .get(id.0 as usize)
            .copied()
            .unwrap_or(false)
    }
    pub fn deferred_control(&self, id: ControlId) -> bool {
        self.deferred_controls
            .get(id.0 as usize)
            .copied()
            .unwrap_or(false)
    }
    pub fn operations(&self) -> &[EvaluationPhases] {
        &self.operations
    }
    pub fn if_display(&self, id: BlockId) -> IfDisplay {
        self.ifs
            .get(id.0 as usize)
            .copied()
            .unwrap_or(IfDisplay::Both)
    }
    pub fn omitted_return(&self, id: BlockId) -> bool {
        self.omitted_returns
            .get(id.0 as usize)
            .copied()
            .unwrap_or(false)
    }

    pub fn terminals(&self) -> &[EvaluationPhases] {
        &self.terminals
    }
    pub fn transfers(&self) -> &[EmissionTransfer] {
        &self.transfers
    }
    pub fn initial(&self) -> &[CopyPlanId] {
        &self.initial
    }
}
