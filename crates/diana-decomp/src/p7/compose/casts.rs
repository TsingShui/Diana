//! Composes an original `check-cast` into the existing display of its value's
//! first read, or omits it when P4 established it cannot fail there.
//!
//! Nothing is reordered and nothing is created: the check statement, its
//! operation, its exception site and the checked local all keep their original
//! facts, and the read that prints the check keeps its own expression. Only the
//! check's separate display action is dropped, so the surviving text still names
//! one original check.
//!
//! Declaration initialization and copy steps retain their own locations. Other
//! checks follow one cursor to the first available read in the supplied run.
//! Adjacency can cross whole-operation portions, but never an event, phase,
//! selection or protected scope. Each carrier names its original read and check.
//!
//! A check whose pre-check disposition P4 published as redundant never composes:
//! the cast states nothing there, so its own statement is omitted and the
//! surviving carrier prints its original text unchanged.
//!
//! Anything else keeps the standalone check statement: a phase, a route
//! annotation, a value write, a selection, a Save or a gap report between the
//! check and the read would move the check past an event it now precedes.

use super::super::build::{bug, copy, push};
use super::super::walk::{Event, Roots, Scope, Walk, occurrences};
use super::super::*;
use super::access::{LocalAccess, read_of};
use crate::p2::{Effects, OpId};
use crate::p4::CheckDisposition;
use crate::p5::{CopyLocation, Materialization, TransferSite};

