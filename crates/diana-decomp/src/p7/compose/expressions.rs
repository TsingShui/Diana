//! Compose a definition at an adjacent use in the established layout, keeping
//! its original store when an ordered Condition needs the value again.
//!
//! No control, storage or type solver runs here. A candidate that cannot retain
//! operand, commit, exception and copy order remains an ordinary statement.

use super::super::build::{bug, filled, push};
use super::super::*;
use super::access::{LocalAccess, LocalProtection};
use crate::p2::{BranchOp, CommitRule, Effects, OpId};
use crate::p3::{StateId, UseSite, ValueId};
use crate::p4::{Family, Initialization, Interpretation};
use crate::p5::{LocalId, LocalKind, Materialization, StorageRead, StorageType};

pub(super) fn construct(
    code: &mut MethodCode,
    key: crate::MethodKey,
    adjacency: &super::adjacency::Adjacency,
    access: &LocalAccess,
) -> Result<(), ExpressionError> {
    for group_index in 0..code.constructions.len() {
        let count = code.constructions[group_index].prepared_arguments.len();
        for argument_index in 0..count {
            let group = &code.constructions[group_index];
            let argument = &group.prepared_arguments[argument_index];
            let operand = argument.operand;
            let evaluation = Evaluation {
                expression: argument.expression,
                operation: argument.operation,
                predicate: EvaluationPredicate::Value,
                result: EvaluationResult::Value,
                site: None,
                source_control: None,
            };
            code.place_evaluation(operand, evaluation)?;
        }
    }
    if !access.complete {
        return Ok(());
    }

    let mut last: Vec<Option<OpId>> = filled(code.terminals.len(), None)?;
    let incoming = &adjacency.incoming;
    let sibling = &adjacency.sibling;
    for operation in &code.operations {
        *last
            .get_mut(operation.original_block.0 as usize)
            .ok_or(bug("operation names an unknown original block"))? = Some(operation.source);
    }
    for node in 0..code.emission.blocks.len() {
        if let Some((group, use_id, control)) =
            construction_throw(&Cover::new(code), node, &last, incoming, sibling, access)
        {
            let construction = &code.constructions[group.0 as usize];
            let evaluation = Evaluation {
                expression: construction.expression,
                operation: construction.initialization,
                predicate: EvaluationPredicate::Value,
                result: EvaluationResult::Value,
                site: None,
                source_control: Some(control),
            };
            code.place_evaluation(use_id, evaluation)?;
            code.emission.deferred_controls[control.0 as usize] = true;
        }
        let candidate = terminal_use(&Cover::new(code), node, &last, incoming, sibling, access);
        if let Some((producer, use_id, control)) = candidate {
            install(code, producer, use_id, Some(control), key)?;
        }
    }
    // Terminal placement supplies roots; the same ordered cover then prepares
    // all ordinary operands along established copy-free sibling portions.
    ordinary_runs(code, key, adjacency, access)
}

fn construction_throw(
    code: &Cover<'_>,
    node: usize,
    last: &[Option<OpId>],
    incoming: &[u8],
    sibling: &[Option<ControlId>],
    access: &LocalAccess,
) -> Option<(ConstructionId, ExprId, ControlId)> {
    let block = code.emission.blocks.get(node)?;
    let BodyAction::Statement {
        operation,
        statement,
    } = *block.actions.last()?
    else {
        return None;
    };
    let StatementKind::Construction(id) = code.statements.get(statement.0 as usize)?.kind else {
        return None;
    };
    let group = code.constructions.get(id.0 as usize)?;
    let source = code.operations.get(operation.0 as usize)?;
    if group.initialization != operation
        || group.statement != statement
        || last
            .get(source.original_block.0 as usize)
            .copied()
            .flatten()
            != Some(operation)
        || group.prelude.is_some()
        || !group.snapshot_members.is_empty()
        || group.publications.len() != 1
    {
        return None;
    }
    let phases = code.emission.operations.get(operation.0 as usize)?;
    if !phases.before.is_empty() || !phases.snapshots.is_empty() || !phases.after.is_empty() {
        return None;
    }
    let construction_statement = code.statements.get(statement.0 as usize)?;
    for &member in &group.members {
        let operation = code.operations.get(member.0 as usize)?;
        if operation.effects.may_throw()
            && (operation.exceptional.is_none()
                || !construction_statement.sites.iter().any(|site| {
                    code.sites.get(site.0 as usize).is_some_and(|site| {
                        site.original == crate::p5::ThrowSite::Operation(member)
                    })
                }))
        {
            return None;
        }
    }
    for &site in &construction_statement.sites {
        let site = code.sites.get(site.0 as usize)?;
        if matches!(site.syntax, ExceptionSyntax::ImpossibleRelease)
            || !site.source_snapshots.is_empty()
        {
            return None;
        }
    }
    let publication = &group.publications[0];
    let local = publication.destination;
    let [definition] = construction_statement.outputs.as_slice() else {
        return None;
    };
    if !matches!(definition.materialization(), Materialization::Deferred { group: owner, local: destination } if owner.0 == id.0 && destination.0 == local.0)
    {
        return None;
    }
    let declaration = code.locals.get(local.0 as usize)?;
    if declaration.gap.is_some()
        || !matches!(declaration.original.kind(), LocalKind::AllocationCarrier { value, operation } if value == publication.original.value && operation == group.allocation)
        || access.reads.get(local.0 as usize).copied()? != 1
    {
        return None;
    }
    for copy in &code.copies {
        for assignment in &copy.assignments {
            if assignment.issue().is_some()
                || assignment.destination().is_err()
                || assignment.source().is_err()
                || assignment
                    .destination()
                    .ok()
                    .is_some_and(|destination| destination.0 == local.0)
                || assignment
                    .source()
                    .ok()
                    .and_then(|source| source.local())
                    .is_some_and(|source| source.0 == local.0)
            {
                return None;
            }
        }
        for step in &copy.steps {
            match *step {
                CopyStatement::Save {
                    destination,
                    source,
                } if destination == local || source == local => return None,
                CopyStatement::Assign {
                    destination,
                    original,
                    ..
                } if destination == local
                    || original.local().is_some_and(|source| source.0 == local.0) =>
                {
                    return None;
                }
                _ => {}
            }
        }
    }
    let BlockEnd::Transfers(links) = &block.end else {
        return None;
    };
    let [link] = links.as_slice() else {
        return None;
    };
    let transfer = code.transfers.get(link.0 as usize)?;
    let emitted = code.emission.transfers.get(link.0 as usize)?;
    if transfer.edge.is_some()
        || !matches!(emitted.spelling, TransferSpelling::Fallthrough)
        || !emitted.copies.is_empty()
    {
        return None;
    }
    let next_node = transfer.destination.0 as usize;
    let next = code.emission.blocks.get(next_node)?;
    if next.show_label
        || !next.actions.is_empty()
        || incoming.get(next_node).copied()? != 1
        || code.blocks.get(next_node)?.original_block != source.original_block
    {
        return None;
    }
    let owner = code.labels.get(node)?.owner;
    let control = code.controls.get(owner.0 as usize)?;
    if !matches!(control.layout, EmissionLayout::Block(n) if n.0 as usize == node)
        || control.continuation.is_some()
        || code
            .emission
            .deferred_controls
            .get(owner.0 as usize)
            .copied()?
    {
        return None;
    }
    let following = sibling.get(owner.0 as usize).copied().flatten()?;
    let following_control = code.controls.get(following.0 as usize)?;
    if !matches!(following_control.layout, EmissionLayout::Block(target) if target.0 as usize == next_node)
        || following_control.continuation.is_some()
    {
        return None;
    }
    let BlockEnd::Terminal(terminal) = next.end else {
        return None;
    };
    let target = code.terminals.get(terminal.0 as usize)?;
    if !matches!(target.emission, TerminalEmission::Block) {
        return None;
    }
    let TerminalKind::Throw(use_id) = target.kind else {
        return None;
    };
    let phases = code.emission.terminals.get(terminal.0 as usize)?;
    if !phases.before.is_empty() || !phases.snapshots.is_empty() || !phases.after.is_empty() {
        return None;
    }
    let site = code.sites.get(target.site?.0 as usize)?;
    if matches!(site.syntax, ExceptionSyntax::ImpossibleRelease)
        || !site.source_snapshots.is_empty()
    {
        return None;
    }
    if construction_statement.sites.iter().any(|id| {
        let Some(member) = code.sites.get(id.0 as usize) else {
            return true;
        };
        member.chain != site.chain
            || std::mem::discriminant(&member.syntax) != std::mem::discriminant(&site.syntax)
    }) {
        return None;
    }
    let expression = code.expressions.get(use_id.0 as usize)?;
    let ExprKind::Read {
        storage,
        view: Interpretation::Direct(Family::Reference),
    } = expression.kind
    else {
        return None;
    };
    let origin = expression.origin.as_ref()?;
    if storage != local
        || origin.initialization != Some(Initialization::Initialized)
        || origin.site
            != (UseSite::Terminal {
                block: terminal,
                argument: 0,
            })
        || !matches!(origin.read, Ok(StorageRead::Value { local: source, value, .. }) if source.0 == local.0 && value == publication.original.value)
        || code.evaluation(use_id).is_some()
    {
        return None;
    }
    Some((id, use_id, owner))
}

