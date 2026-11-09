//! Composes an original `check-cast` into the existing display of its value's
//! first read, or omits it when P4 established it cannot fail there.
//!
//! Nothing is reordered and nothing is created: the check statement, its
//! operation, its exception site and the checked local all keep their original
//! facts, and the read that prints the check keeps its own expression. Only the
//! check's separate display action is dropped, so the surviving text still names
//! one original check.
//!
//! The fixed priority is an adjacent standalone definition, the next statement,
//! the first step of one unconditional copy transfer, the immediate terminal,
//! then a later first read in the same supplied linear run. Adjacency can cross
//! whole-operation portions, but never an event, phase, selection or protected
//! scope. Every carrier names its exact original read and CHECK operation/PC.
//!
//! A check whose pre-check disposition P4 published as redundant never composes:
//! the cast states nothing there, so its own statement is omitted and the
//! surviving carrier prints its original text unchanged.
//!
//! Anything else keeps the standalone check statement: a phase, a route
//! annotation, a value write, a selection, a Save or a gap report between the
//! check and the read would move the check past an event it now precedes.

use super::super::build::{bug, copy, push};
use super::super::*;
use crate::p2::{Effects, OpId};
use crate::p4::CheckDisposition;
use crate::p5::{CopyLocation, Materialization, TransferSite};

/// The first reason this statement could not print at a read. The first refusal
/// is the one that decides; later paths would only repeat a symptom.
/// Shared read-only state one carrier decision may consult. The driver owns the
/// iteration, the "already claimed" bookkeeping and every written slot.
struct Carrier<'a> {
    code: &'a MethodCode,
    adjacency: &'a super::adjacency::Adjacency,
    protection: &'a [Option<ControlId>],
    found: &'a [(StatementId, InlineCheck)],
    /// The operation that owns each statement, so a carrier never rescans.
    operation_of: &'a [Option<OpId>],
    /// Whether a suspension surface owns that statement's operands.
    suspended: &'a [bool],
    /// Where the driver found this check in the printed order.
    node: usize,
    index: usize,
    operation: OpId,
}
/// What one carrier decided for one check statement.
enum Carried {
    /// The check prints at this occurrence.
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
type Find = for<'a> fn(&Carrier<'a>, StatementId, StorageId) -> Result<Option<Carried>, Stop>;

/// Priority is this order. A carrier that refuses leaves the check to the next
/// one; the driver keeps the first refusal per check for the trace.
const CARRIERS: &[Find] = &[
    definition_carrier,
    statement_carrier,
    copy_carrier,
    terminal_carrier,
    distant_carrier,
];

