//! Ordinary-Engine construction checks. The independent evaluator executes only
//! the final numeric argument trees, not method control, catches or constructors.
use diana_decomp::p2::{BinaryOp, OpKind};
use diana_decomp::p3::EntryRole;
use diana_decomp::p5::LocalKind;
use diana_decomp::p7::*;
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn origins(code: &MethodCode, rendered: &Rendered) {
    for operation in code.operations().iter().filter(|operation| {
        operation.effects.may_throw()
            || operation.result_pc.is_some()
            || code
                .constructions()
                .iter()
                .any(|group| group.members.contains(&operation.source))
    }) {
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(|span| matches!(span.origin,
            Origin::Operation { operation: id, .. } if id == operation.source))
                .count(),
            1,
            "one original operation anchor: {:?}",
            operation.source
        );
        if operation.result_pc.is_some() {
            assert_eq!(
                rendered
                    .spans()
                    .iter()
                    .filter(|span| matches!(span.origin,
                Origin::ResultCommit { operation: id, .. } if id == operation.source))
                    .count(),
                1
            );
        }
    }
    for site in code.sites() {
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(|span| matches!(span.origin,
            Origin::Site { site: id, .. } if id == site.id))
                .count(),
            1,
            "one original throw-site anchor"
        );
    }
}
fn selected<'a>(
    code: &'a MethodCode,
    rendered: &Rendered,
    preparations: usize,
) -> &'a Construction {
    assert_eq!(code.constructions().len(), 1);
    let group = &code.constructions()[0];
    assert_eq!(group.prepared_arguments.len(), preparations);
    assert!(
        !rendered
            .text()
            .contains("object exists before its constructor")
    );
    assert!(!rendered.text().contains(".<init>"));
    origins(code, rendered);
    group
}