#[derive(Clone, Copy)]
struct Producer {
    operation: OpId,
    statement: StatementId,
    expression: ExprId,
    local: StorageId,
    destination: LocalId,
    value: ValueId,
    state: StateId,
    family: Family,
    boolean: bool,
    site: Option<SiteId>,
    result: EvaluationResult,
}

#[derive(Clone, Copy)]
enum UsePolicy {
    SingleRead,
    PreserveStore,
}

fn producer(
    code: &Cover<'_>,
    operation: OpId,
    statement: StatementId,
    access: &LocalAccess,
    policy: UsePolicy,
) -> Result<Producer, &'static str> {
    producer_with_nested(code, operation, statement, access, policy, false)
}

fn producer_with_nested(
    code: &Cover<'_>,
    operation: OpId,
    statement: StatementId,
    access: &LocalAccess,
    policy: UsePolicy,
    allow_nested_arguments: bool,
) -> Result<Producer, &'static str> {
    let source = code
        .operations
        .get(operation.0 as usize)
        .ok_or("producer_operation_unavailable")?;
    if code.emission.suspension(operation).is_some()
        || code
            .emission
            .coroutine_flow()
            .is_some_and(|p| p.sites.iter().any(|p| p.call == operation))
        || code
            .emission
            .coroutine_loop()
            .is_some_and(|p| p.call == operation)
    {
        return Err("suspension_owns_its_result_and_route");
    }
    if code.nested_operation(operation)
        || matches!(
            code.emission
                .origin_only_operations
                .get(operation.0 as usize),
            Some(Some(super::super::emission::OriginOnly::Attachment))
        )
        || !matches!(source.emission, Emission::Block { .. })
    {
        return Err("producer_not_a_separate_block_operation");
    }
    let phase = code
        .emission
        .operations
        .get(operation.0 as usize)
        .ok_or("producer_phase_unavailable")?;
    if !phase.before.is_empty() || !phase.snapshots.is_empty() || !phase.after.is_empty() {
        return Err("producer_has_copy_or_snapshot_phase");
    }
    let statement_ref = code
        .statements
        .get(statement.0 as usize)
        .ok_or("producer_statement_unavailable")?;
    let (expression, moving) = match statement_ref.kind {
        StatementKind::Evaluate { expression } => (expression, false),
        StatementKind::Move { expression, .. } => (expression, true),
        _ => return Err("producer_statement_kind"),
    };
    if statement_ref.outputs.len() != 1 {
        return Err("producer_not_one_definition");
    }
    let definition = &statement_ref.outputs[0];
    let Materialization::Store(destination) = definition.materialization() else {
        return Err("producer_not_a_local_store");
    };
    let expr = code
        .expressions
        .get(expression.0 as usize)
        .ok_or("producer_expression_unavailable")?;
    // Calls commit at their delivered move-result; adjacency below keeps that
    // commit before the branch, and install retains its result_pc origin.
    let commit = if matches!(expr.kind, ExprKind::Call(_)) {
        CommitRule::AfterResult
    } else {
        CommitRule::AtOperation
    };
    if definition.commit() != commit {
        return Err("producer_commit_position");
    }
    let local = StorageId(destination.0);
    let declared = code
        .locals
        .get(local.0 as usize)
        .ok_or("producer_local_unavailable")?;
    if declared.gap.is_some() {
        return Err("producer_local_has_gap");
    }
    let value = definition.value().ok_or("producer_value_unavailable")?;
    let value_access = access
        .value(value)
        .ok_or("producer_value_uses_unavailable")?;
    let result = match value_access.protection {
        LocalProtection::Pinned => return Err("producer_local_pinned"),
        LocalProtection::CopySource if matches!(policy, UsePolicy::PreserveStore) => {
            EvaluationResult::Store(local)
        }
        LocalProtection::CopySource => return Err("producer_copy_source_requires_store"),
        LocalProtection::None => match value_access.reads {
            1 => EvaluationResult::Value,
            count if count > 1 && matches!(policy, UsePolicy::PreserveStore) => {
                EvaluationResult::Store(local)
            }
            _ => return Err("producer_not_single_read"),
        },
    };
    let StorageType::Family { family, .. } = declared.original.storage_type() else {
        return Err("producer_storage_family_unavailable");
    };
    let Arguments::Known(inputs) = &statement_ref.inputs else {
        return Err("producer_operands_unavailable");
    };
    if !inputs.iter().all(|&operand| {
        leaf(code, operand) || (allow_nested_arguments && nested_argument(code, operand, operation))
    }) {
        return Err("producer_expression_or_nonleaf_operands");
    }
    let boolean = match &expr.kind {
        ExprKind::Read { storage, view } if moving => {
            let origin = expr
                .origin
                .as_ref()
                .ok_or("move_source_origin_unavailable")?;
            if !compatible(*view, family)
                || !initialized(origin.initialization, family)
                || origin.site
                    != (UseSite::Operation {
                        operation,
                        argument: 0,
                    })
                || !matches!(origin.read, Ok(StorageRead::Value { local, view: read_view, .. })
                    if local.0 == storage.0 && read_view == *view)
            {
                return Err("move_source_view_or_initialization");
            }
            boolean_type(code, declared, family)
                && code
                    .locals
                    .get(storage.0 as usize)
                    .is_some_and(|source| boolean_type(code, source, family))
        }
        ExprKind::Unary { .. } if !moving && family != Family::Reference => false,
        ExprKind::Binary { .. } if family != Family::Reference => false,
        ExprKind::String(reference)
            if family == Family::Reference && matches!(result, EvaluationResult::Value) =>
        {
            let reference = code
                .references
                .get(reference.0 as usize)
                .ok_or("string_reference_unavailable")?;
            if !matches!(
                reference.declaration,
                Ok(ReferenceDeclaration::String { .. })
            ) || reference.use_issue.is_some()
            {
                return Err("string_reference_unavailable");
            }
            false
        }
        ExprKind::Compare {
            op: crate::p2::CompareOp::CmpLong,
            left,
            right,
        } if family == Family::Integral
            && matches!(result, EvaluationResult::Value)
            && long_leaf(code, *left)
            && long_leaf(code, *right) =>
        {
            false
        }
        // A field read keeps its original receiver evaluation and, when it may
        // throw, its original exception position. The consumer checks below
        // decide whether that position is preserved.
        ExprKind::Field { .. } => false,
        ExprKind::InstanceOf { reference, .. } if family == Family::Integral => {
            let reference = code
                .references
                .get(reference.0 as usize)
                .ok_or("instance_of_reference_unavailable")?;
            if !matches!(reference.declaration, Ok(ReferenceDeclaration::Type { .. }))
                || reference.use_issue.is_some()
            {
                return Err("instance_of_type_unavailable");
            }
            true
        }
        ExprKind::Call(_) if !moving => {
            // Boolean shorthand still requires complete, nonempty `Z` sources;
            // an `I` call keeps its numeric comparison, and any other family
            // keeps its own interpretation. The ordered condition producer
            // decides whether that numeric value may compose as a test operand.
            boolean_type(code, declared, family)
        }
        _ => return Err("producer_expression_or_nonleaf_operands"),
    };
    if boolean && !moving {
        // The adjacent terminal is the first consumer. The original check or
        // call can throw, so only an uncaught route without saved state is eligible.
        if !matches!(
            source.original,
            crate::p2::OpKind::InstanceOf { .. }
                | crate::p2::OpKind::Invoke { .. }
                | crate::p2::OpKind::UnresolvedInvoke { .. }
        ) || !source.effects.may_throw()
            || source.exceptional.is_none()
            || statement_ref.sites.len() != 1
        {
            return Err("boolean_producer_effect_or_site");
        }
        let site = code
            .sites
            .get(statement_ref.sites[0].0 as usize)
            .ok_or("producer_site_unavailable")?;
        if !matches!(site.syntax, ExceptionSyntax::Propagate) || !site.source_snapshots.is_empty() {
            return Err("producer_exception_route_or_snapshot");
        }
    } else if source.effects != Effects::default()
        || source.exceptional.is_some()
        || !statement_ref.sites.is_empty()
    {
        // An effectful producer stays at the same execution position: one
        // delivered site with nothing saved, so the original throw route and
        // evaluation count stay unchanged.
        source
            .exceptional
            .ok_or("producer_exception_completion_unavailable")?;
        let [site] = statement_ref.sites.as_slice() else {
            return Err("producer_not_one_exception_site");
        };
        let site = code
            .sites
            .get(site.0 as usize)
            .ok_or("producer_site_unavailable")?;
        if !matches!(site.syntax, ExceptionSyntax::Propagate) || !site.source_snapshots.is_empty() {
            return Err("producer_exception_route_or_snapshot");
        }
    }
    Ok(Producer {
        operation,
        statement,
        expression,
        local,
        destination,
        value,
        state: definition.state(),
        family,
        boolean,
        site: statement_ref.sites.first().copied(),
        result,
    })
}

