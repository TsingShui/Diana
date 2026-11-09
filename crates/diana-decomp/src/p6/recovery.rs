//! Publishes named roles from this P6 execution. No downstream stage rematches
//! children, conditional polarity or loop placement from positional source lists.

use super::build::{RegionError, bug, push};
use super::graph::Graph;
use super::*;
use crate::p2::{EdgeRole, NormalEdge};
use crate::p5::StorageMethod;
use crate::recovery::{ConditionElse, Finally, IfShape, LoopPlacement, Predicate};

pub(super) fn construct(
    storage: &StorageMethod<'_>,
    facts: &mut RegionFacts,
    graph: &Graph,
) -> Result<(), RegionError> {
    for region in &facts.regions {
        let layout = layout(storage, facts, graph, region)?;
        push(&mut facts.controls, layout)?;
    }
    Ok(())
}

fn child(region: &Region, index: usize) -> Result<RegionId, RegionError> {
    region
        .children
        .get(index)
        .copied()
        .ok_or(bug("recovery control lost a selected child role"))
}

fn children(
    region: &Region,
    range: std::ops::RangeFrom<usize>,
) -> Result<Vec<RegionId>, RegionError> {
    let source = region
        .children
        .get(range)
        .ok_or(bug("recovery control lost its child list"))?;
    let mut result = Vec::new();
    crate::substrate::owned::reserve(&mut result, source.len())?;
    result.extend_from_slice(source);
    Ok(result)
}

fn predicate(
    storage: &StorageMethod<'_>,
    facts: &RegionFacts,
    graph: &Graph,
    node: NodeId,
    select: Option<LinkId>,
) -> Result<Predicate, RegionError> {
    let terminal = facts.nodes[node.0 as usize].block;
    let portion = graph.terminal_node[terminal.0 as usize];
    let mut when_true = None;
    let mut when_false = None;
    for &link in &graph.outgoing[portion.0 as usize] {
        let LinkOrigin::Edge(edge) = facts.links[link.0 as usize].origin else {
            continue;
        };
        match storage.execution().edges()[edge.0 as usize].role() {
            EdgeRole::Normal(NormalEdge::ConditionalTrue) => when_true = Some(link),
            EdgeRole::Normal(NormalEdge::ConditionalFalse) => when_false = Some(link),
            _ => {}
        }
    }
    let when_true = when_true.ok_or(bug("recovery predicate lost its true edge"))?;
    let when_false = when_false.ok_or(bug("recovery predicate lost its false edge"))?;
    if select.is_some_and(|edge| edge != when_true && edge != when_false) {
        return Err(bug("recovery predicate selects a foreign edge"));
    }
    Ok(Predicate {
        terminal,
        when_true,
        when_false,
        negate: select == Some(when_false),
    })
}

fn layout(
    storage: &StorageMethod<'_>,
    facts: &RegionFacts,
    graph: &Graph,
    region: &Region,
) -> Result<ControlLayout, RegionError> {
    if let Some(decision) = &region.decision {
        let mut bodies = Vec::new();
        for arm in &decision.tests {
            push(&mut bodies, arm.body)?;
        }
        push(&mut bodies, decision.default)?;
        return Ok(ControlLayout::When {
            bodies: bodies.into(),
        });
    }
    Ok(match &region.kind {
        RegionKind::Block(node) => ControlLayout::Block(*node),
        RegionKind::Sequence => ControlLayout::Sequence,
        RegionKind::If { .. } => ControlLayout::If {
            header: child(region, 0)?,
            yes: child(region, 1)?,
            no: child(region, 2)?,
            test: predicate(
                storage,
                facts,
                graph,
                region.entry.ok_or(bug("if lost its entry"))?,
                None,
            )?,
            shape: IfShape::IfElse,
        },
        RegionKind::Condition(id) => {
            let source = &facts.conditions[id.0 as usize];
            let mut tests = Vec::new();
            for test in &source.tests {
                push(
                    &mut tests,
                    predicate(storage, facts, graph, test.predicate, Some(test.select))?,
                )?;
            }
            let count = source.tests.len();
            ControlLayout::Condition {
                header: child(region, 0)?,
                tests: tests.into(),
                yes: child(region, count)?,
                no: if source.else_arm {
                    ConditionElse::Body(child(region, count + 1)?)
                } else if source.other_exit {
                    ConditionElse::Transfer
                } else {
                    ConditionElse::None
                },
                fall: source
                    .tests
                    .last()
                    .ok_or(bug("condition has no predicate"))?
                    .other,
                routing: (!source.routing.is_empty())
                    .then(|| child(region, count + 1 + usize::from(source.else_arm)))
                    .transpose()?,
            }
        }
        RegionKind::Loop(id) => {
            let placement = match facts.loops[id.0 as usize].form {
                LoopForm::Infinite => LoopPlacement::Infinite,
                LoopForm::While(test) => LoopPlacement::While {
                    predicate: child(region, 1)?,
                    test: predicate(storage, facts, graph, test.node, Some(test.repeat))?,
                    compact: false,
                },
                LoopForm::DoWhile(test) => LoopPlacement::DoWhile {
                    predicate: child(region, 1)?,
                    test: predicate(storage, facts, graph, test.node, Some(test.repeat))?,
                    compact: false,
                },
            };
            ControlLayout::Loop {
                body: child(region, 0)?,
                placement,
            }
        }
        RegionKind::Switch { cases, .. } => {
            if cases
                .as_ref()
                .is_some_and(|groups| region.children.len().checked_sub(1) != Some(groups.len()))
            {
                return Err(bug("switch lost its case-group children"));
            }
            let cases = cases
                .as_ref()
                .map(|groups| {
                    let mut result = Vec::new();
                    for group in groups {
                        let mut links = Vec::new();
                        crate::substrate::owned::reserve(&mut links, group.len())?;
                        links.extend_from_slice(group);
                        push(&mut result, links)?;
                    }
                    Ok::<_, RegionError>(result.into())
                })
                .transpose()?;
            ControlLayout::Switch {
                header: child(region, 0)?,
                bodies: children(region, 1..)?.into(),
                cases,
                terminal: facts.nodes[region.entry.ok_or(bug("switch lost its entry"))?.0 as usize]
                    .block,
            }
        }
        RegionKind::LabelledBlock { .. } => ControlLayout::Labelled,
        RegionKind::Try(id) => {
            let finally = facts.tries[id.0 as usize].finally.as_ref();
            let mut catches = children(region, 1..)?;
            let finally = if finally.is_some() {
                if catches.len() != 2 {
                    return Err(bug("finally lost its cleanup or duplicate child"));
                }
                let duplicate = catches
                    .pop()
                    .ok_or(bug("finally lost its duplicate child"))?;
                Some(Finally {
                    cleanup: catches[0],
                    duplicate,
                })
            } else {
                None
            };
            ControlLayout::Try {
                body: child(region, 0)?,
                catches: catches.into(),
                finally,
            }
        }
        RegionKind::Synchronized(id) => {
            let sync = &facts.synchronized[id.0 as usize];
            ControlLayout::Synchronized {
                implicit: sync.implicit.is_some(),
                entry: child(region, 0)?,
                body: child(region, 1)?,
                acquire: sync.enter,
            }
        }
        RegionKind::Graph { .. } => ControlLayout::Graph,
    })
}