#[test]
fn original_point_chains_keep_two_calls_and_two_distinct_field_reads() {
    inspect(
        "atoms/constructor-argument-chains",
        "Lfixture/ConstructorArguments;",
        "point",
        "()Lfixture/Point;",
        |_, code, rendered| {
            selected(code, rendered, 6);
            let text = rendered.text();
            assert_eq!(text.matches("getSystem()").count(), 2);
            assert_eq!(text.matches("getDisplayMetrics()").count(), 2);
            assert_eq!(text.matches("widthPixels").count(), 1);
            assert_eq!(text.matches("heightPixels").count(), 1);
            assert!(text.contains("Point point = new "));
        },
    );
}
#[test]
fn caught_argument_chains_retain_all_protected_sites_and_the_actual_catch() {
    inspect(
        "atoms/constructor-argument-chains",
        "Lfixture/ConstructorArguments;",
        "caughtPoint",
        "()Lfixture/Point;",
        |_, code, rendered| {
            let group = selected(code, rendered, 6);
            let sites = &code.statements()[group.statement.0 as usize].sites;
            assert_eq!(
                sites.len(),
                8,
                "allocation, six preparations and constructor"
            );
            let chain = code.sites()[sites[0].0 as usize].chain;
            assert!(
                sites
                    .iter()
                    .all(|id| code.sites()[id.0 as usize].chain == chain)
            );
            assert!(rendered.text().contains("catch (all)"));
            assert_eq!(
                rendered
                    .text()
                    .matches("ConstructorArguments.note(")
                    .count(),
                1
            );
        },
    );
}
#[test]
fn protected_exception_construction_composes_into_its_original_throw() {
    for (fixture, owner, name, count) in [
        (
            "constructor-argument-chains",
            "Lfixture/ConstructorArguments;",
            "bodyError",
            1,
        ),
        (
            "constructor-argument-effects",
            "Lfixture/ArgumentEffects;",
            "caughtError",
            2,
        ),
    ] {
        inspect(
            &format!("atoms/{fixture}"),
            owner,
            name,
            "(I)V",
            |_, code, rendered| {
                let group = selected(code, rendered, count);
                assert!(code.emission().composed_operation(group.initialization));
                assert_eq!(rendered.text().matches("throw new ").count(), 1);
                assert_eq!(rendered.text().matches(" [exn c0]").count(), 1);
            },
        );
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Fault {
    Read(i32),
    Unsupported,
}
struct ArgumentsModel {
    parameters: [i32; 2],
    events: Vec<String>,
}
impl ArgumentsModel {
    fn evaluate(&mut self, code: &MethodCode, id: ExprId) -> Result<i32, Fault> {
        if let Some(composed) = code.emission().composed_use(id) {
            return self.evaluate(code, composed.expression);
        }
        match &code.expressions()[id.0 as usize].kind {
            ExprKind::Literal { bits, .. } => Ok(bits.value as i32),
            ExprKind::Read { storage, .. } => {
                let LocalKind::Parameter {
                    role: EntryRole::Parameter(index),
                    ..
                } = code.locals()[storage.0 as usize].original.kind()
                else {
                    return Err(Fault::Unsupported);
                };
                self.parameters
                    .get(index as usize)
                    .copied()
                    .ok_or(Fault::Unsupported)
            }
            ExprKind::Binary {
                op: BinaryOp::AddInt,
                left,
                right,
            } => {
                let left = self.evaluate(code, *left)?;
                let right = self.evaluate(code, *right)?;
                Ok(left.wrapping_add(right))
            }
            ExprKind::Call(call) if call.receiver.is_none() => {
                let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method {
                    owner, name, ..
                })) = &code.references()[call.target.0 as usize].declaration
                else {
                    return Err(Fault::Unsupported);
                };
                if code.types()[owner.0 as usize].units
                    != "Lfixture/ArgumentEffects;"
                        .encode_utf16()
                        .collect::<Vec<_>>()
                {
                    return Err(Fault::Unsupported);
                }
                let Arguments::Known(arguments) = &call.arguments else {
                    return Err(Fault::Unsupported);
                };
                let values: Vec<_> = arguments
                    .iter()
                    .map(|id| self.evaluate(code, *id))
                    .collect::<Result<_, _>>()?;
                match (
                    String::from_utf16(&name.units)
                        .map_err(|_| Fault::Unsupported)?
                        .as_str(),
                    values.as_slice(),
                ) {
                    ("read", &[stage, fault]) => {
                        self.events.push(format!("R{stage}"));
                        if fault & (1 << stage) != 0 {
                            Err(Fault::Read(stage))
                        } else {
                            Ok(stage + 7)
                        }
                    }
                    ("finish", &[value]) => {
                        self.events.push(format!("F{value}"));
                        Ok(value.wrapping_mul(2))
                    }
                    _ => Err(Fault::Unsupported),
                }
            }
            _ => Err(Fault::Unsupported),
        }
    }
}
#[test]
fn final_argument_tree_retains_results_call_arguments_and_fault_short_circuiting() {
    inspect(
        "atoms/constructor-argument-effects",
        "Lfixture/ArgumentEffects;",
        "tree",
        "(II)Lfixture/Point;",
        |_, code, rendered| {
            let group = selected(code, rendered, 6);
            let ExprKind::New { arguments, .. } =
                &code.expressions()[group.expression.0 as usize].kind
            else {
                panic!("selected New");
            };
            for (fault, expected, events) in [
                (0, Ok(vec![30, 14]), vec!["R0", "R1", "F15", "R2"]),
                (1, Err(Fault::Read(0)), vec!["R0"]),
                (2, Err(Fault::Read(1)), vec!["R0", "R1"]),
                (4, Err(Fault::Read(2)), vec!["R0", "R1", "F15", "R2"]),
            ] {
                let mut model = ArgumentsModel {
                    parameters: [5, fault],
                    events: Vec::new(),
                };
                let actual: Result<Vec<_>, _> = arguments
                    .iter()
                    .map(|id| model.evaluate(code, *id))
                    .collect();
                assert_eq!(actual, expected);
                assert_eq!(model.events, events);
            }
        },
    );
}
#[test]
fn reversed_shared_and_interleaved_preparations_keep_original_effects() {
    for (name, calls) in [("reversed", 2), ("shared", 1), ("interleaved", 2)] {
        inspect(
            "atoms/constructor-argument-effects",
            "Lfixture/ArgumentEffects;",
            name,
            "(I)Lfixture/Point;",
            |_, code, rendered| {
                assert!(code.constructions().is_empty());
                origins(code, rendered);
                assert!(rendered.text().contains(".<init>"));
                assert_eq!(
                    rendered.text().matches("ArgumentEffects.read(").count(),
                    calls
                );
                if name == "interleaved" {
                    assert!(
                        rendered.text().find("read(1,").unwrap()
                            < rendered.text().find("finish(").unwrap()
                    );
                }
            },
        );
    }
}
#[test]
fn different_handlers_and_changing_failure_snapshots_remain_explicit() {
    for (name, prototype) in [
        ("differentProtection", "(I)Lfixture/Point;"),
        ("changingSnapshot", "(I)I"),
    ] {
        inspect(
            "atoms/constructor-argument-effects",
            "Lfixture/ArgumentEffects;",
            name,
            prototype,
            |_, code, rendered| {
                assert!(code.constructions().is_empty());
                origins(code, rendered);
                assert!(rendered.text().contains(".<init>"));
                if name == "differentProtection" {
                    assert!(rendered.text().contains("note(10,"));
                    assert!(rendered.text().contains("note(20,"));
                } else {
                    assert!(code.copies().iter().any(|copy| !copy.steps.is_empty()));
                    assert!(rendered.text().contains(" = 0;"));
                    assert!(rendered.text().contains(" = 1;"));
                    assert_eq!(
                        code.operations()
                            .iter()
                            .filter(|op| matches!(op.original, OpKind::NewInstance { .. }))
                            .count(),
                        1
                    );
                }
            },
        );
    }
}