fn nested_argument(code: &Cover<'_>, argument: ExprId, consumer: OpId) -> bool {
    code.evaluation(argument).is_some_and(|composed| {
        matches!(composed.result, EvaluationResult::Value)
            && composed.operation.0 < consumer.0
            && composed.source_control.is_none()
    })
}

fn long_leaf(code: &Cover<'_>, id: ExprId) -> bool {
    leaf(code, id)
        && matches!(
            code.expressions[id.0 as usize].kind,
            ExprKind::Read {
                view: Interpretation::Direct(Family::Long),
                ..
            } | ExprKind::Literal {
                view: Interpretation::Literal {
                    family: Family::Long,
                    ..
                },
                ..
            } | ExprKind::Literal {
                view: Interpretation::Direct(Family::Long),
                ..
            }
        )
}

fn terminal_use(
    code: &Cover<'_>,
    node: usize,
    last: &[Option<OpId>],
    incoming: &[u8],
    sibling: &[Option<ControlId>],
    access: &LocalAccess,
) -> Option<(Producer, ExprId, ControlId)> {
    let block = code.emission.blocks.get(node)?;
    let BodyAction::Statement {
        operation,
        statement,
    } = *block.actions.last()?
    else {
        return None;
    };
    let source = code.operations.get(operation.0 as usize)?;
    if last
        .get(source.original_block.0 as usize)
        .copied()
        .flatten()?
        != operation
    {
        return None;
    }
    let producer = producer(code, operation, statement, access, UsePolicy::SingleRead).ok()?;
    let BlockEnd::Transfers(links) = &block.end else {
        return None;
    };
    let [link] = links.as_slice() else {
        return None;
    };
    let transfer = code.transfers.get(link.0 as usize)?;
    let emitted = code.emission.transfers.get(link.0 as usize)?;
    if transfer.edge.is_some()
        || !matches!(emitted.spelling, TransferSpelling::Fallthrough)
        || !emitted.copies.is_empty()
    {
        return None;
    }
    let next_node = transfer.destination.0 as usize;
    let next = code.emission.blocks.get(next_node)?;
    if next.show_label
        || !next.actions.is_empty()
        || incoming.get(next_node).copied()? != 1
        || code.blocks.get(next_node)?.original_block != source.original_block
    {
        return None;
    }
    let owner = code.labels.get(node)?.owner;
    let source_control = code.controls.get(owner.0 as usize)?;
    if !matches!(source_control.layout, EmissionLayout::Block(n) if n.0 as usize == node)
        || source_control.continuation.is_some()
    {
        return None;
    }
    let following = sibling.get(owner.0 as usize).copied().flatten()?;
    let following_control = code.controls.get(following.0 as usize)?;
    // The producer's only normal delivery reaches this empty, single-entry
    // header in the same original block. The selected layout must print that
    // header immediately after its source; no operation or copy moves across it.
    let terminal = match following_control.layout {
        EmissionLayout::If { header, test, .. } => {
            let header = code.controls.get(header.0 as usize)?;
            if !matches!(header.layout, EmissionLayout::Block(target) if target.0 as usize == next_node)
                || header.continuation.is_some()
                || !matches!(next.end, BlockEnd::None)
            {
                return None;
            }
            let target = code.terminals.get(test.terminal.0 as usize)?;
            if !matches!(target.emission, TerminalEmission::If(control) if control == following) {
                return None;
            }
            test.terminal
        }
        EmissionLayout::Block(target) if target.0 as usize == next_node => {
            match code.blocks.get(next_node)?.completion {
                BlockCompletion::Syntax {
                    terminal,
                    owner: TerminalEmission::Loop(control),
                } => {
                    let EmissionLayout::Loop { placement, .. } =
                        &code.controls.get(control.0 as usize)?.layout
                    else {
                        return None;
                    };
                    let test = match placement {
                        LoopPlacement::While { test, .. } | LoopPlacement::DoWhile { test, .. } => {
                            test
                        }
                        LoopPlacement::Infinite => return None,
                    };
                    if test.terminal != terminal
                        || following_control.continuation.is_some()
                        || !matches!(next.end, BlockEnd::None)
                    {
                        return None;
                    }
                    terminal
                }
                _ => {
                    let BlockEnd::Terminal(terminal) = next.end else {
                        return None;
                    };
                    if !matches!(
                        code.terminals.get(terminal.0 as usize)?.emission,
                        TerminalEmission::Block
                    ) {
                        return None;
                    }
                    terminal
                }
            }
        }
        _ => return None,
    };
    let target = code.terminals.get(terminal.0 as usize)?;
    let use_id = match &target.kind {
        TerminalKind::If {
            condition,
            when_true,
            when_false,
        } => {
            let yes = code.transfers.get(when_true.0 as usize)?;
            let no = code.transfers.get(when_false.0 as usize)?;
            if yes.destination == no.destination
                && code
                    .emission
                    .transfers
                    .get(when_true.0 as usize)?
                    .copies
                    .is_empty()
                && code
                    .emission
                    .transfers
                    .get(when_false.0 as usize)?
                    .copies
                    .is_empty()
            {
                // A predicate with no distinct delivery can disappear. Keep
                // its producer as a statement instead of creating an empty If.
                return None;
            }
            if !supported_role(code, terminal, producer, 0) {
                return None;
            }
            match condition {
                Condition::Pair { left, .. } => *left,
                Condition::Zero { value, .. } => *value,
            }
        }
        TerminalKind::Return(Some(value))
            if !matches!(
                code.expressions[producer.expression.0 as usize].kind,
                ExprKind::Compare { .. }
            ) =>
        {
            *value
        }
        _ => return None,
    };
    let target = code.terminals.get(terminal.0 as usize)?;
    let phases = code.emission.terminals.get(terminal.0 as usize)?;
    if !phases.before.is_empty()
        || !phases.snapshots.is_empty()
        || !phases.after.is_empty()
        || target.site.is_some()
    {
        return None;
    }
    matches_use(
        code,
        producer,
        use_id,
        UseSite::Terminal {
            block: terminal,
            argument: 0,
        },
    )
    .then_some((producer, use_id, owner))
}

