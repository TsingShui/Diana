//! Preserves ordered catch facts, exceptional routes and handler receives.

use super::mapping::*;
use super::*;
use crate::p7::build::{bug, push};
use crate::p7::values::Context;

pub(super) fn chains(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let f = c.planned.facts();
    let flow = c.planned.execution();
    for chain in f.handler_chains() {
        let mut clauses = Vec::new();
        for clause in chain.clauses() {
            let catch = if let Some(type_id) = clause.catch {
                let fact = flow
                    .catch_type(type_id)
                    .ok_or(bug("catch clause lost its original descriptor result"))?;
                let descriptor = match fact.descriptor() {
                    Ok(name) => Ok(super::references::type_use(
                        &mut c.code.types,
                        crate::substrate::owned::copy_units(name)?,
                        TypeSource::Catch { original: type_id },
                    )?),
                    Err(error) => Err(super::references::failure(
                        error.kind(),
                        format_args!("{error}"),
                    )?),
                };
                Some(CatchType {
                    original: type_id,
                    descriptor,
                    use_issue: fact
                        .use_issue()
                        .map(|e| super::references::failure(e.kind(), format_args!("{e}")))
                        .transpose()?,
                })
            } else {
                None
            };
            let target = match clause.target {
                p6::CatchTarget::Handler(n) => Some(label(c, n)?),
                p6::CatchTarget::Uncaught => None,
            };
            push(
                &mut clauses,
                Clause {
                    order: clause.order,
                    catch,
                    target,
                },
            )?;
        }
        push(
            &mut c.code.chains,
            ExceptionChain {
                id: chain.id(),
                clauses,
            },
        )?;
    }
    Ok(())
}
pub(super) fn sites(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let f = c.planned.facts();
    for site in f.throws() {
        let mut routes = Vec::new();
        let mut impossible = true;
        for &edge in site.edges() {
            let p6::EdgeRoute::Exception {
                order, delivery, ..
            } = f.edges()[edge.0 as usize].route()
            else {
                return Err(bug("throw site lost its exceptional route"));
            };
            let delivery = match delivery {
                p6::ExceptionDelivery::Explicit => {
                    impossible = false;
                    Delivery::Explicit
                }
                p6::ExceptionDelivery::Catch(t) => {
                    impossible = false;
                    Delivery::Catch(control(c.tries[t.0 as usize])?)
                }
                p6::ExceptionDelivery::ImpossibleRelease(op) => {
                    Delivery::ImpossibleRelease(statement(c, op)?)
                }
            };
            push(
                &mut routes,
                ExceptionRoute {
                    edge,
                    order,
                    delivery,
                },
            )?;
        }
        let syntax = if impossible {
            ExceptionSyntax::ImpossibleRelease
        } else if site.protection() == p6::SiteProtection::Natural {
            ExceptionSyntax::Natural
        } else if f.handler_chains()[site.chain().0 as usize]
            .clauses()
            .iter()
            .all(|x| x.target == p6::CatchTarget::Uncaught)
        {
            ExceptionSyntax::Propagate
        } else {
            ExceptionSyntax::Explicit
        };
        push(
            &mut c.code.sites,
            ExceptionSite {
                id: SiteId(site.id().0),
                original: site.site(),
                completion: site.completion(),
                chain: site.chain(),
                routes,
                syntax,
                construction: site.construction().map(|g| ConstructionId(g.0)),
                source_snapshots: ids(site.snapshots(), |p| CopyPlanId(p.0))?,
            },
        )?;
    }
    Ok(())
}
pub(super) fn handlers(c: &mut Context<'_, '_>) -> Result<(), ExpressionError> {
    let f = c.planned.facts();
    for h in f.handler_entries() {
        let capture = if let Some(op) = h.capture() {
            let statement = statement(c, op)?;
            match c.code.operations[op.0 as usize].emission {
                Emission::Catch(control) => HandlerCapture::Catch { statement, control },
                Emission::Block { .. } => HandlerCapture::Body(statement),
                _ => return Err(bug("handler capture has an incompatible expression owner")),
            }
        } else {
            HandlerCapture::None
        };
        push(
            &mut c.code.handlers,
            Handler {
                node: h.node(),
                capture,
                receives: ids(h.receives(), |p| CopyPlanId(p.0))?,
            },
        )?;
    }
    Ok(())
}
