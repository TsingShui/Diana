//! Display shapes over completed child schedules. Original arm ownership and
//! predicate-edge identities remain unchanged, including for an inverted If.

use super::super::build::{filled, push};
use super::super::*;

pub(super) fn construct(code: &mut MethodCode) -> Result<(), ExpressionError> {
    let mut empty = filled(code.controls.len(), false)?;
    let mut single_if = filled(code.controls.len(), false)?;
    // One fact per original branch terminal: both arms print nothing, only the
    // false arm prints nothing, only the true arm prints nothing. `ifs` then
    // selects the single printed form from those facts.
    let mut both_silent = filled(code.terminals.len(), false)?;
    let mut false_silent = filled(code.terminals.len(), false)?;
    let mut true_silent = filled(code.terminals.len(), false)?;
    let mut work = Vec::new();
    if let Some(root) = code.root {
        push(&mut work, (root, false))?;
    }
    while let Some((id, visited)) = work.pop() {
        if !visited {
            push(&mut work, (id, true))?;
            for &child in
                code.emission.controls[id.0 as usize].children(&code.controls[id.0 as usize])
            {
                push(&mut work, (child, false))?;
            }
            continue;
        }
        // Inner display choices must be complete before the containing arm is
        // classified: an empty If can itself empty a surrounding Sequence.
        if let EmissionLayout::If { yes, no, test, .. } =
            code.emission.controls[id.0 as usize].layout
        {
            let yes_empty = empty[yes.0 as usize] && copies_print_nothing(code, test.when_true);
            let no_empty = empty[no.0 as usize] && arm_prints_nothing(code, test.when_false, true);
            if no_empty {
                code.emission.controls[no.0 as usize].omitted = true;
            }
            let both_empty =
                yes_empty && empty[no.0 as usize] && copies_print_nothing(code, test.when_false);
            let control = &code.emission.controls[id.0 as usize];
            let shape = if both_empty
                && control.continuation.is_none()
                && !code.emission.deferred_control(id)
                && discardable_predicate(code, test)
            {
                IfShape::Empty
            } else if yes_empty && !no_empty && !displayed_predicate_site(code, test) {
                IfShape::Inverted
            } else if !no_empty
                && arm_prints_nothing(code, test.when_false, true)
                && single_if[no.0 as usize]
            {
                IfShape::ElseIf
            } else {
                IfShape::IfElse
            };
            if let EmissionLayout::If {
                shape: selected,
                test,
                ..
            } = &mut code.emission.controls[id.0 as usize].layout
            {
                *selected = shape;
                if matches!(shape, IfShape::Inverted) {
                    test.negate = !test.negate;
                }
            }
        }
        let control = &code.emission.controls[id.0 as usize];
        if control.label_required
            || control.continuation.is_some()
            || code.emission.deferred_control(id)
        {
            continue;
        }
        match &control.layout {
            EmissionLayout::Block(node) => {
                let block = &code.emission.blocks[node.0 as usize];
                if let BlockEnd::Terminal(terminal) = block.end
                    && let TerminalKind::If {
                        when_true,
                        when_false,
                        ..
                    } = code.terminals[terminal.0 as usize].kind
                {
                    both_silent[terminal.0 as usize] =
                        [when_true, when_false].iter().all(|&edge| {
                            matches!(
                                code.emission.transfers[edge.0 as usize].spelling,
                                TransferSpelling::Fallthrough
                            ) && copies_print_nothing(code, edge)
                        }) && discardable_predicate(
                            code,
                            PredicateEmission {
                                terminal,
                                when_true,
                                when_false,
                                negate: false,
                            },
                        );
                }
                // A `nop` prints no text, so a block whose actions are all
                // origin-only still prints nothing.
                let silent_actions = block.actions.iter().all(|action| match action {
                    BodyAction::Statement {
                        operation,
                        statement,
                    } => {
                        code.emission.omitted_nop(*statement)
                            || code.emission.origin_only_operation(*operation)
                    }
                    BodyAction::Release { .. }
                    | BodyAction::Receive { .. }
                    | BodyAction::AbiOperation { .. } => false,
                });
                empty[id.0 as usize] =
                    silent_actions && !block.show_label && display_empty_end(code, &block.end);
            }
            EmissionLayout::Sequence => {
                empty[id.0 as usize] = true;
                for &child in
                    code.emission.controls[id.0 as usize].children(&code.controls[id.0 as usize])
                {
                    if empty[child.0 as usize] {
                        continue;
                    }
                    if !empty[id.0 as usize] {
                        single_if[id.0 as usize] = false;
                        break;
                    }
                    empty[id.0 as usize] = false;
                    single_if[id.0 as usize] = single_if[child.0 as usize];
                }
            }
            EmissionLayout::If {
                header,
                test,
                shape,
                ..
            } => {
                if matches!(shape, IfShape::Empty) {
                    empty[id.0 as usize] = empty[header.0 as usize];
                } else if empty[header.0 as usize]
                    && code.emission.terminals[test.terminal.0 as usize]
                        .before
                        .is_empty()
                {
                    // Nothing may print between `else ` and the inner `if`.
                    single_if[id.0 as usize] = true;
                }
            }
            _ => {}
        }
    }
    for (index, silent) in false_silent.iter_mut().enumerate() {
        if let TerminalKind::If { when_false, .. } = code.terminals[index].kind {
            let phases = &code.emission.terminals[index];
            *silent = phases.before.is_empty()
                && phases.snapshots.is_empty()
                && phases.after.is_empty()
                && arm_prints_nothing(code, when_false, false);
        }
    }
    // A plain conditional terminal has no If control: its arms are the selected
    // transfer spellings. An empty true arm and a printing false arm invert the
    // same way, and the predicate still evaluates once at the same position.
    let mut owned = vec![false; code.terminals.len()];
    for control in &code.emission.controls {
        if let EmissionLayout::If { test, .. } = &control.layout {
            owned[test.terminal.0 as usize] = true;
        }
    }
    for (index, terminal) in code.terminals.iter().enumerate() {
        let TerminalKind::If {
            when_true,
            when_false,
            ..
        } = terminal.kind
        else {
            continue;
        };
        if owned[index] || both_silent[index] {
            continue;
        }
        let phases = &code.emission.terminals[index];
        if !phases.before.is_empty() || !phases.snapshots.is_empty() || !phases.after.is_empty() {
            continue;
        }
        if arm_prints_nothing(code, when_true, false)
            && !arm_prints_nothing(code, when_false, false)
        {
            let test = PredicateEmission {
                terminal: crate::p2::BlockId(
                    u32::try_from(index).map_err(|_| crate::RequestError::PlatformCapacity)?,
                ),
                when_true,
                when_false,
                negate: false,
            };
            if !displayed_predicate_site(code, test) {
                true_silent[index] = true;
            }
        }
    }
    // An arm that prints nothing is a selected control, not a fallthrough edge,
    // so the empty-else decision above cannot see it. The control tree can: its
    // printed body is exactly the child control's text.
    for control in &code.emission.controls {
        let EmissionLayout::If {
            test,
            yes,
            no,
            shape: IfShape::IfElse,
            ..
        } = &control.layout
        else {
            continue;
        };
        let terminal = test.terminal.0 as usize;
        if both_silent[terminal] {
            continue;
        }
        let phases = &code.emission.terminals[terminal];
        let moves_nothing =
            phases.before.is_empty() && phases.snapshots.is_empty() && phases.after.is_empty();
        if empty[no.0 as usize] {
            false_silent[terminal] = true;
        } else if moves_nothing
            && empty[yes.0 as usize]
            && arm_prints_nothing(code, test.when_true, false)
            && !displayed_predicate_site(code, *test)
        {
            // `if (c) { } else { X }` prints the same behavior as `if (!c) { X }`,
            // and the predicate is still evaluated once, in the same place.
            true_silent[terminal] = true;
        }
    }
    for index in 0..code.terminals.len() {
        if !matches!(code.terminals[index].kind, TerminalKind::If { .. }) {
            continue;
        }
        code.emission.ifs[index] = if both_silent[index] {
            IfDisplay::Omitted
        } else if false_silent[index] {
            IfDisplay::NoElse
        } else if true_silent[index] {
            IfDisplay::Inverted
        } else {
            IfDisplay::Both
        };
    }
    Ok(())
}