fn boolean_type(code: &Cover<'_>, local: &LocalDeclaration, family: Family) -> bool {
    family == Family::Integral
        && !local.type_source_unknown
        && !local.declared_types.is_empty()
        && local.declared_types.iter().all(|id| {
            code.types
                .get(id.0 as usize)
                .is_some_and(|ty| ty.units == [u16::from(b'Z')])
        })
}

fn compatible(view: Interpretation, family: Family) -> bool {
    matches!(view, Interpretation::Direct(actual) if actual == family)
        || matches!(view, Interpretation::IntegralView { .. } if family == Family::Integral)
}

fn initialized(initialization: Option<Initialization>, family: Family) -> bool {
    if family == Family::Reference {
        matches!(initialization, Some(Initialization::Initialized))
    } else {
        initialization.is_none()
    }
}

fn matches_use(code: &Cover<'_>, producer: Producer, id: ExprId, site: UseSite) -> bool {
    if code.evaluation(id).is_some() {
        return false;
    }
    let Some(expression) = code.expressions.get(id.0 as usize) else {
        return false;
    };
    let ExprKind::Read { storage, view } = expression.kind else {
        return false;
    };
    let Some(origin) = &expression.origin else {
        return false;
    };
    storage == producer.local
        && compatible(view, producer.family)
        && initialized(origin.initialization, producer.family)
        && origin.site == site
        && matches!(origin.read, Ok(StorageRead::Value { local, value, state, .. })
            if local == producer.destination && value == producer.value && state == producer.state)
}

fn prepare_evaluation(
    code: &Cover<'_>,
    producer: Producer,
    use_id: ExprId,
    source_control: Option<ControlId>,
) -> Result<Evaluation, ExpressionError> {
    let predicate = if let ExprKind::Compare {
        op: crate::p2::CompareOp::CmpLong,
        left,
        right,
    } = code.expressions[producer.expression.0 as usize].kind
    {
        let Some(UseSite::Terminal { block, .. }) = code.expressions[use_id.0 as usize]
            .origin
            .as_ref()
            .map(|origin| origin.site)
        else {
            return Err(bug("composed cmp-long lost its terminal use"));
        };
        let TerminalKind::If {
            condition:
                Condition::Zero {
                    op,
                    meaning: ZeroMeaning::IntegralZero,
                    ..
                },
            ..
        } = code.terminals[block.0 as usize].kind
        else {
            return Err(bug("composed cmp-long lost its zero branch"));
        };
        EvaluationPredicate::LongComparison { op, left, right }
    } else if producer.boolean {
        EvaluationPredicate::Boolean
    } else {
        EvaluationPredicate::Value
    };
    Ok(Evaluation {
        expression: producer.expression,
        operation: producer.operation,
        predicate,
        result: producer.result,
        site: producer.site,
        source_control,
    })
}

