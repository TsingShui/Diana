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
}
/// The exact printed carrier scope that shows an original `check-cast` instead
/// of its own statement. A check writes no register, so only a read of the
/// checked value can carry it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum CheckCarrier {
    /// A statement scope; `InlineCheck::occurrence` names its exact read.
    Statement(StatementId),
    /// One step of one original copy bundle. Only this step's right-hand side
    /// reads the value; the bundle's other steps and gap reports keep their own
    /// text at their own positions.
    Copy { bundle: CopyPlanId, step: u32 },
    /// An original terminal's operand: a return or throw value, a branch operand
    /// or a switch selector. `InlineCheck::occurrence` names its exact read.
    Terminal(BlockId),
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
    /// carrier: the statement's first evaluated read, or the copy step's
    /// right-hand side. P9 matches it exactly, so another read of the same
    /// storage never prints the check again.
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
#[derive(Clone, Copy, Debug, Serialize)]
pub enum ComposedResult {
    Value,
    Store(StorageId),
}
/// The predicate spelling selected by P7, not inferred by the renderer.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum ComposedPredicate {
    Value,
    Boolean,
    LongComparison {
        #[serde(serialize_with = "super::debug")]
        op: crate::p2::BranchOp,
        left: ExprId,
        right: ExprId,
    },
}
/// One original producer evaluated at a selected adjacent read occurrence.
/// Store retention preserves its other reads and its original materialization.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ComposedExpression {
    pub expression: ExprId,
    pub operation: OpId,
    pub statement: StatementId,
    pub pc: crate::CodeUnitOffset,
    pub result_pc: Option<crate::CodeUnitOffset>,
    pub site: Option<SiteId>,
    pub predicate: ComposedPredicate,
    pub result: ComposedResult,
    /// A cross-node composition closes the source control at this use.
    pub source_control: Option<ControlId>,
}
#[derive(Debug, Serialize)]
pub struct EmissionBlock {
    /// An unreferenced entry remains a zero-length source anchor.
    pub show_label: bool,
    pub actions: Vec<BodyAction>,
    pub end: BlockEnd,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct PredicateEmission {
    pub terminal: BlockId,
    /// Copies belong to selection of these original edges, never to a common
    /// loop continuation. A break does not execute the predicate-exit copies.
    pub when_true: TransferId,
    pub when_false: TransferId,
    pub negate: bool,
}
#[derive(Debug, Serialize)]
pub enum LoopPlacement {
    Infinite,
    While {
        predicate: ControlId,
        test: PredicateEmission,
        compact: bool,
    },
    DoWhile {
        predicate: ControlId,
        test: PredicateEmission,
        compact: bool,
    },
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
#[derive(Clone, Copy, Debug, Serialize)]
pub enum IfShape {
    IfElse,
    /// One arm ends in an established return/throw. The other edge's copies
    /// and complete body follow the guard in the enclosing lexical scope.
    Guard {
        inverted: bool,
    },
    ElseIf,
    /// The false edge and its original body become the sole printed arm. Any
    /// true-edge copy bundles have no steps or diagnostics, only source anchors.
    Inverted,
    /// Only the header remains; evaluating the predicate has no displayed work.
    Empty,
}
/// The last condition test's original other edge either reaches the common
/// follow, executes its boundary transfer, or enters its delivered body.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum ConditionElse {
    None,
    Copies,
    /// Execute the original final other edge's copies and boundary completion.
    Transfer,
    Body(ControlId),
}
#[derive(Debug, Serialize)]
pub enum EmissionLayout {
    Block(crate::p6::NodeId),
    Sequence,
    If {
        header: ControlId,
        yes: ControlId,
        no: ControlId,
        test: PredicateEmission,
        shape: IfShape,
    },
    /// Several original predicates print as one short-circuit condition. The
    /// shared arm prints on `select`; the optional other arm prints otherwise.
    Condition {
        header: ControlId,
        tests: Vec<PredicateEmission>,
        yes: ControlId,
        no: ConditionElse,
        fall: TransferId,
        /// P6-owned empty goto controls retain origins without printed routing.
        routing: Option<ControlId>,
    },
    /// A P6-owned alternative over the original pure integer tests.
    When {
        bodies: Vec<ControlId>,
    },
    /// The call statement owns the marker/normal-result protocol. The original
    /// dispatch and resume bodies remain in their existing scopes.
    Suspension {
        plan: u32,
    },
    Loop {
        body: ControlId,
        placement: LoopPlacement,
    },
    Switch {
        header: ControlId,
        bodies: Vec<ControlId>,
        cases: Option<Vec<Vec<TransferId>>>,
        terminal: BlockId,
    },
    Labelled,
    Try {
        body: ControlId,
        catches: Vec<ControlId>,
        finally: Option<FinallyEmission>,
    },
    /// Explicit planned exits prevent implicit second releases, including when
    /// an unlocked routing terminal remains within its original nested catch.
    Synchronized {
        implicit: bool,
        entry: ControlId,
        body: ControlId,
        acquire: OpId,
    },
    Graph,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct FinallyEmission {
    pub cleanup: ControlId,
    pub duplicate: ControlId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ControlCompletion {
    Goto(LabelId),
    Break(ControlId),
    Continue(ControlId),
    BreakHere(ControlId),
    ContinueHere(ControlId),
}

#[derive(Debug, Serialize)]
pub struct EmissionControl {
    pub label: String,
    /// Retain a labeled scope only when another emitted transfer names it.
    pub label_required: bool,
    /// Display-only omission; the original control remains in the product.
    pub omitted: bool,
    /// A labelled scope whose own braces print nothing but one unnamed printed
    /// child. P9 prints that child directly; the control, its labels and its
    /// transfers stay in the product.
    pub transparent: bool,
    pub continuation: Option<ControlCompletion>,
    /// Selected P6 explicit order; the retained control still owns its original
    /// tests/shared/other roles. Other layouts use the source child list.
    pub explicit_children: Option<Vec<ControlId>>,
    /// Original controls retained only as source attachments, not execution children.
    pub source_only: Vec<ControlId>,
    pub layout: EmissionLayout,
}
impl EmissionControl {
    pub fn children<'a>(&'a self, source: &'a Control) -> &'a [ControlId] {
        self.explicit_children
            .as_deref()
            .unwrap_or(&source.children)
    }
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
    pub(super) suspensions: Vec<Suspension>,
    pub(super) coroutine_body: Option<CoroutineBody>,
    pub(super) coroutine_loop: Option<CoroutineLoop>,
    pub(super) coroutine_flow: Option<CoroutineFlow>,
    pub(super) local_types: Vec<LocalSpelling>,
    pub(super) boolean_uses: Vec<bool>,
    pub(super) boolean_locals: Vec<bool>,
    /// A declaration attached to its existing assignment; absence keeps the header form.
    pub(super) local_initializers: Vec<Option<StatementId>>,
    pub(super) controls: Vec<EmissionControl>,
    pub(super) blocks: Vec<EmissionBlock>,
    pub(super) show_chains: Vec<bool>,
    /// Whether a route annotation still adds information beyond the printed
    /// chains and catch scopes.
    pub(super) site_annotations: Vec<bool>,
    /// A field reference prints its declared type only when the delivered
    /// references would otherwise be indistinguishable.
    pub(super) field_qualifications: Vec<bool>,
    pub(super) field_receiver_owners: Vec<Option<TypeUseId>>,
    /// A store prints as an ordinary assignment only when reordering its
    /// operands to Java's left-to-right order cannot be observed.
    pub(super) natural_stores: Vec<bool>,
    pub(super) inline_checks: Vec<InlineCheck>,
    /// Indices into `inline_checks`, by the statement that carries the check.
    pub(super) check_uses: Vec<Vec<u32>>,
    /// Indices into `inline_checks`, by the copy bundle that carries the check.
    pub(super) copy_check_uses: Vec<Vec<u32>>,
    /// Indices into `inline_checks`, by the terminal that carries the check.
    pub(super) terminal_check_uses: Vec<Vec<u32>>,
    /// Indices into `inline_checks`, by the statement whose stored value the
    /// check names.
    pub(super) definition_check_uses: Vec<Vec<u32>>,
    pub(super) composed_uses: Vec<Option<ComposedExpression>>,
    pub(super) composed_operations: Vec<Option<ComposedResult>>,
    /// By the statement whose allocation prints the payload.
    pub(super) array_literals: Vec<Option<ArrayLiteral>>,
    /// By the statement whose check prints nothing because P4 established it
    /// cannot add behavior at that position.
    pub(super) omitted_checks: Vec<Option<OmittedCheck>>,
    /// By the statement whose value prints only where it is used: every output
    /// materializes as a literal, so the statement itself states nothing. Its
    /// operation, PC, result commit and expression keep zero-length anchors.
    pub(super) omitted_literals: Vec<bool>,
    /// A `nop` operation states nothing a reader can execute: the plan keeps its
    /// statement, operation and PC as zero-length anchors instead of text.
    pub(super) omitted_nops: Vec<bool>,
    /// By the statement that allocates an object without constructing it yet.
    /// It prints the allocated type and its local where that allocation happens.
    pub(super) allocation_declarations: Vec<bool>,
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
    pub(super) statement_operations: Vec<Option<OpId>>,
    pub(super) origin_only_operations: Vec<Option<OriginOnly>>,
    pub(super) origin_only_terminals: Vec<bool>,
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
    pub fn origin_only_operation(&self, op: OpId) -> bool {
        match self
            .origin_only_operations
            .get(op.0 as usize)
            .copied()
            .flatten()
        {
            Some(OriginOnly::Attachment) => true,
            Some(OriginOnly::Identity(read)) => {
                !self.composed_operation(op) && self.composed_use(read).is_none()
            }
            None => false,
        }
    }
    pub fn origin_only_terminal(&self, block: BlockId) -> bool {
        self.origin_only_terminals
            .get(block.0 as usize)
            .copied()
            .unwrap_or(false)
    }
    pub fn kotlin_continuation_view(&self) -> bool {
        self.coroutine_body.is_some()
            || self.coroutine_loop.is_some()
            || self.coroutine_flow.is_some()
    }
    pub fn coroutine_body(&self) -> Option<&CoroutineBody> {
        self.coroutine_body.as_ref()
    }
    pub fn coroutine_loop(&self) -> Option<&CoroutineLoop> {
        self.coroutine_loop.as_ref()
    }
    pub fn coroutine_flow(&self) -> Option<&CoroutineFlow> {
        self.coroutine_flow.as_ref()
    }
    pub fn implicit_continuation(&self, expression: ExprId) -> Option<(u32, ExprId)> {
        self.suspension_call(expression)
            .map(|p| (p.continuation_argument, p.continuation))
            .or_else(|| {
                self.coroutine_loop
                    .as_ref()
                    .filter(|p| p.expression == expression)
                    .map(|p| (p.continuation_argument, p.continuation))
            })
            .or_else(|| {
                self.coroutine_flow
                    .as_ref()?
                    .sites
                    .iter()
                    .find(|p| p.expression == expression)
                    .map(|p| (p.continuation_argument, p.continuation))
            })
    }
    pub fn suspensions(&self) -> &[Suspension] {
        &self.suspensions
    }
    pub fn suspension(&self, op: OpId) -> Option<&Suspension> {
        self.suspensions.iter().find(|s| s.call == op)
    }
    pub fn suspension_call(&self, expression: ExprId) -> Option<&Suspension> {
        self.suspensions.iter().find(|s| s.expression == expression)
    }
    pub(super) fn empty() -> Self {
        Self {
            suspensions: Vec::new(),
            coroutine_body: None,
            coroutine_loop: None,
            coroutine_flow: None,
            local_types: Vec::new(),
            boolean_uses: Vec::new(),
            boolean_locals: Vec::new(),
            local_initializers: Vec::new(),
            controls: Vec::new(),
            blocks: Vec::new(),
            show_chains: Vec::new(),
            site_annotations: Vec::new(),
            field_qualifications: Vec::new(),
            field_receiver_owners: Vec::new(),
            natural_stores: Vec::new(),
            inline_checks: Vec::new(),
            check_uses: Vec::new(),
            copy_check_uses: Vec::new(),
            terminal_check_uses: Vec::new(),
            definition_check_uses: Vec::new(),
            composed_uses: Vec::new(),
            composed_operations: Vec::new(),
            array_literals: Vec::new(),
            omitted_checks: Vec::new(),
            omitted_literals: Vec::new(),
            omitted_nops: Vec::new(),
            allocation_declarations: Vec::new(),
            allocation_locals: Vec::new(),
            hidden_locals: Vec::new(),
            deferred_controls: Vec::new(),
            operations: Vec::new(),
            terminals: Vec::new(),
            ifs: Vec::new(),
            omitted_returns: Vec::new(),
            transfers: Vec::new(),
            initial: Vec::new(),
            statement_operations: Vec::new(),
            origin_only_operations: Vec::new(),
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
    pub fn controls(&self) -> &[EmissionControl] {
        &self.controls
    }
    pub fn omitted(&self, id: ControlId) -> bool {
        self.controls
            .get(id.0 as usize)
            .is_some_and(|control| control.omitted)
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
    /// `value`. A statement carrier also requires the expression occurrence P7
    /// selected, so another read of the same storage never prints the check.
    pub fn inline_check(
        &self,
        carrier: CheckCarrier,
        occurrence: ExprId,
        value: StorageId,
    ) -> Option<InlineCheck> {
        let uses = match carrier {
            CheckCarrier::Statement(statement) => self.check_uses.get(statement.0 as usize)?,
            CheckCarrier::Copy { bundle, .. } => self.copy_check_uses.get(bundle.0 as usize)?,
            CheckCarrier::Terminal(terminal) => {
                self.terminal_check_uses.get(terminal.0 as usize)?
            }
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
        self.natural_stores
            .get(id.0 as usize)
            .copied()
            .unwrap_or(false)
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
    pub fn composed_use(&self, id: ExprId) -> Option<ComposedExpression> {
        self.composed_uses.get(id.0 as usize).copied().flatten()
    }
    pub fn composed_operation(&self, id: OpId) -> bool {
        self.composed_operations
            .get(id.0 as usize)
            .is_some_and(Option::is_some)
    }
    pub(super) fn materializes_operation(&self, id: OpId) -> bool {
        !self.origin_only_operation(id)
            && !matches!(
                self.composed_operations.get(id.0 as usize),
                Some(Some(ComposedResult::Value))
            )
    }
    pub fn array_literal(&self, id: StatementId) -> Option<ArrayLiteral> {
        self.array_literals.get(id.0 as usize).copied().flatten()
    }
    /// The original check this statement's own display omits, if P4 established
    /// it cannot add behavior at this position.
    /// Whether this statement is a `nop` the plan prints without text.
    pub fn omitted_nop(&self, id: StatementId) -> bool {
        self.omitted_nops
            .get(id.0 as usize)
            .copied()
            .unwrap_or(false)
    }
    pub fn omitted_literal(&self, id: StatementId) -> bool {
        self.omitted_literals
            .get(id.0 as usize)
            .copied()
            .unwrap_or(false)
    }
    pub fn omitted_check(&self, id: StatementId) -> Option<&OmittedCheck> {
        self.omitted_checks.get(id.0 as usize)?.as_ref()
    }
    pub fn hidden_local(&self, id: StorageId) -> bool {
        self.hidden_locals
            .get(id.0 as usize)
            .copied()
            .unwrap_or(false)
    }
    pub fn allocation_declaration(&self, id: StatementId) -> bool {
        self.allocation_declarations
            .get(id.0 as usize)
            .copied()
            .unwrap_or(false)
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
    pub fn statement_operation(&self, id: StatementId) -> Option<OpId> {
        self.statement_operations
            .get(id.0 as usize)
            .copied()
            .flatten()
    }
}
