use super::build::{bug, copy, id, push};
use super::values::Context;
use super::*;
use crate::p5::{CopyLocation, CopyStep, StorageRead};
use crate::p6::CopyOwner;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyEmission {
    Original,
    Construction(ConstructionId),
    SyncExit(ControlId),
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyStatement {
    Save {
        destination: StorageId,
        source: StorageId,
    },
    Assign {
        copy: u32,
        destination: StorageId,
        expression: ExprId,
        #[serde(serialize_with = "super::debug")]
        original: StorageRead,
    },
}
#[derive(Debug, Serialize)]
pub struct CopyBundle {
    pub id: CopyPlanId,
    #[serde(serialize_with = "super::debug")]
    pub location: CopyLocation,
    pub emission: CopyEmission,
    pub steps: Vec<CopyStatement>,
    /// Requirements remain distinct from executable steps; unavailable
    /// components have no invented assignment or default value.
    #[serde(serialize_with = "super::debug")]
    pub assignments: Vec<crate::p5::CopyAssignment>,
}
#[derive(Debug, Serialize)]
pub struct Publication {
    #[serde(serialize_with = "super::debug")]
    pub original: crate::p5::Publication,
    pub destination: StorageId,
}
#[derive(Debug, Serialize)]
pub struct PreparedExpression {
    pub local: StorageId,
    #[serde(serialize_with = "super::debug")]
    pub operation: crate::p2::OpId,
    pub statement: StatementId,
    pub expression: ExprId,
    pub operand: ExprId,
}
#[derive(Debug, Serialize)]
pub struct Construction {
    pub id: ConstructionId,
    pub statement: StatementId,
    pub expression: ExprId,
    #[serde(serialize_with = "super::debug")]
    pub allocation: crate::p2::OpId,
    #[serde(serialize_with = "super::debug")]
    pub initialization: crate::p2::OpId,
    #[serde(serialize_with = "super::debug")]
    pub normal: crate::p3::PointId,
    #[serde(serialize_with = "super::debug")]
    pub members: Vec<crate::p2::OpId>,
    #[serde(serialize_with = "super::debug")]
    pub interior_edges: Vec<crate::p2::EdgeId>,
    #[serde(serialize_with = "super::debug")]
    pub arguments: Vec<StorageRead>,
    pub prepared_arguments: Vec<PreparedExpression>,
    pub publications: Vec<Publication>,
    pub prelude: Option<CopyPlanId>,
    pub snapshot_members: Vec<CopyPlanId>,
}
pub(super) fn construct(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    for p in c.planned.storage_facts().copy_plans() {
        let owner = c
            .planned
            .facts()
            .copy_placements()
            .get(p.id().0 as usize)
            .ok_or(bug("copy lost its P6 emission owner"))?;
        let emission = match owner.owner {
            CopyOwner::Original => CopyEmission::Original,
            CopyOwner::Construction(g) => CopyEmission::Construction(ConstructionId(g.0)),
            CopyOwner::SynchronizedExit(s) => CopyEmission::SyncExit(
                c.syncs[s.0 as usize].ok_or(bug("copy names an unselected synchronized scope"))?,
            ),
        };
        let mut steps = Vec::new();
        for step in p.steps() {
            push(
                &mut steps,
                match *step {
                    CopyStep::Save {
                        destination,
                        source,
                    } => CopyStatement::Save {
                        destination: StorageId(destination.0),
                        source: StorageId(source.0),
                    },
                    CopyStep::Assign {
                        copy,
                        destination,
                        source,
                    } => CopyStatement::Assign {
                        copy: copy.0,
                        destination: StorageId(destination.0),
                        expression: c.source_read(source, None)?,
                        original: source,
                    },
                },
            )?;
        }
        push(
            &mut c.code.copies,
            CopyBundle {
                id: CopyPlanId(p.id().0),
                location: p.location(),
                emission,
                steps,
                assignments: copy(p.assignments())?,
            },
        )?;
    }
    for group in c.planned.storage_facts().construction_groups() {
        let at = ConstructionId(id(c.code.constructions.len())?);
        let reference = c.first_reference(group.allocation())?;
        let constructor = c.first_reference(group.initialization())?;
        let uses = c
            .planned
            .storage_facts()
            .operation_uses(group.initialization())
            .and_then(Result::ok)
            .ok_or(bug("selected construction lost its inputs"))?;
        let mut initialization_inputs = Vec::new();
        // The allocation receiver is protocol storage, not an extra emitted
        // expression read. Only the actual constructor arguments are lowered.
        if uses.len().checked_sub(1) != Some(group.arguments().len()) {
            return Err(bug(
                "selected construction lost its argument correspondence",
            ));
        }
        for (input, &read) in uses.iter().skip(1).zip(group.arguments()) {
            let origin = Some(ReadOrigin {
                site: input.site(),
                point: input.point(),
                state: input.state(),
                read: Ok(read),
                initialization: input.initialization(),
            });
            push(&mut initialization_inputs, c.source_read(read, origin)?)?;
        }
        let arguments = copy(&initialization_inputs)?;
        let mut lowered = Vec::new();
        for prepared in group.prepared_arguments() {
            // P5 selected each immediate producer/use edge and its exact order.
            // Lower the existing operation once; composition attaches it to that
            // delivered use, including an operand of another preparation.
            let inputs = c.inputs(prepared.operation)?;
            let source = c
                .planned
                .execution()
                .operations()
                .get(prepared.operation.0 as usize)
                .ok_or(bug("prepared construction argument lost its operation"))?;
            let kind = super::lower::semantic(c, source, &inputs)?;
            let expression = match &kind {
                StatementKind::Evaluate { expression } | StatementKind::Move { expression, .. } => {
                    *expression
                }
                _ => return Err(bug("prepared construction argument is not an expression")),
            };
            let statement = c.statement(kind, prepared.operation, inputs)?;
            push(&mut lowered, (prepared, statement, expression))?;
        }
        let mut prepared_arguments = Vec::new();
        for &(prepared, statement, expression) in &lowered {
            let crate::p3::UseSite::Operation {
                operation: target, ..
            } = prepared.target
            else {
                return Err(bug("prepared construction argument lost its target"));
            };
            let operands = if target == group.initialization() {
                &initialization_inputs
            } else {
                let (_, target_statement, _) = lowered
                    .iter()
                    .find(|(p, _, _)| p.operation == target)
                    .ok_or(bug(
                        "prepared construction argument lost its consuming operation",
                    ))?;
                let Arguments::Known(inputs) =
                    &c.code.statements[target_statement.0 as usize].inputs
                else {
                    return Err(bug("prepared construction target lost its inputs"));
                };
                inputs
            };
            let operand = operands
                .iter()
                .copied()
                .find(|operand| {
                    c.code.expressions[operand.0 as usize]
                        .origin
                        .as_ref()
                        .is_some_and(|origin| origin.site == prepared.target)
                })
                .ok_or(bug(
                    "prepared construction argument lost its consuming read",
                ))?;
            push(
                &mut prepared_arguments,
                PreparedExpression {
                    local: StorageId(prepared.local.0),
                    operation: prepared.operation,
                    statement,
                    expression,
                    operand,
                },
            )?;
        }
        let expression = c.expression(
            ExprKind::New {
                reference,
                constructor,
                arguments,
            },
            None,
        )?;
        let statement = c.statement(
            StatementKind::Construction(at),
            group.allocation(),
            Arguments::Known(Vec::new()),
        )?;
        let sites = std::mem::take(&mut c.group_sites[group.id().0 as usize]);
        c.code.statements[statement.0 as usize].sites = sites;
        let mut publications = Vec::new();
        for p in group.publications() {
            push(
                &mut publications,
                Publication {
                    original: *p,
                    destination: StorageId(p.local.0),
                },
            )?;
        }
        let mut snapshot_members = Vec::new();
        for p in group.snapshot_members() {
            push(&mut snapshot_members, CopyPlanId(p.0))?;
        }
        let prelude = group.prelude().map(|p| CopyPlanId(p.0));
        // P5 chose one common prelude; individual member snapshots are source
        // obligations, not repeated executable saves inside this expression.
        if let Some(prelude) = prelude {
            push(
                &mut c.code.statements[statement.0 as usize].snapshots,
                prelude,
            )?;
        }
        push(
            &mut c.code.constructions,
            Construction {
                id: at,
                statement,
                expression,
                allocation: group.allocation(),
                initialization: group.initialization(),
                normal: group.normal(),
                members: copy(group.members())?,
                interior_edges: copy(group.interior_edges())?,
                arguments: copy(group.arguments())?,
                prepared_arguments,
                publications,
                prelude,
                snapshot_members,
            },
        )?;
    }
    Ok(())
}