fn install(
    code: &mut MethodCode,
    producer: Producer,
    use_id: ExprId,
    source_control: Option<ControlId>,
    key: crate::MethodKey,
) -> Result<(), ExpressionError> {
    let evaluation = prepare_evaluation(&Cover::new(code), producer, use_id, source_control)?;
    install_evaluation(code, use_id, evaluation, key)
}

fn install_evaluation(
    code: &mut MethodCode,
    use_id: ExprId,
    evaluation: Evaluation,
    key: crate::MethodKey,
) -> Result<(), ExpressionError> {
    if let Some(control) = evaluation.source_control {
        let deferred = code
            .emission
            .deferred_controls
            .get(control.0 as usize)
            .ok_or(bug("composed operation lost its control"))?;
        if *deferred {
            return Err(bug("control has multiple deferred compositions"));
        }
    }
    code.place_evaluation(use_id, evaluation)?;
    if let Some(control) = evaluation.source_control {
        code.emission.deferred_controls[control.0 as usize] = true;
    }
    let kind = match code.expressions[evaluation.expression.0 as usize].kind {
        ExprKind::String(_) => Some("string"),
        ExprKind::Compare {
            op: crate::p2::CompareOp::CmpLong,
            ..
        } => Some("cmp_long"),
        _ => None,
    };
    if let Some(kind) = kind {
        super::trace::expression_selected(
            key,
            kind,
            evaluation.operation,
            code.operations[evaluation.operation.0 as usize]
                .statement
                .ok_or(bug("evaluation lost its statement"))?,
            use_id,
        );
    }
    Ok(())
}

fn leaf(code: &Cover<'_>, id: ExprId) -> bool {
    code.evaluation(id).is_none()
        && matches!(
            code.expressions.get(id.0 as usize).map(|v| &v.kind),
            Some(ExprKind::Read { .. } | ExprKind::Literal { .. })
        )
}

/// Composes every delivered short-circuit test at its original evaluation
/// position. The P6 plan already fixes the test order, both original links and
/// the shared arm; this pass only decides whether each test's scheduled prelude
/// can be stated as the condition operand. A test whose prelude cannot keep its
/// facts prints the established explicit schedule instead.
pub(super) fn conditions(
    code: &mut MethodCode,
    key: crate::MethodKey,
    access: &LocalAccess,
) -> Result<(), ExpressionError> {
    let mut layouts = Vec::new();
    for index in 0..code.controls.len() {
        if matches!(
            code.controls[index].layout,
            EmissionLayout::Condition { .. }
        ) {
            push(&mut layouts, index)?;
        }
    }
    for index in layouts {
        let choice = if access.complete {
            compose(code, index, access, key)?
        } else {
            ConditionChoice::Explicit {
                test: None,
                statement: None,
                why: "copy_dependency_unavailable",
            }
        };
        let control = code.controls[index].id;
        match choice {
            ConditionChoice::Compact { values, stores } => {
                super::trace::condition_selected(key, control, values, stores);
            }
            ConditionChoice::Explicit {
                test,
                statement,
                why,
            } => {
                explicit(code, index)?;
                super::trace::condition_refused(key, control, test, statement, why);
            }
        }
    }
    Ok(())
}

enum ConditionChoice {
    Compact {
        values: usize,
        stores: usize,
    },
    Explicit {
        test: Option<usize>,
        statement: Option<StatementId>,
        why: &'static str,
    },
}

/// Sparse proposed execution ownership over the immutable current product.
struct Cover<'a> {
    code: &'a MethodCode,
    work: Vec<(ExprId, Evaluation)>,
}
impl<'a> Cover<'a> {
    fn new(code: &'a MethodCode) -> Self {
        Self {
            code,
            work: Vec::new(),
        }
    }
    fn evaluation(&self, id: ExprId) -> Option<Evaluation> {
        self.work
            .iter()
            .find(|(operand, _)| *operand == id)
            .map(|(_, evaluation)| *evaluation)
            .or_else(|| self.code.evaluation(id))
    }
    fn nested_operation(&self, operation: OpId) -> bool {
        self.code.nested_operation(operation)
            || self
                .work
                .iter()
                .any(|(_, evaluation)| evaluation.operation == operation)
    }
    fn propose(&mut self, producer: Producer, operand: ExprId) -> Result<(), ExpressionError> {
        if self.evaluation(operand).is_some() || self.nested_operation(producer.operation) {
            return Err(bug("cover has multiple execution owners"));
        }
        let evaluation = prepare_evaluation(self, producer, operand, None)?;
        push(&mut self.work, (operand, evaluation))
    }
}
impl std::ops::Deref for Cover<'_> {
    type Target = MethodCode;
    fn deref(&self) -> &Self::Target {
        self.code
    }
}

struct Unscheduled {
    statement: Option<StatementId>,
    why: &'static str,
}