fn displayed_predicate_site(code: &MethodCode, test: PredicateEmission) -> bool {
    let terminal = &code.terminals[test.terminal.0 as usize];
    if terminal
        .site
        .is_some_and(|site| code.emission.site_annotation(site))
    {
        return true;
    }
    let displayed_operand_site = |operand| {
        code.emission
            .composed_use(operand)
            .and_then(|value| value.site)
            .is_some_and(|site| code.emission.site_annotation(site))
    };
    match terminal.kind {
        TerminalKind::If {
            condition: Condition::Pair { left, right, .. },
            ..
        } => displayed_operand_site(left) || displayed_operand_site(right),
        TerminalKind::If {
            condition: Condition::Zero { value, .. },
            ..
        } => displayed_operand_site(value),
        _ => true,
    }
}

fn discardable_predicate(code: &MethodCode, test: PredicateEmission) -> bool {
    let phases = &code.emission.terminals[test.terminal.0 as usize];
    if !phases.before.is_empty()
        || !phases.snapshots.is_empty()
        || !phases.after.is_empty()
        || displayed_predicate_site(code, test)
    {
        return false;
    }
    // A composed operand can own an original evaluation/exception site or a
    // deferred source span. Keep that evaluation even when both arms are empty.
    match code.terminals[test.terminal.0 as usize].kind {
        TerminalKind::If {
            condition: Condition::Pair { left, right, .. },
            ..
        } => {
            code.emission.composed_use(left).is_none()
                && code.emission.composed_use(right).is_none()
        }
        TerminalKind::If {
            condition: Condition::Zero { value, .. },
            ..
        } => code.emission.composed_use(value).is_none(),
        _ => false,
    }
}

