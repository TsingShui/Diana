//! Lowers P6's established protocol associations over the ordinary control arena.
//! No body/loop/flow execution program or downstream protocol rematching exists.
use super::build::{bug, copy, filled, id, push};
use super::values::Context;
use super::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Suspension {
    pub call: crate::p2::OpId,
    pub statement: StatementId,
    pub expression: ExprId,
    pub target: ReferenceId,
    pub continuation: ExprId,
    pub continuation_argument: u32,
    pub state: i32,
}
/// Protocol associations supplement ordinary selected controls and operations.
#[derive(Debug, Serialize)]
pub struct Protocol {
    pub calls: Vec<Suspension>,
    pub abi_entries: Vec<crate::p6::NodeId>,
    pub forwarded_copies: Vec<ProtocolCopy>,
}
#[derive(Debug, Serialize)]
pub struct ProtocolCopy {
    pub bundle: CopyPlanId,
    pub step: u32,
    pub destination: StorageId,
    pub source: StorageId,
    pub expression: ExprId,
}

pub(super) fn construct(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let f = c.planned.facts();
    let Some(plan) = f.protocol() else {
        return Ok(());
    };
    let mut calls = Vec::new();
    for original in &plan.calls {
        let operation = &c.code.operations[original.call.0 as usize];
        let statement = operation
            .statement
            .ok_or(bug("protocol call lost its statement"))?;
        let StatementKind::Evaluate { expression } = c.code.require_statement(statement)?.kind
        else {
            return Err(bug("protocol call lost its evaluation"));
        };
        let ExprKind::Call(source) = &c.code.expressions[expression.0 as usize].kind else {
            return Err(bug("protocol call lost its established invoke"));
        };
        let Arguments::Known(arguments) = &source.arguments else {
            return Err(bug("protocol call lost its established arguments"));
        };
        let argument = original
            .continuation_argument
            .checked_sub(u32::from(source.receiver.is_some()))
            .ok_or(bug("protocol continuation precedes receiver"))?;
        let continuation = *arguments
            .get(argument as usize)
            .ok_or(bug("protocol call lost continuation"))?;
        push(
            &mut calls,
            Suspension {
                call: original.call,
                statement,
                expression,
                target: source.target,
                continuation,
                continuation_argument: argument,
                state: original.state,
            },
        )?;
        if let Some((bundle, local)) = original.publication {
            let [
                CopyStatement::Assign {
                    expression: source,
                    destination,
                    ..
                },
            ] = c.code.copies[bundle.0 as usize].steps.as_slice()
            else {
                return Err(bug("protocol publication lost its established assignment"));
            };
            if destination.0 != local.0 {
                return Err(bug("protocol publication changed destination"));
            }
            c.code.operations[original.call.0 as usize].publication = Some(ResultPublication {
                destination: StorageId(local.0),
                copy: CopyPlanId(bundle.0),
                transfer: TransferId(original.normal.0),
                source: *source,
            });
        }
    }
    // ABI text remains an operation action: state publication is still an
    // execution-order barrier before expression covering selects its operands.
    for &op in &plan.operations {
        let node = c.op_nodes[op.0 as usize].ok_or(bug("protocol event lost its portion"))?;
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
    // Generic raw switch placement may precede selection of the protocol
    // partition. Consume the supplied edge spellings after that placement.
    for link in f.link_plans() {
        let id = link.link().0 as usize;
        match link.action() {
            crate::p6::TransferAction::Next(_) => {
                c.code.emission.transfers[id].spelling = TransferSpelling::Fallthrough
            }
            crate::p6::TransferAction::Goto(label) => {
                c.code.emission.transfers[id].spelling = TransferSpelling::Goto(LabelId(label.0))
            }
            _ => {}
        }
    }
    for record in f.controls() {
        let EmissionLayout::Block(node) = record.layout else {
            continue;
        };
        let Some(end) = &record.end else {
            continue;
        };
        c.code.emission.blocks[node.0 as usize].end = match *end {
            crate::recovery::BodyEnd::None => BlockEnd::None,
            crate::recovery::BodyEnd::Terminal(terminal) => BlockEnd::Terminal(terminal),
            crate::recovery::BodyEnd::Annotated {
                terminal,
                mode,
                normal,
                suspended,
            } => BlockEnd::Annotated {
                terminal,
                mode,
                normal: TransferId(normal.0),
                suspended: TransferId(suspended.0),
            },
        };
    }
    c.code.emission.protocol = Some(Protocol {
        calls,
        abi_entries: copy(&plan.abi_entries)?,
        forwarded_copies: Vec::new(),
    });
    Ok(())
}

pub(super) fn finish(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let Some(plan) = c.planned.facts().protocol() else {
        return Ok(());
    };
    let visible = visible_storage(
        c.code,
        plan.source_nodes.iter().copied(),
        plan.source_nodes
            .iter()
            .flat_map(|node| c.code.blocks[node.0 as usize].transfers.iter().copied()),
        &plan.operations,
    )?;
    for (index, used) in visible.into_iter().enumerate() {
        if !used {
            c.code.emission.hidden_locals[index] = true;
        }
    }
    for &local in &plan.hidden_locals {
        c.code.emission.hidden_locals[local.0 as usize] = true;
    }
    // Existing interval correspondence still owns protocol aliases. This is
    // storage metadata, not another executable loop or a new reachability pass.
    if let Some(loop_plan) = c.planned.facts().coroutine_loop() {
        let mut forwarded = Vec::new();
        for &link in loop_plan.normal_links() {
            if !loop_plan
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
                        || forwarded.iter().any(|p: &ProtocolCopy| {
                            p.destination == destination
                                || p.destination == source
                                || p.source == destination
                        })
                    {
                        continue;
                    }
                    if forwardable(c, loop_plan, link, destination, source) {
                        push(
                            &mut forwarded,
                            ProtocolCopy {
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
            .protocol
            .as_mut()
            .ok_or(bug("protocol lost associations"))?
            .forwarded_copies = forwarded;
    }
    // Origin associations refer to unused original containers individually;
    // their children are owned elsewhere and must not execute a second time.
    let mut active = filled(c.code.controls.len(), false)?;
    let mut work = Vec::new();
    if let Some(root) = c.code.root {
        push(&mut work, root)?;
    }
    while let Some(id) = work.pop() {
        if active[id.0 as usize] {
            continue;
        }
        active[id.0 as usize] = true;
        let control = &c.code.controls[id.0 as usize];
        for &child in control.children.iter().chain(&control.source_only) {
            push(&mut work, child)?;
        }
    }
    for (index, used) in active.into_iter().enumerate() {
        if !used {
            push(&mut c.code.emission.control_origins, ControlId(id(index)?))?;
        }
    }
    Ok(())
}

/// Source/ABI visibility consumes owned P7 reads, never another P5-use scan.
/// Selected copies and construction operands keep their original storage facts.
fn visible_storage(
    code: &MethodCode,
    nodes: impl Iterator<Item = crate::p6::NodeId>,
    transfers: impl Iterator<Item = TransferId>,
    protocol: &[crate::p2::OpId],
) -> Result<Vec<bool>, ExpressionError> {
    let mut visible = super::build::filled(code.locals.len(), false)?;
    let mut mark = |source: Option<crate::p5::LocalId>| {
        if let Some(local) = source {
            visible[local.0 as usize] = true;
        }
    };
    for node in nodes {
        let block = &code.blocks[node.0 as usize];
        for index in block.original_operations.clone() {
            let source = &code.operations[index as usize];
            if protocol.contains(&source.source) {
                continue;
            }
            let Some(id) = source.statement else { continue };
            let statement = code.require_statement(id)?;
            let implicit = match statement.kind {
                StatementKind::Evaluate { expression } => code
                    .emission
                    .implicit_continuation(expression)
                    .map(|(_, id)| id),
                _ => None,
            };
            if let Arguments::Known(inputs) = &statement.inputs {
                for &expression in inputs {
                    if implicit != Some(expression)
                        && let Some(Ok(source)) = code.expressions[expression.0 as usize]
                            .origin
                            .as_ref()
                            .map(|origin| origin.read)
                    {
                        mark(source.local());
                    }
                }
            }
            if let StatementKind::Construction(group) = statement.kind {
                for &source in &code.constructions[group.0 as usize].arguments {
                    mark(source.local());
                }
            }
            for output in &statement.outputs {
                if let crate::p5::Materialization::Store(local) = output.materialization() {
                    mark(Some(local));
                }
            }
        }
        if !matches!(block.completion, BlockCompletion::None)
            && !matches!(
                code.emission.blocks[node.0 as usize].end,
                BlockEnd::Annotated { .. }
            )
        {
            for &expression in &code.terminals[block.original_block.0 as usize].inputs {
                if let Some(Ok(source)) = code.expressions[expression.0 as usize]
                    .origin
                    .as_ref()
                    .map(|origin| origin.read)
                {
                    mark(source.local());
                }
            }
        }
    }
    for transfer in transfers {
        for &bundle in &code.emission.transfers[transfer.0 as usize].copies {
            for step in &code.copies[bundle.0 as usize].steps {
                match *step {
                    CopyStatement::Assign {
                        destination,
                        original,
                        ..
                    } => {
                        visible[destination.0 as usize] = true;
                        if let Some(source) = original.local() {
                            visible[source.0 as usize] = true;
                        }
                    }
                    CopyStatement::Save {
                        destination,
                        source,
                    } => {
                        visible[destination.0 as usize] = true;
                        visible[source.0 as usize] = true;
                    }
                }
            }
        }
    }
    Ok(visible)
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