/// One test's prelude states its operands only as an ordered suffix of the run's
/// statements. The suffix order must equal the terminal's operand order, because
/// P9 evaluates a pair's left operand first. A later run prints no header of its
/// own, so every statement in it must participate in that suffix.
fn compose(
    code: &mut MethodCode,
    index: usize,
    access: &LocalAccess,
    key: crate::MethodKey,
) -> Result<ConditionChoice, ExpressionError> {
    let count = match &code.controls[index].layout {
        EmissionLayout::Condition { tests, .. } => tests.len(),
        _ => return Err(bug("condition composition lost its layout")),
    };
    let composed = (|| -> Result<(ConditionChoice, Vec<(ExprId, Evaluation)>), ExpressionError> {
        let mut cover = Cover::new(code);
        let code = &mut cover;
        for position in 0..count {
            let refuse = |why, statement| {
                Ok((
                    ConditionChoice::Explicit {
                        test: Some(position),
                        statement,
                        why,
                    },
                    Vec::new(),
                ))
            };
            let test = {
                let EmissionLayout::Condition { tests, .. } = &code.controls[index].layout else {
                    return Err(bug("condition composition changed its layout"));
                };
                tests[position]
            };
            let Some(&control) = code.controls[index].source_children.get(position) else {
                return refuse("test_control_unavailable", None);
            };
            let Some(nodes) = run_nodes(code, control)? else {
                return refuse("test_run_not_flat_or_has_continuation", None);
            };
            let Some(predicate) = nodes.last().copied() else {
                return refuse("test_run_empty", None);
            };
            if code.blocks[predicate.0 as usize].original_block != test.terminal
                || !matches!(
                    code.emission.blocks[predicate.0 as usize].end,
                    BlockEnd::None
                )
                || {
                    let transfers = &code.blocks[predicate.0 as usize].transfers;
                    transfers.len() != 2
                        || !transfers.contains(&test.when_true)
                        || !transfers.contains(&test.when_false)
                }
            {
                return refuse("predicate_portion_or_original_edges", None);
            }
            let TerminalKind::If { condition, .. } = &code.terminals[test.terminal.0 as usize].kind
            else {
                return refuse("predicate_not_if", None);
            };
            let phases = &code.emission.terminals[test.terminal.0 as usize];
            if !phases.before.is_empty() || !phases.snapshots.is_empty() || !phases.after.is_empty()
            {
                return refuse("predicate_has_copy_or_snapshot_phase", None);
            }
            if code.terminals[test.terminal.0 as usize].site.is_some() {
                return refuse("predicate_has_exception_site", None);
            }
            let (select, other) = if test.negate {
                (test.when_false, test.when_true)
            } else {
                (test.when_true, test.when_false)
            };
            if !code.emission.transfers[select.0 as usize].copies.is_empty() {
                return refuse("selected_edge_has_copy_bundle", None);
            }
            if !code.emission.transfers[other.0 as usize].copies.is_empty() {
                if position + 1 != count {
                    return refuse("intermediate_other_edge_has_copy_bundle", None);
                }
                if !original_edge_copies(code, other)? {
                    return refuse("final_other_copy_not_original_edge", None);
                }
            }
            let mut operands = Vec::new();
            match condition {
                Condition::Pair { left, right, .. } => {
                    push(&mut operands, *left)?;
                    push(&mut operands, *right)?;
                }
                Condition::Zero { value, .. } => push(&mut operands, *value)?,
            }
            compose_condition_arguments(code, &nodes, access)?;
            let scheduled =
                scheduled(code, &nodes, test.terminal, &operands, access, position > 0)?;
            match scheduled {
                Ok(scheduled) => {
                    for (producer, operand) in scheduled {
                        code.propose(producer, operand)?;
                    }
                }
                Err(reason) => return refuse(reason.why, reason.statement),
            }
        }
        Ok((
            ConditionChoice::Compact {
                values: 0,
                stores: 0,
            },
            cover.work,
        ))
    })();
    let (choice, work) = composed?;
    if matches!(choice, ConditionChoice::Explicit { .. }) {
        return Ok(choice);
    }
    // Install only after every test admits its whole schedule. A refused later
    // run must not leave a hidden local or relocated store in the explicit form.
    let ControlKind::Condition { routing, .. } = &code.controls[index].kind else {
        return Err(bug("condition composition lost its routing product"));
    };
    let routing = super::super::build::copy(routing)?;
    for link in routing {
        code.emission.transfers[link.0 as usize].spelling = TransferSpelling::Fallthrough;
        let node = code.transfers[link.0 as usize].from;
        let owner = code.labels[node.0 as usize].owner;
        code.controls[owner.0 as usize].continuation = None;
    }
    if let EmissionLayout::Condition {
        routing: Some(control),
        ..
    } = code.controls[index].layout
    {
        code.controls[control.0 as usize].continuation = None;
    }
    let mut values = 0;
    let mut stores = 0;
    for (operand, evaluation) in work {
        install_evaluation(code, operand, evaluation, key)?;
        match evaluation.result {
            EvaluationResult::Value => values += 1,
            EvaluationResult::Store(storage) => {
                stores += 1;
                super::trace::condition_store(
                    key,
                    code.controls[index].id,
                    evaluation.operation,
                    code.operations[evaluation.operation.0 as usize]
                        .statement
                        .ok_or(bug("condition evaluation lost its statement"))?,
                    operand,
                    storage,
                );
            }
        }
    }
    let copy_else = match &code.controls[index].layout {
        EmissionLayout::Condition {
            no: ConditionElse::None,
            fall,
            ..
        } => !super::branches::copies_print_nothing(code, *fall),
        _ => false,
    };
    let control_id = code.controls[index].id;
    if let EmissionLayout::Condition {
        tests, no, fall, ..
    } = &mut code.controls[index].layout
    {
        let other_spelling = code.emission.transfers[fall.0 as usize].spelling;
        for test in tests.iter() {
            code.emission.transfers[test.when_true.0 as usize].spelling =
                TransferSpelling::Fallthrough;
            code.emission.transfers[test.when_false.0 as usize].spelling =
                TransferSpelling::Fallthrough;
        }
        if matches!(no, ConditionElse::Transfer) {
            // P6 distinguishes a context exit from the common follow even when
            // neither owns a child body. Compaction removes intermediate test
            // routing, never this final edge's original boundary completion.
            code.emission.transfers[fall.0 as usize].spelling = match other_spelling {
                TransferSpelling::Selected => {
                    TransferSpelling::Goto(code.transfers[fall.0 as usize].destination)
                }
                spelling => spelling,
            };
        }
        if copy_else {
            *no = ConditionElse::Copies;
            for &copy in &code.emission.transfers[fall.0 as usize].copies {
                super::trace::condition_copy_else(
                    key,
                    control_id,
                    *fall,
                    copy,
                    code.copies[copy.0 as usize].steps.len(),
                );
            }
        }
    }
    Ok(ConditionChoice::Compact { values, stores })
}