/// A zero-step bundle still owns its original edge and a zero-length source
/// anchor. Unavailable assignments print even when there are no executable steps.
pub(super) fn copies_print_nothing(code: &MethodCode, edge: TransferId) -> bool {
    code.emission.transfers[edge.0 as usize]
        .copies
        .iter()
        .all(|&id| {
            let bundle = &code.copies[id.0 as usize];
            bundle.steps.is_empty() && bundle.assignments.iter().all(|copy| copy.issue().is_none())
        })
}

/// A syntax-owned If arm prints only its copies; an ordinary terminal arm
/// additionally prints its transfer spelling.
fn arm_prints_nothing(code: &MethodCode, id: TransferId, syntax_owned: bool) -> bool {
    let transfer = &code.emission.transfers[id.0 as usize];
    copies_print_nothing(code, id)
        && (syntax_owned || matches!(transfer.spelling, TransferSpelling::Fallthrough))
}

fn display_empty_end(code: &MethodCode, end: &BlockEnd) -> bool {
    match end {
        BlockEnd::None => true,
        BlockEnd::Transfers(transfers) => transfers
            .iter()
            .all(|&id| arm_prints_nothing(code, id, false)),
        BlockEnd::Terminal(id) if code.emission.ifs[id.0 as usize] == IfDisplay::Omitted => true,
        BlockEnd::Terminal(id) => {
            let phases = &code.emission.terminals[id.0 as usize];
            phases.before.is_empty()
                && phases.snapshots.is_empty()
                && phases.after.is_empty()
                && matches!(code.terminals[id.0 as usize].kind,
                    TerminalKind::Goto(transfer) if arm_prints_nothing(code, transfer, false))
        }
    }
}
