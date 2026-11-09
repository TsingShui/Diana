//! Lowers the closed P6 coroutine-body alternative. A local marker pattern on
//! its own is not authority to change a method header or remove its ABI dispatch.
use super::build::{bug, copy, id, push};
use super::values::Context;
use super::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Suspension {
    #[serde(serialize_with = "super::debug")]
    pub original: crate::p6::SuspensionPlan,
    pub call: crate::p2::OpId,
    pub statement: StatementId,
    pub expression: ExprId,
    pub target: ReferenceId,
    pub continuation: ExprId,
    pub continuation_argument: u32,
    pub merged: StorageId,
    pub normal_copy: CopyPlanId,
    pub normal_source: ExprId,
    pub control: ControlId,
    pub header: ControlId,
    pub bodies: [ControlId; 2],
    pub predicate: crate::p2::BlockId,
    pub marker_return: crate::p2::BlockId,
    pub links: [TransferId; 2],
}
/// ABI-only source attachments remain ordinary original controls. P9 prints
/// them as comments in the same traversal, not as a second executed body.
#[derive(Debug, Serialize)]
pub struct CoroutineBody {
    pub root: ControlId,
    pub prefix: ControlId,
    pub dispatch: ControlId,
    pub initial: ControlId,
    pub resume: ControlId,
    pub invalid: ControlId,
    pub entry_check: crate::p2::OpId,
    #[serde(serialize_with = "super::debug")]
    pub operations: Vec<crate::p6::CoroutineOperation>,
}

#[derive(Debug, Serialize)]
pub struct CoroutineLoop {
    pub initial: Vec<crate::p6::NodeId>,
    pub body: Vec<crate::p6::NodeId>,
    pub tail: Vec<crate::p6::NodeId>,
    pub abi: Vec<crate::p6::NodeId>,
    pub abi_entries: Vec<crate::p6::NodeId>,
    pub test: PredicateEmission,
    pub header: crate::p6::NodeId,
    pub call: crate::p2::OpId,
    pub statement: StatementId,
    pub expression: ExprId,
    pub target: ReferenceId,
    pub continuation: ExprId,
    pub continuation_argument: u32,
    pub state: i32,
    pub forwarded_copies: Vec<CoroutineCopy>,
}
#[derive(Debug, Serialize)]
pub struct CoroutineCopy {
    pub bundle: CopyPlanId,
    pub step: u32,
    pub destination: StorageId,
    pub source: StorageId,
    pub expression: ExprId,
}

pub(super) fn construct_loop(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let Some(plan) = c.planned.facts().coroutine_loop() else {
        return Ok(());
    };
    let call = plan.call();
    let statement = c.code.operations[call.0 as usize]
        .statement
        .ok_or(bug("suspend loop call lost its statement"))?;
    let StatementKind::Evaluate { expression } = c.code.statements[statement.0 as usize].kind
    else {
        return Err(bug("suspend loop call lost its expression"));
    };
    let ExprKind::Call(source) = &c.code.expressions[expression.0 as usize].kind else {
        return Ok(());
    };
    let Arguments::Known(arguments) = &source.arguments else {
        return Ok(());
    };
    let argument = plan
        .continuation_argument()
        .checked_sub(u32::from(source.receiver.is_some()))
        .ok_or(bug("suspend loop continuation precedes receiver"))?;
    let continuation = *arguments
        .get(argument as usize)
        .ok_or(bug("suspend loop call lost its continuation"))?;
    let condition = plan.condition();
    let terminal = c.planned.facts().nodes()[condition.node().0 as usize].block();
    let TerminalKind::If {
        when_true,
        when_false,
        ..
    } = c.code.terminals[terminal.0 as usize].kind
    else {
        return Err(bug("suspend loop lost its head predicate"));
    };
    let owned = CoroutineLoop {
        initial: copy(plan.initial())?,
        body: copy(plan.body())?,
        tail: copy(plan.tail())?,
        abi: copy(plan.abi())?,
        abi_entries: copy(plan.abi_entries())?,
        test: PredicateEmission {
            terminal,
            when_true,
            when_false,
            negate: when_false == TransferId(condition.repeat().0),
        },
        header: condition.node(),
        call,
        statement,
        expression,
        target: source.target,
        continuation,
        continuation_argument: argument,
        state: plan.state(),
        forwarded_copies: Vec::new(),
    };
    for &op in plan.protocol_operations() {
        let node = c.op_nodes[op.0 as usize].ok_or(bug("suspend loop protocol lost its node"))?;
        for action in &mut c.code.emission.blocks[node.0 as usize].actions {
            if let BodyAction::Statement {
                operation,
                statement,
            } = *action
                && operation == op
            {
                *action = BodyAction::AbiOperation {
                    operation,
                    statement,
                };
            }
        }
    }
    c.code.emission.coroutine_loop = Some(owned);
    Ok(())
}