/// The final other edge is selected only after every test declined the shared
/// arm. Its copies can print in that exact else position, never in the common
/// continuation or on another test's selected edge.
fn original_edge_copies(code: &Cover<'_>, edge: TransferId) -> Result<bool, ExpressionError> {
    let transfer = code
        .transfers
        .get(edge.0 as usize)
        .ok_or(bug("condition lost its final other edge"))?;
    let Some(original) = transfer.edge else {
        return Ok(false);
    };
    if transfer.construction.is_some() {
        return Ok(false);
    }
    for &copy in &code.emission.transfers[edge.0 as usize].copies {
        let bundle = code
            .copies
            .get(copy.0 as usize)
            .ok_or(bug("condition edge names an unknown copy bundle"))?;
        if !matches!(bundle.emission, CopyEmission::Original)
            || !matches!(bundle.location, crate::p5::CopyLocation::Transfer {
                site: crate::p5::TransferSite::NormalEdge(source), ..
            } if source == original)
            || bundle
                .assignments
                .iter()
                .any(|assignment| assignment.issue().is_some())
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// A producer composes into a branch only under the established branch-role
/// policy. A boolean value is a zero test with an integral-zero meaning; it is
/// not a pair operand or a different comparison operator.
fn supported_role(
    code: &Cover<'_>,
    terminal: crate::p2::BlockId,
    producer: Producer,
    argument: u32,
) -> bool {
    if matches!(
        code.expressions[producer.expression.0 as usize].kind,
        ExprKind::Compare { .. }
    ) {
        return argument == 0
            && matches!(producer.result, EvaluationResult::Value)
            && matches!(
                code.terminals[terminal.0 as usize].kind,
                TerminalKind::If {
                    condition: Condition::Zero {
                        op: BranchOp::Eqz
                            | BranchOp::Nez
                            | BranchOp::Ltz
                            | BranchOp::Gez
                            | BranchOp::Gtz
                            | BranchOp::Lez,
                        meaning: ZeroMeaning::IntegralZero,
                        ..
                    },
                    ..
                }
            );
    }
    if !producer.boolean {
        return true;
    }
    argument == 0
        && matches!(
            code.terminals[terminal.0 as usize].kind,
            TerminalKind::If {
                condition: Condition::Zero {
                    op: BranchOp::Eqz | BranchOp::Nez,
                    meaning: ZeroMeaning::IntegralZero,
                    ..
                },
                ..
            }
        )
}

/// The block nodes of one straight test run, in original order.
fn run_nodes(
    code: &Cover<'_>,
    control: ControlId,
) -> Result<Option<Vec<crate::p6::NodeId>>, ExpressionError> {
    let Some(plan) = code.controls.get(control.0 as usize) else {
        return Ok(None);
    };
    if plan.continuation.is_some() {
        return Ok(None);
    }
    let mut out = Vec::new();
    match plan.layout {
        EmissionLayout::Block(node) => push(&mut out, node)?,
        EmissionLayout::Sequence => {
            for &child in plan.children() {
                let Some(child) = code.controls.get(child.0 as usize) else {
                    return Ok(None);
                };
                let EmissionLayout::Block(node) = child.layout else {
                    return Ok(None);
                };
                if child.continuation.is_some() {
                    return Ok(None);
                }
                push(&mut out, node)?;
            }
        }
        _ => return Ok(None),
    }
    Ok(Some(out))
}

/// One exact consumer view, with operand order already established by P7.
enum CoverTarget {
    Operation(OpId, StatementId),
    Terminal(crate::p2::BlockId, bool),
}
struct Consumer<'a> {
    target: CoverTarget,
    operands: &'a [ExprId],
}
impl Consumer<'_> {
    fn site(&self, argument: u32) -> UseSite {
        match self.target {
            CoverTarget::Operation(operation, _) => UseSite::Operation {
                operation,
                argument,
            },
            CoverTarget::Terminal(block, _) => UseSite::Terminal { block, argument },
        }
    }
    fn required(&self) -> bool {
        matches!(self.target, CoverTarget::Terminal(_, true))
    }
    fn store(&self, code: &Cover<'_>) -> bool {
        matches!(self.target, CoverTarget::Operation(_, statement)
            if matches!(code.statements[statement.0 as usize].kind,
                StatementKind::FieldStore { .. } | StatementKind::ArrayStore { .. }))
    }
    fn permits(
        &self,
        code: &Cover<'_>,
        producer: Producer,
        operand: ExprId,
        argument: u32,
    ) -> bool {
        if let CoverTarget::Terminal(block, _) = self.target {
            return supported_role(code, block, producer, argument);
        }
        if producer.boolean
            || matches!(
                code.expressions[producer.expression.0 as usize].kind,
                ExprKind::Compare { .. }
            )
        {
            return false;
        }
        let string = matches!(
            code.expressions[producer.expression.0 as usize].kind,
            ExprKind::String(_)
        );
        let CoverTarget::Operation(_, statement) = self.target else {
            return false;
        };
        let statement = &code.statements[statement.0 as usize];
        if let StatementKind::Evaluate { expression } = statement.kind
            && let ExprKind::Call(call) = &code.expressions[expression.0 as usize].kind
            && let Some(ConcatOutcome::Inferred(plan)) = call.concatenation()
        {
            return string && plan.permits_string_composition(operand);
        }
        if self.store(code) {
            let source = &code.operations[producer.operation.0 as usize];
            return (string
                || (source.effects == Effects::default()
                    && source.exceptional.is_none()
                    && code.statements[producer.statement.0 as usize]
                        .sites
                        .is_empty()))
                && self
                    .operands
                    .iter()
                    .take(argument as usize)
                    .all(|&operand| leaf(code, operand));
        }
        true
    }
}

/// Match one source suffix jointly. Roots keep their original order; a nested
/// root already owns its earlier preparation. No unmatched effect is crossed.
fn cover_suffix(
    code: &Cover<'_>,
    actions: &[(OpId, StatementId)],
    consumer: Consumer<'_>,
    access: &LocalAccess,
    ignore_checks: bool,
) -> Result<Result<Vec<(Producer, ExprId)>, Unscheduled>, ExpressionError> {
    let mut out = Vec::new();
    let mut last = None;
    for &(operation, statement) in actions.iter().rev() {
        if code.nested_operation(operation)
            || (ignore_checks
                && matches!(
                    code.statements[statement.0 as usize].kind,
                    StatementKind::CheckCast { .. }
                ))
        {
            continue;
        }
        let policy = if consumer.required() {
            UsePolicy::PreserveStore
        } else {
            UsePolicy::SingleRead
        };
        let mut why = "producer_not_exact_consumer_read";
        let mut matched = None;
        match producer_with_nested(
            code,
            operation,
            statement,
            access,
            policy,
            !consumer.store(code),
        ) {
            Ok(producer) => {
                for (argument, &operand) in consumer.operands.iter().enumerate() {
                    let argument =
                        u32::try_from(argument).map_err(|_| bug("consumer argument overflow"))?;
                    if !matches_use(code, producer, operand, consumer.site(argument)) {
                        continue;
                    }
                    if !consumer.permits(code, producer, operand, argument) {
                        why = "producer_consumer_role";
                        continue;
                    }
                    if matched.is_some() {
                        return Ok(Err(Unscheduled {
                            statement: Some(statement),
                            why: "producer_matches_multiple_operands",
                        }));
                    }
                    matched = Some((producer, operand, argument));
                }
            }
            Err(reason) => why = reason,
        }
        let ordered =
            matched.is_some_and(|(_, _, argument)| last.is_none_or(|previous| argument < previous));
        if !ordered {
            if consumer.required()
                || (matched.is_some() && matches!(consumer.target, CoverTarget::Terminal(..)))
            {
                return Ok(Err(Unscheduled {
                    statement: Some(statement),
                    why: if matched.is_some() {
                        "producer_order_differs_from_operands"
                    } else {
                        why
                    },
                }));
            }
            break;
        }
        let (producer, operand, argument) =
            matched.ok_or(bug("ordered suffix lost its producer"))?;
        last = Some(argument);
        push(&mut out, (producer, operand))?;
    }
    Ok(Ok(out))
}

fn cover_operations(
    code: &mut Cover<'_>,
    actions: &[(OpId, StatementId)],
    access: &LocalAccess,
    ignore_checks: bool,
) -> Result<(), ExpressionError> {
    for index in 1..actions.len() {
        let (operation, statement) = actions[index];
        let source = &code.statements[statement.0 as usize];
        if !matches!(
            source.kind,
            StatementKind::Move { .. }
                | StatementKind::Evaluate { .. }
                | StatementKind::FieldStore { .. }
                | StatementKind::ArrayStore { .. }
        ) || !code.emission.operations[operation.0 as usize]
            .before
            .is_empty()
        {
            continue;
        }
        let Arguments::Known(operands) = &source.inputs else {
            continue;
        };
        let consumer = Consumer {
            target: CoverTarget::Operation(operation, statement),
            operands,
        };
        if let Ok(work) = cover_suffix(code, &actions[..index], consumer, access, ignore_checks)? {
            for (producer, operand) in work {
                code.propose(producer, operand)?;
            }
        }
    }
    Ok(())
}