fn why_at(slots: &mut [Option<&'static str>], statement: StatementId, why: &'static str) {
    if let Some(slot) = slots.get_mut(statement.0 as usize)
        && slot.is_none()
    {
        *slot = Some(why);
    }
}

pub(super) fn construct(
    code: &mut MethodCode,
    types: &crate::p4::TypeFacts,
    key: crate::MethodKey,
    adjacency: &super::adjacency::Adjacency,
) -> Result<(), ExpressionError> {
    let omitted = redundant(code, types)?;
    let mut why_slots: Vec<Option<&'static str>> =
        super::super::build::filled(code.statements.len(), None)?;
    let mut found: Vec<(StatementId, InlineCheck)> = Vec::new();
    let protection = protected_contexts(code)?;
    let mut operation_of: Vec<Option<OpId>> =
        super::super::build::filled(code.statements.len(), None)?;
    for (index, operation) in code.operations.iter().enumerate() {
        if let Some(statement) = operation.statement {
            operation_of[statement.0 as usize] = Some(OpId(
                u32::try_from(index).map_err(|_| crate::RequestError::PlatformCapacity)?,
            ));
        }
    }
    let mut suspended: Vec<bool> = super::super::build::filled(code.statements.len(), false)?;
    for surface in code.emission.suspensions() {
        suspended[surface.statement.0 as usize] = true;
    }
    let mut claimed: Vec<bool> = super::super::build::filled(code.statements.len(), false)?;
    // One traversal: every check statement asks the carriers in priority order,
    // and the driver applies the one decision that answers.
    for node in 0..code.emission.blocks.len() {
        for index in 0..code.emission.blocks[node].actions.len() {
            let BodyAction::Statement {
                operation,
                statement,
            } = code.emission.blocks[node].actions[index]
            else {
                continue;
            };
            if claimed[statement.0 as usize]
                || omitted
                    .get(statement.0 as usize)
                    .is_some_and(Option::is_some)
            {
                continue;
            }
            let Some((storage, _)) = eligible_check(code, operation, statement, None) else {
                continue;
            };
            let carrier = Carrier {
                code,
                adjacency,
                protection: &protection,
                found: &found,
                operation_of: &operation_of,
                suspended: &suspended,
                node,
                index,
                operation,
            };
            for find in CARRIERS {
                match find(&carrier, statement, storage) {
                    Ok(Some(Carried::At(check))) => {
                        claimed[statement.0 as usize] = true;
                        push(&mut found, check)?;
                        break;
                    }
                    Ok(Some(Carried::Declare { check, local, ty })) => {
                        code.emission.local_types[local.0 as usize] = LocalSpelling::Named(ty);
                        code.locals[local.0 as usize].declared_types = vec![ty];
                        claimed[statement.0 as usize] = true;
                        push(&mut found, check)?;
                        break;
                    }
                    Ok(None) => {}
                    Err(Stop::Refuse(why)) => why_at(&mut why_slots, statement, why),
                    Err(Stop::Error(error)) => return Err(error),
                }
            }
        }
    }
    // A redundant check has no text to carry, so its value's read keeps its own
    // spelling and the check's statement only contributes its origins.
    let mut kept: Vec<(StatementId, InlineCheck)> = Vec::new();
    for (statement, check) in found {
        if omitted
            .get(statement.0 as usize)
            .is_some_and(Option::is_some)
        {
            continue;
        }
        let copied = if code.locals[check.value.0 as usize].original.is_reused() {
            checked_copy_states(code, statement, check.value)?
        } else {
            Vec::new()
        };
        if !carries_check(code, statement, check.value, check.occurrence, &copied) {
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
                CheckCarrier::Statement(statement) => {
                    &mut code.emission.check_uses[statement.0 as usize]
                }
                CheckCarrier::Copy { bundle, .. } => {
                    &mut code.emission.copy_check_uses[bundle.0 as usize]
                }
                CheckCarrier::Terminal(terminal) => {
                    &mut code.emission.terminal_check_uses[terminal.0 as usize]
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
                || omitted.get(index).is_some_and(Option::is_some)
            {
                continue;
            }
            super::trace::cast_refused(key, statement, why);
        }
    }
    code.emission.omitted_checks = omitted;
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
        super::super::build::filled(code.statements.len(), None)?;
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
    let s = code.statements.get(statement.0 as usize)?;
    // Only a real check-cast can be refused; every other statement simply has no
    // carrier question, so it is silent.
    let StatementKind::CheckCast { value, reference } = s.kind else {
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
    let StatementKind::CheckCast { value, .. } = code.statements[statement.0 as usize].kind else {
        return None;
    };
    read_of(code, value)
}
fn read_of(
    code: &MethodCode,
    expression: ExprId,
) -> Option<(crate::p3::ValueId, crate::p3::StateId)> {
    match code
        .expressions
        .get(expression.0 as usize)?
        .origin
        .as_ref()?
        .read
        .ok()?
    {
        crate::p5::StorageRead::Value { value, state, .. } => Some((value, state)),
        _ => None,
    }
}
/// States in which the storage holds a pure copy of the checked value.
///
/// P5 can merge the copy of a checked local into that local, which then carries
/// several value bindings and blocks the reused-storage carrier below. Copying a
/// reference does not change the object's type, so the copy's own read keeps the
/// check: it reads exactly the checked `(value, state)`, and the carrier read is
/// the state that copy published. No other event sits between them, because the
/// copy's input already names the checked read itself.
fn checked_copy_states(
    code: &MethodCode,
    statement: StatementId,
    storage: StorageId,
) -> Result<Vec<crate::p3::StateId>, ExpressionError> {
    let Some(source) = checked_read(code, statement) else {
        return Ok(Vec::new());
    };
    let mut states = Vec::new();
    for candidate in &code.statements {
        if candidate.id == statement {
            continue;
        }
        let StatementKind::Move { expression, .. } = candidate.kind else {
            continue;
        };
        if read_of(code, expression) != Some(source) {
            continue;
        }
        let [output] = candidate.outputs.as_slice() else {
            continue;
        };
        if !matches!(output.materialization(), Materialization::Store(local) if StorageId(local.0) == storage)
        {
            continue;
        }
        push(&mut states, output.state())?;
    }
    Ok(states)
}
fn carries_check(
    code: &MethodCode,
    statement: StatementId,
    storage: StorageId,
    occurrence: ExprId,
    copied: &[crate::p3::StateId],
) -> bool {
    !code.locals[storage.0 as usize].original.is_reused()
        || same_checked_read(code, statement, occurrence)
        || read_of(code, occurrence).is_some_and(|(_, state)| copied.contains(&state))
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

/// The checked value read by the very next statement action, at that statement's
/// first evaluated value. Nothing is reordered: the check still happens before
/// every effect of that statement, and the checked local is not written.
fn statement_carrier(
    carrier: &Carrier<'_>,
    statement: StatementId,
    storage: StorageId,
) -> Result<Option<Carried>, Stop> {
    let code = carrier.code;
    let (node, index) = (carrier.node, carrier.index);
    let actions = &code.emission.blocks[node].actions;
    let Some((_, _, next_action)) = carrier.adjacency.next_action(code, node, index) else {
        return Ok(None);
    };
    let BodyAction::Statement {
        operation: consumer,
        statement: next,
    } = next_action
    else {
        return Err(Stop::Refuse("next_action_is_not_a_statement"));
    };
    let BodyAction::Statement { operation, .. } = actions[index] else {
        return Ok(None);
    };
    let Some((_, reference)) = eligible_check(code, operation, statement, None) else {
        return Ok(None);
    };
    if !code.emission.operations[consumer.0 as usize]
        .before
        .is_empty()
    {
        return Err(Stop::Refuse("next_statement_has_before_copies"));
    }
    if carrier.suspended[next.0 as usize] {
        // This surface has an implicit continuation occurrence. Keep a
        // preceding check at its original action rather than attach it
        // to an operand that this surface does not print.
        return Err(Stop::Refuse("suspension_owns_its_operands"));
    }
    if !statement_keeps_protection(code, consumer, carrier.protection, carrier.protection[node]) {
        return Err(Stop::Refuse("next_statement_changes_protection"));
    }
    if code.statements[next.0 as usize]
        .outputs
        .iter()
        .any(|output| {
            matches!(
                output.materialization(),
                Materialization::Store(local) if StorageId(local.0) == storage
            )
        })
    {
        return Err(Stop::Refuse("next_statement_writes_the_value"));
    }
    let Some(occurrence) = observable_carrier(code, next, storage)? else {
        return Err(Stop::Refuse("first_observable_is_another_value"));
    };
    if !rendered_once(code, next, occurrence)? {
        return Err(Stop::Refuse("first_leaf_renders_twice"));
    }
    let carried = carried_check(
        code,
        operation,
        statement,
        storage,
        reference,
        CheckCarrier::Statement(next),
        occurrence,
    )?;
    if !unclaimed_occurrence(carrier.found, &carried.1) {
        return Err(Stop::Refuse("occurrence_claimed"));
    }
    Ok(Some(Carried::At(carried)))
}

/// Attach a still standalone check to the first printed read of its value. The
/// carrier must sit in the same protection context and render that read once.
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
    if code.emission.composed_operation(definition_operation)
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
    let mut stores = code.statements.iter().filter(|s| {
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
    let defined = code.statements[definition.0 as usize]
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
    let occurrence = match code.statements[statement.0 as usize].kind {
        StatementKind::CheckCast { value, .. } => value,
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
/// A later carrier must be reached through the supplied linear source schedule,
/// with no intervening effect, phase, selection or protection change. Searching
/// for a matching value elsewhere in the method is not an ordering guarantee.
fn distant_path(carrier: &Carrier<'_>, target: usize, statement: Option<StatementId>) -> bool {
    let code = carrier.code;
    let mut node = carrier.node;
    let mut index = carrier.index + 1;
    loop {
        if carrier.protection[node] != carrier.protection[carrier.node] {
            return false;
        }
        let actions = &code.emission.blocks[node].actions;
        if node == target && statement.is_none() && index == actions.len() {
            return true;
        }
        if let Some(action) = actions.get(index) {
            let BodyAction::Statement {
                operation,
                statement: current,
            } = *action
            else {
                return false;
            };
            if node == target && statement == Some(current) {
                return true;
            }
            let source = &code.operations[operation.0 as usize];
            let phases = &code.emission.operations[operation.0 as usize];
            if source.effects != Effects::default()
                || source.exceptional.is_some()
                || !phases.before.is_empty()
                || !phases.snapshots.is_empty()
                || !phases.after.is_empty()
            {
                return false;
            }
            index += 1;
        } else {
            if actions.is_empty() && !carrier.adjacency.empty_is_quiet(node) {
                return false;
            }
            let Some(next) = carrier.adjacency.next_node(node) else {
                return false;
            };
            node = next;
            index = 0;
        }
    }
}

fn distant_carrier(
    carrier: &Carrier<'_>,
    statement: StatementId,
    storage: StorageId,
) -> Result<Option<Carried>, Stop> {
    let code = carrier.code;
    let (node, operation) = (carrier.node, carrier.operation);
    let BodyAction::Statement { .. } = code.emission.blocks[node].actions[carrier.index] else {
        return Ok(None);
    };
    let Some((_, reference)) = eligible_check(code, operation, statement, None) else {
        return Ok(None);
    };
    let copied = if code.locals[storage.0 as usize].original.is_reused() {
        checked_copy_states(code, statement, storage)?
    } else {
        Vec::new()
    };
    let mut why = None;
    for candidate in &code.statements {
        if candidate.id == statement {
            continue;
        }
        let Some((_, occurrence)) = first_read(code, candidate.id, storage)? else {
            continue;
        };
        let Some(consumed) = carrier.operation_of[candidate.id.0 as usize]
            .and_then(|operation| code.operations.get(operation.0 as usize))
        else {
            continue;
        };
        if !code.emission.operations[consumed.source.0 as usize]
            .before
            .is_empty()
            || carrier.suspended[candidate.id.0 as usize]
        {
            why = Some("distant_use_has_before_phase_or_suspension");
            continue;
        }
        if !distant_path(carrier, consumed.node.0 as usize, Some(candidate.id)) {
            why = Some("distant_use_crosses_an_event");
            continue;
        }
        if carrier.protection.get(consumed.node.0 as usize).copied()
            != carrier.protection.get(node).copied()
        {
            why = Some("distant_use_changes_protection");
            continue;
        }
        if !rendered_once(code, candidate.id, occurrence)? {
            why = Some("distant_read_renders_twice");
            continue;
        }
        // A merged local publishes several value bindings; its check still
        // carries to the read of a pure copy of the checked value.
        if !carries_check(code, statement, storage, occurrence, &copied) {
            why = Some("distant_checked_value_changed");
            continue;
        }
        let carried = carried_check(
            code,
            operation,
            statement,
            storage,
            reference,
            CheckCarrier::Statement(candidate.id),
            occurrence,
        )?;
        return if unclaimed_occurrence(carrier.found, &carried.1) {
            Ok(Some(Carried::At(carried)))
        } else {
            Err(Stop::Refuse("distant_occurrence_claimed"))
        };
    }
    for (index, body) in code.blocks.iter().enumerate() {
        let terminal = match body.completion {
            BlockCompletion::Ordinary(terminal) | BlockCompletion::Syntax { terminal, .. } => {
                terminal
            }
            BlockCompletion::None => continue,
        };
        let Some(occurrence) = first_terminal_read(code, terminal, storage)? else {
            continue;
        };
        if !code.emission.terminals[terminal.0 as usize]
            .before
            .is_empty()
        {
            why = Some("distant_terminal_has_before_copies");
            continue;
        }
        if !distant_path(carrier, index, None) {
            why = Some("distant_terminal_crosses_an_event");
            continue;
        }
        if carrier.protection.get(index).copied() != carrier.protection.get(node).copied() {
            why = Some("distant_use_changes_protection");
            continue;
        }
        let carried = carried_check(
            code,
            operation,
            statement,
            storage,
            reference,
            CheckCarrier::Terminal(terminal),
            occurrence,
        )?;
        return if unclaimed_occurrence(carrier.found, &carried.1) {
            Ok(Some(Carried::At(carried)))
        } else {
            Err(Stop::Refuse("distant_occurrence_claimed"))
        };
    }
    match why {
        Some(why) => Err(Stop::Refuse(why)),
        None => Ok(None),
    }
}

/// The first executed read of one storage among a terminal's operands.
fn first_terminal_read(
    code: &MethodCode,
    block: crate::p2::BlockId,
    storage: StorageId,
) -> Result<Option<ExprId>, ExpressionError> {
    if code.emission.origin_only_terminal(block) {
        return Ok(None);
    }
    let Some(terminal) = code.terminals.get(block.0 as usize) else {
        return Ok(None);
    };
    let mut roots = Vec::new();
    match &terminal.kind {
        TerminalKind::Return(Some(value)) | TerminalKind::Throw(value) => {
            push(&mut roots, *value)?;
        }
        TerminalKind::If { condition, .. } => match condition {
            Condition::Pair { left, right, .. } => {
                push(&mut roots, *left)?;
                push(&mut roots, *right)?;
            }
            Condition::Zero { value, .. } => push(&mut roots, *value)?,
        },
        TerminalKind::Switch { selector, .. } => push(&mut roots, *selector)?,
        TerminalKind::None | TerminalKind::Goto(_) | TerminalKind::Return(None) => {}
    }
    let mut work = Vec::new();
    for root in roots.into_iter().rev() {
        push(&mut work, root)?;
    }
    while let Some(id) = work.pop() {
        let Some(expression) = code.expressions.get(id.0 as usize) else {
            return Ok(None);
        };
        if code.emission.composed_use(id).is_some() {
            return Ok(None);
        }
        match &expression.kind {
            ExprKind::Read { storage: read, .. } if *read == storage => return Ok(Some(id)),
            ExprKind::Unary { value, .. } => push(&mut work, *value)?,
            ExprKind::Binary { left, right, .. } | ExprKind::Compare { left, right, .. } => {
                push(&mut work, *right)?;
                push(&mut work, *left)?;
            }
            ExprKind::Array { array, index, .. } => {
                push(&mut work, *index)?;
                push(&mut work, *array)?;
            }
            ExprKind::Field {
                receiver: Some(receiver),
                ..
            } => push(&mut work, *receiver)?,
            ExprKind::ArrayLength(array) => push(&mut work, *array)?,
            ExprKind::InstanceOf { value, .. } => push(&mut work, *value)?,
            ExprKind::Call(call) => {
                if let Arguments::Known(args) = &call.arguments {
                    for &argument in args.iter().rev() {
                        push(&mut work, argument)?;
                    }
                }
                if let Some(receiver) = call.receiver {
                    push(&mut work, receiver)?;
                }
            }
            _ => return Ok(None),
        }
    }
    Ok(None)
}

/// The first printed read of one storage inside this statement's expression
/// roots, in the order P9 evaluates them.
fn first_read(
    code: &MethodCode,
    statement: StatementId,
    storage: StorageId,
) -> Result<Option<(StatementId, ExprId)>, ExpressionError> {
    let Some(statement_ref) = code.statements.get(statement.0 as usize) else {
        return Ok(None);
    };
    let mut roots = Vec::new();
    match &statement_ref.kind {
        StatementKind::Move { expression, .. }
        | StatementKind::Evaluate { expression }
        | StatementKind::Initialize { expression, .. } => push(&mut roots, *expression)?,
        StatementKind::FieldStore {
            receiver, value, ..
        } => {
            if code.emission.natural_store(statement)
                && let Some(receiver) = receiver
            {
                push(&mut roots, *receiver)?;
            }
            push(&mut roots, *value)?;
        }
        StatementKind::ArrayStore {
            array,
            index,
            value,
            ..
        } => {
            if code.emission.natural_store(statement) {
                push(&mut roots, *array)?;
                push(&mut roots, *index)?;
            }
            push(&mut roots, *value)?;
        }
        _ => return Ok(None),
    }
    let mut work = Vec::new();
    for root in roots.into_iter().rev() {
        push(&mut work, root)?;
    }
    while let Some(id) = work.pop() {
        let Some(expression) = code.expressions.get(id.0 as usize) else {
            return Ok(None);
        };
        if code.emission.composed_use(id).is_some() {
            return Ok(None);
        }
        match &expression.kind {
            ExprKind::Read { storage: read, .. } if *read == storage => {
                return Ok(Some((statement, id)));
            }
            ExprKind::Unary { value, .. } => push(&mut work, *value)?,
            ExprKind::Binary { left, right, .. } | ExprKind::Compare { left, right, .. } => {
                push(&mut work, *right)?;
                push(&mut work, *left)?;
            }
            ExprKind::Array { array, index, .. } => {
                push(&mut work, *index)?;
                push(&mut work, *array)?;
            }
            ExprKind::Field {
                receiver: Some(receiver),
                ..
            } => push(&mut work, *receiver)?,
            ExprKind::ArrayLength(array) => push(&mut work, *array)?,
            ExprKind::InstanceOf { value, .. } => push(&mut work, *value)?,
            ExprKind::Call(call) => {
                if let Arguments::Known(args) = &call.arguments {
                    for &argument in args.iter().rev() {
                        push(&mut work, argument)?;
                    }
                }
                if let Some(receiver) = call.receiver {
                    push(&mut work, receiver)?;
                }
            }
            _ => return Ok(None),
        }
    }
    Ok(None)
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
        if !code.emission.composed_operation(operation) {
            return true;
        }
        let mut uses = code
            .emission
            .composed_uses
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.is_some_and(|entry| entry.operation == operation));
        let Some((index, _)) = uses.next() else {
            return false;
        };
        if uses.next().is_some() {
            return false;
        }
        let Some(origin) = code
            .expressions
            .get(index)
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

/// The checked value read by the first executed step of this block's single
/// unconditional transfer copy set. A check that is the block's last action and
/// whose transfer carries no earlier step is followed by that read immediately;
/// a terminal, a second transfer, a Save or another bundle between them keeps
/// the standalone statement.
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

/// The checked value read by the first evaluated operand of this block's own
/// terminal. The check is the block's last action, so the terminal follows it
/// with nothing printed in between; a before-terminal copy, snapshot or selected
/// arm would move the check past that event and keeps the standalone statement.
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
        for (index, &child) in control.children.iter().enumerate() {
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

fn terminal_carrier(
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
    // The terminal may sit in a following portion of the same original
    // block: a plain within-block link with no copy prints nothing between
    // the check and the terminal operand.
    let Some(terminal) = terminal_after(code, carrier.protection, node)? else {
        return Err(Stop::Refuse("no_terminal_follows"));
    };
    let phases = &code.emission.terminals[terminal.0 as usize];
    if !phases.before.is_empty() {
        return Err(Stop::Refuse("terminal_has_before_copies"));
    }
    let target = &code.terminals[terminal.0 as usize];
    let operand = match &target.kind {
        TerminalKind::Return(Some(value)) | TerminalKind::Throw(value) => *value,
        TerminalKind::If { condition, .. } => match condition {
            Condition::Pair { left, .. } => *left,
            Condition::Zero { value, .. } => *value,
        },
        TerminalKind::Switch { selector, .. } => *selector,
        TerminalKind::Return(None) | TerminalKind::None | TerminalKind::Goto(_) => {
            return Err(Stop::Refuse("terminal_has_no_operand"));
        }
    };
    let Some(occurrence) = leftmost(code, operand) else {
        return Err(Stop::Refuse("terminal_operand_has_no_first_leaf"));
    };
    if !reads(code, occurrence, storage) {
        return Err(Stop::Refuse("terminal_first_leaf_is_another_value"));
    }
    if !once_in(code, operand, occurrence)? {
        return Err(Stop::Refuse("terminal_operand_renders_twice"));
    }
    let carried = carried_check(
        code,
        operation,
        statement,
        storage,
        reference,
        CheckCarrier::Terminal(terminal),
        occurrence,
    )?;
    if !unclaimed_occurrence(carrier.found, &carried.1) {
        return Err(Stop::Refuse("occurrence_claimed"));
    }
    Ok(Some(Carried::At(carried)))
}

/// The terminal this block reaches without printing anything in between: its own
/// end, or a chain of empty blocks joined by single copy-free within-block links.
fn terminal_after(
    code: &MethodCode,
    protection: &[Option<ControlId>],
    mut node: usize,
) -> Result<Option<crate::p2::BlockId>, ExpressionError> {
    for _ in 0..code.emission.blocks.len() {
        let block = &code.emission.blocks[node];
        match &block.end {
            BlockEnd::Terminal(terminal) => return Ok(Some(*terminal)),
            BlockEnd::None => {
                // A syntax-owned predicate portion: the If or loop control prints
                // this terminal's operand, and nothing else stands between.
                if let BlockCompletion::Syntax {
                    terminal,
                    owner: TerminalEmission::If(_) | TerminalEmission::Loop(_),
                } = code.blocks[node].completion
                {
                    return Ok(Some(terminal));
                }
                return Ok(None);
            }
            BlockEnd::Transfers(transfers) => {
                let [only] = transfers.as_slice() else {
                    return Ok(None);
                };
                let transfer = code
                    .emission
                    .transfers
                    .get(only.0 as usize)
                    .ok_or(bug("check-cast carrier names an unknown transfer"))?;
                if !transfer.copies.is_empty()
                    || !matches!(transfer.spelling, TransferSpelling::Fallthrough)
                {
                    return Ok(None);
                }
                let from = code
                    .transfers
                    .get(only.0 as usize)
                    .ok_or(bug("check-cast carrier names an unknown transfer plan"))?;
                if from.edge.is_some() {
                    return Ok(None);
                }
                let next = from.destination.0 as usize;
                if protection[node] != protection[next] {
                    return Ok(None);
                }
                if code
                    .emission
                    .blocks
                    .get(next)
                    .is_none_or(|next| !next.actions.is_empty())
                {
                    return Ok(None);
                }
                node = next;
            }
        }
    }
    Err(bug("check-cast carrier walk did not reach a terminal"))
}

/// Whether this expression tree renders the occurrence exactly once. A composed
/// subtree prints under another carrier, so it is not a position of this one.
fn once_in(code: &MethodCode, root: ExprId, occurrence: ExprId) -> Result<bool, ExpressionError> {
    let mut work = Vec::new();
    push(&mut work, root)?;
    let mut seen = 0u8;
    while let Some(id) = work.pop() {
        if id == occurrence {
            seen = seen.saturating_add(1);
            if seen > 1 {
                return Ok(false);
            }
        }
        let expression = code
            .expressions
            .get(id.0 as usize)
            .ok_or(bug("terminal expression is outside its arena"))?;
        if code.emission.composed_use(id).is_some() {
            continue;
        }
        match &expression.kind {
            ExprKind::Unary { value, .. } => push(&mut work, *value)?,
            ExprKind::Binary { left, right, .. } | ExprKind::Compare { left, right, .. } => {
                push(&mut work, *left)?;
                push(&mut work, *right)?;
            }
            ExprKind::Array { array, index, .. } => {
                push(&mut work, *array)?;
                push(&mut work, *index)?;
            }
            ExprKind::Field {
                receiver: Some(receiver),
                ..
            } => push(&mut work, *receiver)?,
            ExprKind::ArrayLength(array) => push(&mut work, *array)?,
            ExprKind::InstanceOf { value, .. } => push(&mut work, *value)?,
            ExprKind::NewArray { length, .. } => push(&mut work, *length)?,
            ExprKind::Call(call) => {
                if let Some(receiver) = call.receiver {
                    push(&mut work, receiver)?;
                }
                if let Arguments::Known(args) = &call.arguments {
                    for &argument in args {
                        push(&mut work, argument)?;
                    }
                }
            }
            ExprKind::New { arguments, .. } => {
                for &argument in arguments {
                    push(&mut work, argument)?;
                }
            }
            ExprKind::FilledArray {
                elements: Arguments::Known(elements),
                ..
            } => {
                for &element in elements {
                    push(&mut work, element)?;
                }
            }
            _ => {}
        }
    }
    Ok(seen == 1)
}

/// The leftmost value this statement evaluates in its printed form. This is the
/// exact expression occurrence the check can print at.
/// The exact occurrence where a check may print inside this statement: the first
/// evaluation a reader could observe. Evaluation walks the whole expression in
/// order; a plain local read or literal before the checked value neither acts nor
/// throws, so the check moving past it is not observable. An acting node (call,
/// field read, array access, allocation, instance-of) ends the walk.
fn observable_carrier(
    code: &MethodCode,
    statement: StatementId,
    storage: StorageId,
) -> Result<Option<ExprId>, ExpressionError> {
    enum Step {
        Visit(ExprId),
        /// This value has acted; nothing later may carry the check.
        Act,
    }
    let Some(statement) = code.statements.get(statement.0 as usize) else {
        return Ok(None);
    };
    let mut roots = Vec::new();
    match &statement.kind {
        StatementKind::Move { expression, .. } | StatementKind::Evaluate { expression } => {
            push(&mut roots, *expression)?;
        }
        // An explicit constructor call prints receiver and arguments in
        // evaluation order, so a checked argument keeps the check before every
        // later argument and before the constructor itself.
        StatementKind::Initialize { expression, .. } => {
            push(&mut roots, *expression)?;
        }
        StatementKind::FieldStore {
            receiver, value, ..
        } => {
            if code.emission.natural_store(statement.id) {
                if let Some(receiver) = receiver {
                    push(&mut roots, *receiver)?;
                }
                push(&mut roots, *value)?;
            } else {
                push(&mut roots, *value)?;
            }
        }
        StatementKind::ArrayStore {
            array,
            index,
            value,
            ..
        } => {
            if code.emission.natural_store(statement.id) {
                push(&mut roots, *array)?;
                push(&mut roots, *index)?;
                push(&mut roots, *value)?;
            } else {
                push(&mut roots, *value)?;
            }
        }
        _ => return Ok(None),
    }
    let mut work = Vec::new();
    for root in roots.into_iter().rev() {
        push(&mut work, Step::Visit(root))?;
    }
    while let Some(step) = work.pop() {
        let id = match step {
            Step::Act => return Ok(None),
            Step::Visit(id) => id,
        };
        let Some(expression) = code.expressions.get(id.0 as usize) else {
            return Ok(None);
        };
        if code.emission.composed_use(id).is_some() {
            // The composed subtree prints under another carrier.
            return Ok(None);
        }
        match &expression.kind {
            ExprKind::Read { storage: read, .. } if *read == storage => return Ok(Some(id)),
            ExprKind::Read { .. } | ExprKind::Literal { .. } => {}
            ExprKind::Unary { value, .. } => {
                push(&mut work, Step::Act)?;
                push(&mut work, Step::Visit(*value))?;
            }
            ExprKind::Binary { left, right, .. } | ExprKind::Compare { left, right, .. } => {
                push(&mut work, Step::Act)?;
                push(&mut work, Step::Visit(*right))?;
                push(&mut work, Step::Visit(*left))?;
            }
            ExprKind::Call(call) => {
                push(&mut work, Step::Act)?;
                if let Arguments::Known(args) = &call.arguments {
                    for &argument in args.iter().rev() {
                        push(&mut work, Step::Visit(argument))?;
                    }
                }
                if let Some(receiver) = call.receiver {
                    push(&mut work, Step::Visit(receiver))?;
                }
            }
            ExprKind::Field {
                receiver: Some(receiver),
                ..
            } => {
                push(&mut work, Step::Act)?;
                push(&mut work, Step::Visit(*receiver))?;
            }
            ExprKind::Array { array, index, .. } => {
                push(&mut work, Step::Act)?;
                push(&mut work, Step::Visit(*index))?;
                push(&mut work, Step::Visit(*array))?;
            }
            ExprKind::InstanceOf { value, .. } => {
                push(&mut work, Step::Act)?;
                push(&mut work, Step::Visit(*value))?;
            }
            _ => return Ok(None),
        }
    }
    Ok(None)
}

/// Whether this statement's printed expression trees render the occurrence
/// exactly once. P7 builds one expression per operand use, but the carrier must
/// not depend on that construction rule: a shared occurrence would print the
/// check twice. A composed subtree prints under another statement's carrier, so
/// it is not a position of this carrier.
fn rendered_once(
    code: &MethodCode,
    statement: StatementId,
    occurrence: ExprId,
) -> Result<bool, ExpressionError> {
    if code
        .emission
        .statement_operation(statement)
        .is_some_and(|op| code.emission.origin_only_operation(op))
    {
        return Ok(false);
    }
    let statement = code
        .statements
        .get(statement.0 as usize)
        .ok_or(bug("check-cast carrier names an unknown statement"))?;
    let mut roots = [None; 3];
    match &statement.kind {
        StatementKind::Move { expression, .. } | StatementKind::Evaluate { expression } => {
            roots[0] = Some(*expression);
        }
        StatementKind::Initialize { expression, .. } => {
            roots[0] = Some(*expression);
        }
        StatementKind::FieldStore {
            receiver, value, ..
        } => {
            roots[0] = *receiver;
            roots[1] = Some(*value);
        }
        StatementKind::ArrayStore {
            array,
            index,
            value,
            ..
        } => {
            roots[0] = Some(*array);
            roots[1] = Some(*index);
            roots[2] = Some(*value);
        }
        _ => return Ok(false),
    }
    let mut work = Vec::new();
    for root in roots.into_iter().flatten() {
        push(&mut work, root)?;
    }
    let mut seen = 0u8;
    while let Some(id) = work.pop() {
        if id == occurrence {
            seen = seen.saturating_add(1);
            if seen > 1 {
                return Ok(false);
            }
        }
        let expression = code
            .expressions
            .get(id.0 as usize)
            .ok_or(bug("statement expression is outside its arena"))?;
        if code.emission.composed_use(id).is_some() {
            continue;
        }
        match &expression.kind {
            ExprKind::Unary { value, .. } => push(&mut work, *value)?,
            ExprKind::Binary { left, right, .. } | ExprKind::Compare { left, right, .. } => {
                push(&mut work, *left)?;
                push(&mut work, *right)?;
            }
            ExprKind::Array { array, index, .. } => {
                push(&mut work, *array)?;
                push(&mut work, *index)?;
            }
            ExprKind::Field {
                receiver: Some(receiver),
                ..
            } => push(&mut work, *receiver)?,
            ExprKind::ArrayLength(array) => push(&mut work, *array)?,
            ExprKind::InstanceOf { value, .. } => push(&mut work, *value)?,
            ExprKind::NewArray { length, .. } => push(&mut work, *length)?,
            ExprKind::Call(call) => {
                if let Some(receiver) = call.receiver {
                    push(&mut work, receiver)?;
                }
                if let Arguments::Known(args) = &call.arguments {
                    for &argument in args {
                        push(&mut work, argument)?;
                    }
                }
            }
            ExprKind::New { arguments, .. } => {
                for &argument in arguments {
                    push(&mut work, argument)?;
                }
            }
            ExprKind::FilledArray {
                elements: Arguments::Known(elements),
                ..
            } => {
                for &element in elements {
                    push(&mut work, element)?;
                }
            }
            _ => {}
        }
    }
    Ok(seen == 1)
}

fn leftmost(code: &MethodCode, expression: ExprId) -> Option<ExprId> {
    let expression = code.expressions.get(expression.0 as usize)?;
    if code.emission.composed_use(expression.id).is_some() {
        // A composed use prints another statement's expression under that
        // statement's carrier, so this read cannot carry the check here.
        return None;
    }
    match &expression.kind {
        ExprKind::Read { .. }
        | ExprKind::Snapshot { .. }
        | ExprKind::Literal { .. }
        | ExprKind::Unavailable { .. } => Some(expression.id),
        ExprKind::Unary { value, .. } => leftmost(code, *value),
        ExprKind::Binary { left, .. } | ExprKind::Compare { left, .. } => leftmost(code, *left),
        ExprKind::Array { array, .. } => leftmost(code, *array),
        ExprKind::Field { receiver, .. } => receiver.and_then(|r| leftmost(code, r)),
        ExprKind::ArrayLength(a) => leftmost(code, *a),
        ExprKind::InstanceOf { value, .. } => leftmost(code, *value),
        ExprKind::NewArray { length, .. } => leftmost(code, *length),
        ExprKind::Call(call) => match call.receiver {
            Some(receiver) => leftmost(code, receiver),
            None => match &call.arguments {
                Arguments::Known(args) => args.first().and_then(|a| leftmost(code, *a)),
                Arguments::Unavailable { .. } => None,
            },
        },
        ExprKind::New { arguments, .. } => arguments.first().and_then(|a| leftmost(code, *a)),
        _ => None,
    }
}

fn reads(code: &MethodCode, expression: ExprId, storage: StorageId) -> bool {
    matches!(
        code.expressions.get(expression.0 as usize).map(|e| &e.kind),
        Some(ExprKind::Read { storage: read, .. }) if *read == storage
    )
}