/// Finalize source placement after ordinary expression choices. The P6 list is
/// the sole authority for fallthrough; no P7 reachability/loop solver runs here.
pub(super) fn finish_loop(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let Some(plan) = c.planned.facts().coroutine_loop() else {
        return Ok(());
    };
    if c.code.emission.coroutine_loop.is_none() {
        return Ok(());
    }
    for &node in plan.abi() {
        let original = &c.planned.facts().nodes()[node.0 as usize];
        if original.has_terminal() {
            c.code.emission.blocks[node.0 as usize].end = BlockEnd::Terminal(original.block());
        }
    }
    for &node in plan
        .abi()
        .iter()
        .chain(plan.initial())
        .chain(plan.body())
        .chain(plan.tail())
    {
        if !matches!(c.code.emission.controls[node.0 as usize].layout, EmissionLayout::Block(n) if n == node)
        {
            return Err(bug("suspend loop lost its original node control"));
        }
        c.code.emission.controls[node.0 as usize].continuation = None;
    }
    for &link in plan.normal_links() {
        if !plan
            .abi()
            .contains(&c.planned.facts().links()[link.0 as usize].from())
        {
            c.code.emission.transfers[link.0 as usize].spelling = TransferSpelling::Fallthrough;
        }
    }
    for &node in plan.initial().iter().chain(plan.body()).chain(plan.tail()) {
        c.code.emission.blocks[node.0 as usize].show_label = false;
    }
    // A declaration remains visible whenever executable code uses that storage.
    // ABI attachments still own their original definitions and reads.
    let mut visible = super::build::filled(c.code.locals.len(), false)?;
    for &node in plan.initial().iter().chain(plan.body()).chain(plan.tail()) {
        let original = &c.planned.facts().nodes()[node.0 as usize];
        for index in original.operations() {
            let op = crate::p2::OpId(index);
            if plan.protocol_operations().contains(&op) {
                continue;
            }
            let source = &c.code.operations[index as usize];
            if source.statement.is_none() {
                continue;
            }
            if let Some(id) = source.statement {
                for output in &c.code.statements[id.0 as usize].outputs {
                    if let crate::p5::Materialization::Store(local) = output.materialization() {
                        visible[local.0 as usize] = true;
                    }
                }
            }
            if let Some(Ok(uses)) = c.planned.storage_facts().operation_uses(op) {
                for (argument, use_) in uses.iter().enumerate() {
                    if op == plan.call() && argument == plan.continuation_argument() as usize {
                        continue;
                    }
                    if let Some(local) = use_.read().ok().and_then(crate::p5::StorageRead::local) {
                        visible[local.0 as usize] = true;
                    }
                }
            }
        }
        if original.has_terminal()
            && let Some(uses) = c.planned.storage_facts().terminal_uses(original.block())
        {
            for use_ in uses {
                if let Some(local) = use_.read().ok().and_then(crate::p5::StorageRead::local) {
                    visible[local.0 as usize] = true;
                }
            }
        }
    }
    let condition = plan.condition();
    c.code.emission.blocks[condition.node().0 as usize].end = BlockEnd::None;
    c.code.emission.blocks[condition.node().0 as usize].show_label = false;
    if let Some(uses) = c
        .planned
        .storage_facts()
        .terminal_uses(c.planned.facts().nodes()[condition.node().0 as usize].block())
    {
        for use_ in uses {
            if let Some(local) = use_.read().ok().and_then(crate::p5::StorageRead::local) {
                visible[local.0 as usize] = true;
            }
        }
    }
    for &link in plan.normal_links() {
        for &bundle in &c.code.emission.transfers[link.0 as usize].copies {
            for step in &c.code.copies[bundle.0 as usize].steps {
                if let CopyStatement::Assign {
                    destination,
                    expression,
                    ..
                } = *step
                {
                    visible[destination.0 as usize] = true;
                    if let ExprKind::Read { storage, .. } =
                        c.code.expressions[expression.0 as usize].kind
                    {
                        visible[storage.0 as usize] = true;
                    }
                }
            }
        }
    }
    for (index, used) in visible.into_iter().enumerate() {
        if !used {
            c.code.emission.hidden_locals[index] = true;
        }
    }
    let mut forwarded = Vec::new();
    for &link in plan.normal_links() {
        if !plan
            .body()
            .contains(&c.planned.facts().links()[link.0 as usize].from())
        {
            continue;
        }
        for &bundle in &c.code.emission.transfers[link.0 as usize].copies {
            if !c.code.emission.copy_check_uses[bundle.0 as usize].is_empty() {
                continue;
            }
            for (step, copy) in c.code.copies[bundle.0 as usize].steps.iter().enumerate() {
                let CopyStatement::Assign {
                    destination,
                    expression,
                    ..
                } = *copy
                else {
                    continue;
                };
                let ExprKind::Read {
                    storage: source,
                    view: crate::p4::Interpretation::Direct(_),
                    ..
                } = c.code.expressions[expression.0 as usize].kind
                else {
                    continue;
                };
                if source == destination
                    || forwarded.iter().any(|p: &CoroutineCopy| {
                        p.destination == destination
                            || p.destination == source
                            || p.source == destination
                    })
                {
                    continue;
                }
                if forwardable(c, plan, link, destination, source) {
                    push(
                        &mut forwarded,
                        CoroutineCopy {
                            bundle,
                            step: id(step)?,
                            destination,
                            source,
                            expression,
                        },
                    )?;
                }
            }
        }
    }
    for copy in &forwarded {
        c.code.emission.hidden_locals[copy.destination.0 as usize] = true;
    }
    c.code
        .emission
        .coroutine_loop
        .as_mut()
        .ok_or(bug("suspend loop lost its selected body"))?
        .forwarded_copies = forwarded;
    Ok(())
}