fn run_actions(
    code: &Cover<'_>,
    nodes: &[crate::p6::NodeId],
) -> Result<Option<Vec<(OpId, StatementId)>>, ExpressionError> {
    let mut actions = Vec::new();
    for &node in nodes {
        for &action in &code.emission.blocks[node.0 as usize].actions {
            let BodyAction::Statement {
                operation,
                statement,
            } = action
            else {
                return Ok(None);
            };
            push(&mut actions, (operation, statement))?;
        }
    }
    Ok(Some(actions))
}

fn compose_condition_arguments(
    code: &mut Cover<'_>,
    nodes: &[crate::p6::NodeId],
    access: &LocalAccess,
) -> Result<(), ExpressionError> {
    let Some(actions) = run_actions(code, nodes)? else {
        return Ok(());
    };
    let mut access = access.copy()?;
    for &(_, statement) in &actions {
        if let StatementKind::CheckCast { value, .. } = code.statements[statement.0 as usize].kind
            && let Some((value, _)) = super::access::read_of(code, value)
        {
            access.ignore_read(value);
        }
    }
    cover_operations(code, &actions, &access, true)
}

fn scheduled(
    code: &Cover<'_>,
    nodes: &[crate::p6::NodeId],
    terminal: crate::p2::BlockId,
    operands: &[ExprId],
    access: &LocalAccess,
    skipped: bool,
) -> Result<Result<Vec<(Producer, ExprId)>, Unscheduled>, ExpressionError> {
    let Some(actions) = run_actions(code, nodes)? else {
        return Ok(Err(Unscheduled {
            statement: None,
            why: "test_run_has_nonstatement_action",
        }));
    };
    if !straight(code, nodes, skipped)? {
        return Ok(Err(Unscheduled {
            statement: None,
            why: "test_run_not_straight_or_has_copy_bundle",
        }));
    }
    cover_suffix(
        code,
        &actions,
        Consumer {
            target: CoverTarget::Terminal(terminal, skipped),
            operands,
        },
        access,
        true,
    )
}

/// Only the already delivered copy-free sibling adjacency joins source portions.
fn ordinary_runs(
    code: &mut MethodCode,
    key: crate::MethodKey,
    adjacency: &super::adjacency::Adjacency,
    access: &LocalAccess,
) -> Result<(), ExpressionError> {
    let mut starts = filled(code.blocks.len(), true)?;
    for node in 0..code.blocks.len() {
        if let Some(next) = adjacency.next_node(node) {
            starts[next] = false;
        }
    }
    for (start, first) in starts.into_iter().enumerate() {
        if !first {
            continue;
        }
        let mut cover = Cover::new(code);
        let mut actions = Vec::new();
        let mut current = Some(start);
        while let Some(node) = current {
            for action in &code.emission.blocks[node].actions {
                if let BodyAction::Statement {
                    operation,
                    statement,
                } = *action
                {
                    push(&mut actions, (operation, statement))?;
                } else {
                    cover_operations(&mut cover, &actions, access, false)?;
                    actions.clear();
                }
            }
            current = adjacency.next_node(node);
        }
        cover_operations(&mut cover, &actions, access, false)?;
        let work = cover.work;
        for (operand, mut evaluation) in work.iter().copied() {
            let source = &code.operations[evaluation.operation.0 as usize];
            let owner = code.labels[source.node.0 as usize].owner;
            let carrier_node = code.expressions[operand.0 as usize]
                .origin
                .as_ref()
                .and_then(|origin| match origin.site {
                    UseSite::Operation { operation, .. } => {
                        Some(code.operations[operation.0 as usize].node)
                    }
                    UseSite::Terminal { .. } => None,
                });
            if carrier_node != Some(source.node)
                && !code.emission.deferred_control(owner)
                && !work.iter().any(|(_, other)| {
                    other.operation.0 > source.source.0
                        && code.operations[other.operation.0 as usize].node == source.node
                })
            {
                evaluation.source_control = Some(owner);
            }
            install_evaluation(code, operand, evaluation, key)?;
        }
    }
    Ok(())
}

/// A test run is one straight fallthrough chain: every node except the last
/// transfers only to the next listed node, and the last node owns the predicate
/// terminal. A skipped run may not carry copies whose steps would disappear.
fn straight(
    code: &Cover<'_>,
    nodes: &[crate::p6::NodeId],
    skipped: bool,
) -> Result<bool, ExpressionError> {
    if nodes.is_empty() {
        return Ok(false);
    }
    for window in nodes.windows(2) {
        let BlockEnd::Transfers(links) = &code.emission.blocks[window[0].0 as usize].end else {
            return Ok(false);
        };
        let [link] = links.as_slice() else {
            return Ok(false);
        };
        let Some(transfer) = code.transfers.get(link.0 as usize) else {
            return Ok(false);
        };
        if code.blocks[window[0].0 as usize].transfers.as_slice() != [*link]
            || code.labels[transfer.destination.0 as usize].node != window[1]
        {
            return Ok(false);
        }
        if skipped && !code.emission.transfers[link.0 as usize].copies.is_empty() {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Selects P6's complete explicit alternative. Predicate portions, child order
/// and child completions were established before P6 publication; this consumer
/// neither walks subtrees nor changes retained source roles or continuations.
fn explicit(code: &mut MethodCode, index: usize) -> Result<(), ExpressionError> {
    let ControlKind::Condition {
        tests, explicit, ..
    } = &code.controls[index].kind
    else {
        return Err(bug("explicit condition lost its delivered plan"));
    };
    for test in tests {
        let block = code
            .blocks
            .get(test.predicate.0 as usize)
            .ok_or(bug("explicit condition lost its predicate portion"))?;
        code.emission.blocks[test.predicate.0 as usize].end =
            BlockEnd::Terminal(block.original_block);
    }
    for &link in &explicit.gotos {
        let transfer = code
            .transfers
            .get(link.0 as usize)
            .ok_or(bug("explicit condition lost an original branch edge"))?;
        code.emission.transfers[link.0 as usize].spelling =
            TransferSpelling::Goto(transfer.destination);
    }
    let children = super::super::build::copy(&explicit.children)?;
    let mut completions = Vec::new();
    for completion in &explicit.completions {
        push(
            &mut completions,
            (completion.child, completion.continuation.completion()),
        )?;
    }
    for (child, completion) in completions {
        code.controls[child.0 as usize].continuation = completion;
    }
    code.controls[index].children = children.into();
    code.controls[index].layout = EmissionLayout::Sequence;
    Ok(())
}