/// Shared read-only state one carrier decision may consult. The driver owns the
/// iteration, the "already claimed" bookkeeping and every written slot.
struct Carrier<'a> {
    code: &'a MethodCode,
    access: &'a LocalAccess,
    adjacency: &'a super::adjacency::Adjacency,
    protection: &'a [Option<ControlId>],
    found: &'a [(StatementId, InlineCheck)],
    /// Whether a suspension surface owns that statement's operands.
    suspended: &'a [bool],
    /// Where the driver found this check in the printed order.
    node: usize,
    index: usize,
    operation: OpId,
}
/// What one carrier decided for one check statement.
enum Carried {
    /// The common operation cover owns this exact operand occurrence.
    Operand(ExprId),
    /// A copy step keeps its original location-specific carrier.
    At((StatementId, InlineCheck)),
    /// The declaration takes the check's type; the driver applies it.
    Declare {
        check: (StatementId, InlineCheck),
        local: StorageId,
        ty: TypeUseId,
    },
}
/// Why one carrier did not answer for this check, or an error it hit while
/// reading the products. The driver records the first refusal and propagates an
/// error.
enum Stop {
    Refuse(&'static str),
    Error(ExpressionError),
}
impl From<ExpressionError> for Stop {
    fn from(error: ExpressionError) -> Self {
        Stop::Error(error)
    }
}
fn why_at(slots: &mut [Option<&'static str>], statement: StatementId, why: &'static str) {
    if let Some(slot) = slots.get_mut(statement.0 as usize)
        && slot.is_none()
    {
        *slot = Some(why);
    }
}

pub(super) fn prepare(
    code: &mut MethodCode,
    types: &crate::p4::TypeFacts,
) -> Result<(), ExpressionError> {
    code.emission.omitted_checks = redundant(code, types)?;
    Ok(())
}

pub(super) fn construct(
    code: &mut MethodCode,
    key: crate::MethodKey,
    adjacency: &super::adjacency::Adjacency,
    access: &LocalAccess,
) -> Result<(), ExpressionError> {
    let mut why_slots: Vec<Option<&'static str>> =
        super::super::build::filled(code.operations.len(), None)?;
    let mut found: Vec<(StatementId, InlineCheck)> = Vec::new();
    let protection = protected_contexts(code)?;
    let mut suspended: Vec<bool> = super::super::build::filled(code.operations.len(), false)?;
    if let Some(protocol) = code.emission.protocol() {
        for site in &protocol.calls {
            suspended[site.statement.0 as usize] = true;
        }
    }
    let mut claimed: Vec<bool> = super::super::build::filled(code.operations.len(), false)?;
    // Each check consumes one owning declaration/copy location or one ordered
    // operand query; normal reads use the shared operation cover.
    for node in 0..code.emission.blocks.len() {
        for index in 0..code.emission.blocks[node].actions.len() {
            let BodyAction::Statement {
                operation,
                statement,
            } = code.emission.blocks[node].actions[index]
            else {
                continue;
            };
            if claimed[statement.0 as usize] || code.emission.omitted_check(statement).is_some() {
                continue;
            }
            if code.nested_operation(operation) {
                continue;
            }
            let Some((storage, _)) = eligible_check(code, operation, statement, None) else {
                continue;
            };
            let carrier = Carrier {
                code,
                access,
                adjacency,
                protection: &protection,
                found: &found,
                suspended: &suspended,
                node,
                index,
                operation,
            };
            let mut selected = None;
            for find in [definition_carrier, copy_carrier] {
                match find(&carrier, statement, storage) {
                    Ok(Some(choice)) => {
                        selected = Some(choice);
                        break;
                    }
                    Ok(None) => {}
                    Err(Stop::Refuse(why)) => why_at(&mut why_slots, statement, why),
                    Err(Stop::Error(error)) => return Err(error),
                }
            }
            if selected.is_none() {
                match forward_operand(&carrier, statement, storage) {
                    Ok(Some(choice)) => selected = Some(choice),
                    Ok(None) => {}
                    Err(Stop::Refuse(why)) => why_at(&mut why_slots, statement, why),
                    Err(Stop::Error(error)) => return Err(error),
                }
            }
            match selected {
                Some(Carried::Operand(occurrence)) => {
                    if super::expressions::place_check(code, operation, occurrence, access, key)? {
                        claimed[statement.0 as usize] = true;
                    } else {
                        why_at(
                            &mut why_slots,
                            statement,
                            "check_binding_cannot_cover_operand",
                        );
                    }
                }
                Some(Carried::At(check)) => {
                    claimed[statement.0 as usize] = true;
                    push(&mut found, check)?;
                }
                Some(Carried::Declare { check, local, ty }) => {
                    code.emission.local_types[local.0 as usize] = LocalSpelling::Named(ty);
                    code.locals[local.0 as usize].declared_types = vec![ty];
                    claimed[statement.0 as usize] = true;
                    push(&mut found, check)?;
                }
                None => {}
            }
        }
    }
    // A redundant check has no text to carry, so its value's read keeps its own
    // spelling and the check's statement only contributes its origins.
    let mut kept: Vec<(StatementId, InlineCheck)> = Vec::new();
    for (statement, check) in found {
        if code.emission.omitted_check(statement).is_some() {
            continue;
        }
        if !carries_check(code, access, statement, check.value, check.occurrence) {
            why_at(&mut why_slots, statement, "carrier_value_state_changed");
            continue;
        }
        push(&mut kept, (statement, check))?;
    }
    let found = kept;
    if !found.is_empty() {
        for (_, check) in &found {
            let index = u32::try_from(code.emission.inline_checks.len())
                .map_err(|_| crate::RequestError::PlatformCapacity)?;
            push(&mut code.emission.inline_checks, *check)?;
            let uses = match check.carrier {
                CheckCarrier::Copy { bundle, .. } => {
                    &mut code.emission.copy_check_uses[bundle.0 as usize]
                }
                CheckCarrier::Definition(statement) => {
                    &mut code.emission.definition_check_uses[statement.0 as usize]
                }
            };
            push(uses, index)?;
        }
        // The check statement prints nothing of its own; its provenance moves to
        // the use. Removing the collected statements keeps every other action in
        // place.
        for block in &mut code.emission.blocks {
            block.actions.retain(|action| match action {
                BodyAction::Statement { statement, .. } | BodyAction::Release { statement, .. } => {
                    !found.iter().any(|(check, _)| check == statement)
                }
                BodyAction::Receive { .. } | BodyAction::AbiOperation { .. } => true,
            });
        }
    }
    // One line per check that keeps its own statement, with the first condition
    // that stopped it. A check fused by any carrier is not reported.
    if super::trace::ENABLED {
        for (index, why) in why_slots.iter().enumerate() {
            let Some(why) = why else { continue };
            let statement = StatementId(u32::try_from(index).unwrap_or(u32::MAX));
            if found.iter().any(|(check, _)| *check == statement)
                || code.emission.omitted_check(statement).is_some()
                || code.nested_operation(
                    code.statement_operation(statement)
                        .unwrap_or(OpId(u32::MAX)),
                )
            {
                continue;
            }
            super::trace::cast_refused(key, statement, why);
        }
    }
    Ok(())
}

/// One original `check-cast` per statement whose own display adds no behavior.
/// Only a P4 `Redundant` disposition qualifies, and only for a statement whose
/// text states nothing else: a phase, a printed route or an output definition
/// keeps the statement, because its removal could not keep those obligations.
fn redundant(
    code: &MethodCode,
    types: &crate::p4::TypeFacts,
) -> Result<Vec<Option<OmittedCheck>>, ExpressionError> {
    let mut omitted: Vec<Option<OmittedCheck>> =
        super::super::build::filled(code.operations.len(), None)?;
    for check in types.checks() {
        let CheckDisposition::Redundant(evidence) = check.disposition() else {
            continue;
        };
        let Some(operation) = code.operations.get(check.operation().0 as usize) else {
            continue;
        };
        let Some(statement) = operation.statement else {
            continue;
        };
        let Some((value, reference)) = eligible_check(code, check.operation(), statement, None)
        else {
            continue;
        };
        let at = omitted
            .get_mut(statement.0 as usize)
            .ok_or(bug("check-cast names an unknown statement"))?;
        *at = Some(OmittedCheck {
            check: statement,
            operation: check.operation(),
            pc: operation.pc,
            value,
            reference,
            sources: copy(evidence.sources())?,
        });
    }
    Ok(omitted)
}

/// The checked storage and declared target of a `check-cast` that prints no
/// separate work of its own. A phase or printed route would be an event after
/// the check, and a result definition would be text only the statement carries.
fn eligible_check(
    code: &MethodCode,
    operation: OpId,
    statement: StatementId,
    why_slots: Option<&mut [Option<&'static str>]>,
) -> Option<(StorageId, ReferenceId)> {
    let mut why_slots = why_slots;
    let mut refuse = |why: &'static str| {
        if let Some(slots) = why_slots.as_deref_mut() {
            why_at(slots, statement, why);
        }
    };
    let s = code.statement(statement)?;
    // Only a real check-cast can be refused; every other statement simply has no
    // carrier question, so it is silent.
    let ExprKind::CheckCast {
        value, reference, ..
    } = *s.operator(code)
    else {
        return None;
    };
    if !s.outputs.is_empty() {
        refuse("check_has_output");
        return None;
    }
    let ExprKind::Read { storage, .. } = code.expressions.get(value.0 as usize)?.kind else {
        refuse("value_not_a_read");
        return None;
    };
    let Some(phases) = code.emission.operations.get(operation.0 as usize) else {
        refuse("no_phase_record");
        return None;
    };
    if !phases.before.is_empty() || !phases.snapshots.is_empty() || !phases.after.is_empty() {
        refuse("check_has_copy_phase");
        return None;
    }
    if s.sites
        .iter()
        .any(|&site| code.emission.site_annotation(site))
    {
        refuse("check_site_is_printed");
        return None;
    }
    Some((storage, reference))
}

fn same_checked_read(code: &MethodCode, statement: StatementId, occurrence: ExprId) -> bool {
    matches!(
        (checked_read(code, statement), read_of(code, occurrence)),
        (Some((a, sa)), Some((b, sb))) if a == b && sa == sb
    )
}
/// The published `(value, state)` of a check's own read.
fn checked_read(
    code: &MethodCode,
    statement: StatementId,
) -> Option<(crate::p3::ValueId, crate::p3::StateId)> {
    let ExprKind::CheckCast { value, .. } = *code.statement(statement)?.operator(code) else {
        return None;
    };
    read_of(code, value)
}
fn carries_check(
    code: &MethodCode,
    access: &LocalAccess,
    statement: StatementId,
    storage: StorageId,
    occurrence: ExprId,
) -> bool {
    same_checked_read(code, statement, occurrence)
        || checked_read(code, statement)
            .is_some_and(|source| access.copied_read(code, source, occurrence, storage))
}
fn carried_check(
    code: &MethodCode,
    operation: OpId,
    statement: StatementId,
    value: StorageId,
    reference: ReferenceId,
    carrier: CheckCarrier,
    occurrence: ExprId,
) -> Result<(StatementId, InlineCheck), ExpressionError> {
    let pc = code
        .operations
        .get(operation.0 as usize)
        .ok_or(bug("check-cast lost its operation"))?
        .pc;
    Ok((
        statement,
        InlineCheck {
            check: statement,
            operation,
            pc,
            value,
            reference,
            carrier,
            occurrence,
        },
    ))
}

/// Only one check may name one printed occurrence. A second one keeps its own
/// statement: the exact carrier is what makes the composition a single-use
/// display, so it must never resolve to two omitted checks.
fn unclaimed_occurrence(found: &[(StatementId, InlineCheck)], check: &InlineCheck) -> bool {
    !found.iter().any(|(_, existing)| {
        existing.carrier == check.carrier && existing.occurrence == check.occurrence
    })
}

/// A check that names a whole stored value turns that store into its declaration.
///
/// The check reads exactly the value the store produced, the store is the only
/// one for that local, and the check action follows it directly, so the cast can
/// print on the initializer: `int[][] v = (int[][]) <producer>;`. Every later
/// read then prints the narrow declared type instead of repeating a cast, which
/// the single readable read carrier cannot do for a second use. Only a reference
/// type that narrows the current declaration qualifies.
fn definition_carrier(
    carrier: &Carrier<'_>,
    statement: StatementId,
    storage: StorageId,
) -> Result<Option<Carried>, Stop> {
    let code = carrier.code;
    let (node, index) = (carrier.node, carrier.index);
    let Some((previous_node, _, previous)) = carrier.adjacency.previous_action(code, node, index)
    else {
        return Ok(None);
    };
    let BodyAction::Statement {
        operation: definition_operation,
        statement: definition,
    } = previous
    else {
        return Err(Stop::Refuse("definition_not_adjacent"));
    };
    if carrier.protection[previous_node] != carrier.protection[node] {
        return Err(Stop::Refuse("definition_changes_protection"));
    }
    if code.nested_operation(definition_operation)
        || !matches!(
            code.operations[definition_operation.0 as usize].emission,
            Emission::Block { .. }
        )
    {
        return Err(Stop::Refuse("definition_is_not_standalone"));
    }
    let phases = &code.emission.operations[definition_operation.0 as usize];
    if !phases.before.is_empty() || !phases.snapshots.is_empty() || !phases.after.is_empty() {
        return Err(Stop::Refuse("definition_has_copy_phase"));
    }
    // The carrier is the definition, but the mapped check remains THIS original
    // check operation/PC, never the preceding store or invocation.
    let operation = carrier.operation;
    let Some((_, reference)) = eligible_check(code, operation, statement, None) else {
        return Ok(None);
    };
    let mut stores = code.statements().filter(|s| {
        s.outputs.iter().any(|output| {
            matches!(
                output.materialization(),
                Materialization::Store(local) if StorageId(local.0) == storage
            )
        })
    });
    let unique = stores.next().is_some_and(|s| s.id == definition) && stores.next().is_none();
    if !unique {
        return Err(Stop::Refuse("definition_is_not_the_only_store"));
    }
    let defined = code
        .require_statement(definition)?
        .outputs
        .iter()
        .find(|output| {
            matches!(
                output.materialization(),
                Materialization::Store(local) if StorageId(local.0) == storage
            )
        })
        .map(|output| output.state());
    if defined.is_none() || checked_read(code, statement).map(|(_, state)| state) != defined {
        return Err(Stop::Refuse("definition_state_differs"));
    }
    let Ok(ReferenceDeclaration::Type { type_use }) =
        &code.references[reference.0 as usize].declaration
    else {
        return Err(Stop::Refuse("check_target_not_a_type"));
    };
    let Some(target) = code.types.get(type_use.0 as usize) else {
        return Err(Stop::Refuse("check_target_lost_its_use"));
    };
    // Only the erased declaration narrows: a second, unrelated reference type
    // would replace a fact instead of recovering one.
    let object = |units: &[u16]| {
        units
            .iter()
            .copied()
            .eq(b"Ljava/lang/Object;".iter().copied().map(u16::from))
    };
    let reference_type =
        matches!(target.units.first(), Some(&unit) if unit == b'L' as u16 || unit == b'[' as u16);
    let narrows = !object(&target.units)
        && matches!(
            code.emission.local_types.get(storage.0 as usize).copied(),
            Some(LocalSpelling::Named(ty)) if object(&code.types[ty.0 as usize].units)
        );
    if !reference_type || !narrows || code.locals[storage.0 as usize].original.is_reused() {
        return Err(Stop::Refuse("definition_type_does_not_narrow"));
    }
    let pc = code
        .operations
        .get(operation.0 as usize)
        .ok_or(bug("check-cast lost its operation"))?
        .pc;
    let occurrence = match *code.require_statement(statement)?.operator(code) {
        ExprKind::CheckCast { value, .. } => value,
        _ => return Err(Stop::Error(bug("definition carrier lost its check"))),
    };
    Ok(Some(Carried::Declare {
        check: (
            statement,
            InlineCheck {
                check: statement,
                operation,
                pc,
                value: storage,
                reference,
                carrier: CheckCarrier::Definition(definition),
                occurrence,
            },
        ),
        local: storage,
        ty: *type_use,
    }))
}
/// One finite cursor through the supplied source schedule. A candidate read
/// must precede every acting event, phase, selection or protection change.
fn forward_operand(
    carrier: &Carrier<'_>,
    statement: StatementId,
    storage: StorageId,
) -> Result<Option<Carried>, Stop> {
    let code = carrier.code;
    let context = carrier.protection[carrier.node];
    let mut node = carrier.node;
    let mut index = carrier.index + 1;
    let mut remaining = code.emission.blocks.len();
    loop {
        if carrier.protection[node] != context {
            return Err(Stop::Refuse("read_changes_protection"));
        }
        let block = &code.emission.blocks[node];
        if let Some(action) = block.actions.get(index) {
            let BodyAction::Statement {
                operation,
                statement: consumer,
            } = *action
            else {
                return Err(Stop::Refuse("read_crosses_an_event"));
            };
            let source = &code.operations[operation.0 as usize];
            let phases = &code.emission.operations[operation.0 as usize];
            if !phases.before.is_empty()
                || carrier.suspended[consumer.0 as usize]
                || !statement_keeps_protection(code, operation, carrier.protection, context)
            {
                return Err(Stop::Refuse("read_has_phase_or_foreign_carrier"));
            }
            if let Some((_, occurrence)) = first_read(code, consumer, storage)?
                && rendered_once(code, consumer, occurrence)?
                && carries_check(code, carrier.access, statement, storage, occurrence)
            {
                return Ok(Some(Carried::Operand(occurrence)));
            }
            if source.effects != Effects::default()
                || source.exceptional.is_some()
                || !phases.snapshots.is_empty()
                || !phases.after.is_empty()
            {
                return Err(Stop::Refuse("read_crosses_an_effect"));
            }
            index += 1;
            continue;
        }
        let terminal = match block.end {
            BlockEnd::Annotated { .. } => {
                return Err(Stop::Refuse("annotated_terminal_not_a_check_carrier"));
            }
            BlockEnd::Terminal(terminal) => Some(terminal),
            BlockEnd::None => match code.blocks[node].completion {
                BlockCompletion::Syntax { terminal, .. } => Some(terminal),
                _ => None,
            },
            BlockEnd::Transfers(_) => None,
        };
        if let Some(terminal) = terminal {
            let phases = &code.emission.terminals[terminal.0 as usize];
            if !phases.before.is_empty() {
                return Err(Stop::Refuse("terminal_has_before_copies"));
            }
            if let Some(occurrence) = first_terminal_read(code, terminal, storage)?
                && occurrences(code, Roots::terminal(code, terminal)?, occurrence)? == 1
                && carries_check(code, carrier.access, statement, storage, occurrence)
            {
                return Ok(Some(Carried::Operand(occurrence)));
            }
            return Err(Stop::Refuse("terminal_has_no_available_checked_read"));
        }
        if block.actions.is_empty() && !carrier.adjacency.empty_is_quiet(node) {
            return Err(Stop::Refuse("empty_portion_has_an_effect"));
        }
        let BlockEnd::Transfers(links) = &block.end else {
            return Ok(None);
        };
        let [link] = links.as_slice() else {
            return Err(Stop::Refuse("read_crosses_a_selection"));
        };
        let original = &code.transfers[link.0 as usize];
        let transfer = &code.emission.transfers[link.0 as usize];
        if original.edge.is_some()
            || !transfer.copies.is_empty()
            || !matches!(transfer.spelling, TransferSpelling::Fallthrough)
        {
            return Err(Stop::Refuse("read_crosses_a_transfer_or_copy"));
        }
        let next = original.destination.0 as usize;
        if code.blocks[next].original_block != code.blocks[carrier.node].original_block
            || (carrier.adjacency.next_node(node) != Some(next)
                && !code.emission.blocks[next].actions.is_empty())
        {
            return Err(Stop::Refuse("read_is_not_in_the_supplied_run"));
        }
        remaining = remaining
            .checked_sub(1)
            .ok_or_else(|| bug("check operand run contains a cycle"))?;
        node = next;
        index = 0;
    }
}

/// Find the checked read before any evaluation that could act or throw.
/// Plain local reads and literals do not establish another execution boundary.
fn first_available_read(
    code: &MethodCode,
    roots: Roots<'_>,
    storage: StorageId,
) -> Result<Option<ExprId>, ExpressionError> {
    let mut walk = Walk::new(roots, Scope::Carrier)?;
    while let Some(event) = walk.next(code)? {
        let id = match event {
            Event::Nested(_) => return Ok(None),
            Event::End(id) => {
                if !matches!(
                    code.expressions[id.0 as usize].kind,
                    ExprKind::Read { .. } | ExprKind::Literal { .. }
                ) {
                    return Ok(None);
                }
                continue;
            }
            Event::Enter(id) => id,
        };
        match &code.expressions[id.0 as usize].kind {
            ExprKind::Read { storage: read, .. } if *read == storage => return Ok(Some(id)),
            ExprKind::Read { .. }
            | ExprKind::Literal { .. }
            | ExprKind::Unary { .. }
            | ExprKind::Binary { .. }
            | ExprKind::Compare { .. }
            | ExprKind::ArrayLength(_)
            | ExprKind::Call(_)
            | ExprKind::Array { .. }
            | ExprKind::Field {
                receiver: Some(_), ..
            }
            | ExprKind::InstanceOf { .. } => {}
            _ => return Ok(None),
        }
    }
    Ok(None)
}

fn first_terminal_read(
    code: &MethodCode,
    block: crate::p2::BlockId,
    storage: StorageId,
) -> Result<Option<ExprId>, ExpressionError> {
    if code.emission.origin_only_terminal(block) {
        return Ok(None);
    }
    first_available_read(code, Roots::terminal(code, block)?, storage)
}

fn first_read(
    code: &MethodCode,
    statement: StatementId,
    storage: StorageId,
) -> Result<Option<(StatementId, ExprId)>, ExpressionError> {
    let Some(roots) = Roots::statement(code, statement)? else {
        return Ok(None);
    };
    Ok(first_available_read(code, roots, storage)?.map(|occurrence| (statement, occurrence)))
}

/// Follow an already selected composition to its printed carrier. This reads
/// the P7 plan; it does not choose another evaluation position.
fn statement_keeps_protection(
    code: &MethodCode,
    mut operation: OpId,
    protection: &[Option<ControlId>],
    context: Option<ControlId>,
) -> bool {
    for _ in 0..code.operations.len() {
        let Some(source) = code.operations.get(operation.0 as usize) else {
            return false;
        };
        if protection.get(source.node.0 as usize).copied() != Some(context) {
            return false;
        }
        if !code.nested_operation(operation) {
            return true;
        }
        let Some(origin) = source
            .nested_at
            .and_then(|operand| code.expressions.get(operand.0 as usize))
            .and_then(|expression| expression.origin.as_ref())
        else {
            return false;
        };
        match origin.site {
            crate::p3::UseSite::Operation {
                operation: target, ..
            } => operation = target,
            crate::p3::UseSite::Terminal { block, .. } => {
                return code.blocks.iter().enumerate().any(|(node, body)| {
                    let terminal = match body.completion {
                        BlockCompletion::Ordinary(terminal)
                        | BlockCompletion::Syntax { terminal, .. } => Some(terminal),
                        BlockCompletion::None => None,
                    };
                    terminal == Some(block) && protection.get(node).copied() == Some(context)
                });
            }
        }
    }
    false
}

/// The check is the last action of its block: a copy or terminal carrier may
/// only print at the event that follows it with nothing in between.
fn last_action_check(
    carrier: &Carrier<'_>,
    statement: StatementId,
) -> Option<(crate::p7::ReferenceId, usize)> {
    let code = carrier.code;
    let block = &code.emission.blocks[carrier.node];
    if carrier.index + 1 != block.actions.len() {
        return None;
    }
    let BodyAction::Statement { operation, .. } = block.actions[carrier.index] else {
        return None;
    };
    let (_, reference) = eligible_check(code, operation, statement, None)?;
    Some((reference, carrier.node))
}

/// The first step of this transfer's original copy bundle owns the checked read.
/// A selection, Save, relocated bundle or earlier step keeps the check separate.
fn copy_carrier(
    carrier: &Carrier<'_>,
    statement: StatementId,
    storage: StorageId,
) -> Result<Option<Carried>, Stop> {
    let code = carrier.code;
    let Some((reference, node)) = last_action_check(carrier, statement) else {
        return Ok(None);
    };
    let BodyAction::Statement { operation, .. } = code.emission.blocks[node].actions[carrier.index]
    else {
        return Ok(None);
    };
    let block = &code.emission.blocks[node];
    let BlockEnd::Transfers(transfers) = &block.end else {
        // A terminal evaluates its own operands and selects between its
        // original edges, so the check still precedes both outcomes.
        return Err(Stop::Refuse("block_ends_in_a_terminal"));
    };
    let [transfer] = transfers.as_slice() else {
        return Err(Stop::Refuse("block_has_two_transfers"));
    };
    let Some(planned) = code.emission.transfers.get(transfer.0 as usize) else {
        return Ok(None);
    };
    if matches!(planned.spelling, TransferSpelling::Selected) {
        // A selected arm prints this bundle for one outcome only.
        return Err(Stop::Refuse("transfer_is_a_selected_arm"));
    }
    let [bundle] = planned.copies.as_slice() else {
        return Err(Stop::Refuse("transfer_has_not_exactly_one_bundle"));
    };
    let Some(bundle) = code.copies.get(bundle.0 as usize) else {
        return Ok(None);
    };
    let CopyLocation::Transfer {
        site: TransferSite::NormalEdge(edge),
        ..
    } = bundle.location
    else {
        return Err(Stop::Refuse("bundle_is_not_a_normal_edge_copy"));
    };
    // The bundle must be this transfer's own normal edge, not a copy another
    // composition step placed here from a different origin.
    if code.transfers.get(transfer.0 as usize).and_then(|t| t.edge) != Some(edge) {
        return Err(Stop::Refuse("bundle_edge_is_not_this_transfer"));
    }
    if !matches!(bundle.emission, CopyEmission::Original) {
        return Err(Stop::Refuse("bundle_was_relocated"));
    }
    let Some(CopyStatement::Assign {
        destination,
        expression,
        ..
    }) = bundle.steps.first()
    else {
        return Err(Stop::Refuse("first_copy_step_is_not_an_assignment"));
    };
    if *destination == storage {
        // The surviving assignment would look like the check wrote the local.
        return Err(Stop::Refuse("copy_writes_the_checked_local"));
    }
    if !reads(code, *expression, storage) {
        return Err(Stop::Refuse("copy_rhs_is_another_value"));
    }
    let carried = carried_check(
        code,
        operation,
        statement,
        storage,
        reference,
        // The carrier is the bundle's first executed step.
        CheckCarrier::Copy {
            bundle: bundle.id,
            step: 0,
        },
        *expression,
    )?;
    if !unclaimed_occurrence(carrier.found, &carried.1) {
        return Err(Stop::Refuse("occurrence_claimed"));
    }
    Ok(Some(Carried::At(carried)))
}

fn protected_contexts(code: &MethodCode) -> Result<Vec<Option<ControlId>>, ExpressionError> {
    let mut nodes = super::super::build::filled(code.blocks.len(), None)?;
    let mut work = Vec::new();
    if let Some(root) = code.root {
        push(&mut work, (root, None))?;
    }
    while let Some((id, context)) = work.pop() {
        let control = &code.controls[id.0 as usize];
        if let ControlKind::Block { node } = control.kind {
            nodes[node.0 as usize] = context;
        }
        for (index, &child) in control.source_children.iter().enumerate() {
            // A Try protects its body, not its own catch clauses. Nested Try
            // identities retain the rest of their enclosing scope ancestry.
            let child_context = if matches!(control.kind, ControlKind::Try { .. }) && index == 0 {
                Some(id)
            } else {
                context
            };
            push(&mut work, (child, child_context))?;
        }
    }
    Ok(nodes)
}

/// Count only this carrier's occurrences; nested evaluation owns its own tree.
fn rendered_once(
    code: &MethodCode,
    statement: StatementId,
    occurrence: ExprId,
) -> Result<bool, ExpressionError> {
    if code
        .statement_operation(statement)
        .is_some_and(|op| code.origin_only_operation(op))
    {
        return Ok(false);
    }
    let Some(roots) = Roots::statement(code, statement)? else {
        return Ok(false);
    };
    Ok(occurrences(code, roots, occurrence)? == 1)
}

fn reads(code: &MethodCode, expression: ExprId, storage: StorageId) -> bool {
    matches!(
        code.expressions.get(expression.0 as usize).map(|e| &e.kind),
        Some(ExprKind::Read { storage: read, .. }) if *read == storage
    )
}