fn forwardable(
    c: &Context<'_, '_>,
    plan: &crate::p6::CoroutineLoopPlan,
    copied_at: crate::p6::LinkId,
    destination: StorageId,
    source: StorageId,
) -> bool {
    let reads = |uses: &[crate::p5::UseAccess]| {
        uses.iter().any(|u| {
            u.read().ok().and_then(crate::p5::StorageRead::local)
                == Some(crate::p5::LocalId(destination.0))
        })
    };
    for &node in plan
        .initial()
        .iter()
        .chain(plan.tail())
        .chain(std::iter::once(&plan.condition().node()))
    {
        let original = &c.planned.facts().nodes()[node.0 as usize];
        for index in original.operations() {
            if c.planned
                .storage_facts()
                .operation_uses(crate::p2::OpId(index))
                .and_then(Result::ok)
                .is_none_or(reads)
            {
                return false;
            }
        }
        if original.has_terminal()
            && c.planned
                .storage_facts()
                .terminal_uses(original.block())
                .is_none_or(reads)
        {
            return false;
        }
    }
    let mut ready = false;
    let mut used = false;
    for &node in plan.body() {
        let original = &c.planned.facts().nodes()[node.0 as usize];
        for index in original.operations() {
            let operation = crate::p2::OpId(index);
            if plan.protocol_operations().contains(&operation) {
                continue;
            }
            let Some(Ok(uses)) = c.planned.storage_facts().operation_uses(operation) else {
                return false;
            };
            if reads(uses) {
                if !ready {
                    return false;
                }
                used = true;
            }
            let Some(definitions) = c.planned.storage_facts().operation_definitions(operation)
            else {
                return false;
            };
            for definition in definitions {
                if definition.materialization()
                    == crate::p5::Materialization::Store(crate::p5::LocalId(destination.0))
                {
                    return false;
                }
                if definition.materialization()
                    == crate::p5::Materialization::Store(crate::p5::LocalId(source.0))
                {
                    ready = false;
                }
            }
        }
        if original.has_terminal()
            && c.planned
                .storage_facts()
                .terminal_uses(original.block())
                .is_some_and(reads)
        {
            if !ready {
                return false;
            }
            used = true;
        }
        for &link in &c.code.blocks[node.0 as usize].transfers {
            for &bundle in &c.code.emission.transfers[link.0 as usize].copies {
                for step in &c.code.copies[bundle.0 as usize].steps {
                    match *step {
                        CopyStatement::Assign {
                            destination: to,
                            expression,
                            ..
                        } => {
                            if let ExprKind::Read { storage, .. } =
                                c.code.expressions[expression.0 as usize].kind
                                && storage == destination
                            {
                                if !ready {
                                    return false;
                                }
                                used = true;
                            }
                            if to == source {
                                ready = false;
                            }
                            if to == destination {
                                if link.0 != copied_at.0 {
                                    return false;
                                }
                                ready = true;
                            }
                        }
                        CopyStatement::Save { .. } => return false,
                    }
                }
            }
        }
    }
    used
}

pub(super) fn construct(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let Some(body) = c.planned.facts().coroutine_body() else {
        return Ok(());
    };
    let original = *c
        .planned
        .facts()
        .suspensions()
        .first()
        .ok_or(bug("closed coroutine body lost its suspension"))?;
    let call = original.call();
    let statement = c.code.operations[call.0 as usize]
        .statement
        .ok_or(bug("suspension call lost its statement"))?;
    let StatementKind::Evaluate { expression } = c.code.statements[statement.0 as usize].kind
    else {
        return Err(bug("suspension call lost its evaluate expression"));
    };
    let ExprKind::Call(source) = &c.code.expressions[expression.0 as usize].kind else {
        return Err(bug("suspension call lost its invoke expression"));
    };
    let Arguments::Known(arguments) = &source.arguments else {
        return Err(bug("suspension lost its known arguments"));
    };
    let argument = original
        .continuation_argument()
        .checked_sub(u32::from(source.receiver.is_some()))
        .ok_or(bug("suspension continuation precedes receiver"))?;
    let continuation = *arguments
        .get(argument as usize)
        .ok_or(bug("suspension lost its continuation occurrence"))?;
    let normal_copy = CopyPlanId(original.normal_copy().0);
    let [
        CopyStatement::Assign {
            expression: normal_source,
            ..
        },
    ] = c.code.copies[normal_copy.0 as usize].steps.as_slice()
    else {
        return Err(bug("suspension lost its normal result copy"));
    };
    let control = ControlId(original.control().0);
    let suspension = Suspension {
        original,
        call,
        statement,
        expression,
        target: source.target,
        continuation,
        continuation_argument: argument,
        merged: StorageId(original.merged().0),
        normal_copy,
        normal_source: *normal_source,
        control,
        header: ControlId(original.header().0),
        bodies: original.bodies().map(|r| ControlId(r.0)),
        predicate: original.predicate(),
        marker_return: original.marker_return(),
        links: original.links().map(|r| TransferId(r.0)),
    };
    let mut sequence = Vec::new();
    for &region in body.sequence() {
        push(&mut sequence, ControlId(region.0))?;
    }
    let owned = CoroutineBody {
        root: ControlId(body.root().0),
        prefix: ControlId(body.prefix().0),
        dispatch: ControlId(body.dispatch().0),
        initial: ControlId(body.initial().0),
        resume: ControlId(body.resume().0),
        invalid: ControlId(body.invalid().0),
        entry_check: body.entry_check(),
        operations: copy(body.operations())?,
    };
    // Keep the state-publication event as a display action and an adjacency
    // barrier. Hiding its text must not let an effectful receiver/argument
    // producer move from before this field write into the suspension call.
    let node = c.op_nodes[original.state_store().0 as usize]
        .ok_or(bug("coroutine state publication lost its original node"))?;
    let action=c.code.emission.blocks[node.0 as usize].actions.iter_mut().find(|action|
        matches!(action,BodyAction::Statement{operation,..} if *operation==original.state_store()))
        .ok_or(bug("coroutine state publication lost its original action"))?;
    let BodyAction::Statement {
        operation,
        statement,
    } = *action
    else {
        return Err(bug("coroutine state action changed kind"));
    };
    let implicit = BodyAction::AbiOperation {
        operation,
        statement,
    };
    let index = id(c.code.emission.suspensions.len())?;
    push(&mut c.code.emission.suspensions, suspension)?;
    *action = implicit;
    c.code.emission.controls[control.0 as usize].layout =
        EmissionLayout::Suspension { plan: index };
    c.code.emission.controls[owned.root.0 as usize].explicit_children = Some(sequence);
    for &local in body.implicit_locals() {
        c.code.emission.hidden_locals[local.0 as usize] = true;
    }
    c.code.emission.coroutine_body = Some(owned);
    Ok(())
}
