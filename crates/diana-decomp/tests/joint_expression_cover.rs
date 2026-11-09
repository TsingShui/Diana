//! Selected ownership and refused joint forests use the ordinary Engine entry.
//! Inputs and independent scenarios: atoms/joint-expression-cover/README.md.

use diana_decomp::p7::{
    ExprId, ExprKind, MemberDeclaration, MethodCode, ReferenceDeclaration, StatementKind,
};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;

fn ownership(code: &MethodCode, rendered: &Rendered) {
    for expression in code.expressions() {
        let Some(evaluation) = expression.evaluation else {
            continue;
        };
        let operation = &code.operations()[evaluation.operation.0 as usize];
        assert_eq!(operation.nested_at, Some(expression.id));
        assert!(
            expression.origin.is_some(),
            "retain the original exact read"
        );
        let statement = operation.statement.expect("original producer statement");
        let source_expression = match code.statements()[statement.0 as usize].kind {
            StatementKind::Move { expression, .. } | StatementKind::Evaluate { expression } => {
                expression
            }
            _ => panic!("fixture producers are ordinary value operations"),
        };
        assert_eq!(evaluation.expression, source_expression);
        for origin in [
            Origin::Operation {
                method: diana_decomp::MethodId(0),
                operation: operation.source,
                pc: operation.pc,
            },
            Origin::Statement {
                method: diana_decomp::MethodId(0),
                statement,
            },
        ] {
            // Match original member-local identities; method IDs are engine-wide.
            let spans = rendered
                .spans()
                .iter()
                .filter(|span| match (span.origin, origin) {
                    (
                        Origin::Operation {
                            operation: actual,
                            pc,
                            ..
                        },
                        Origin::Operation {
                            operation: expected,
                            pc: expected_pc,
                            ..
                        },
                    ) => actual == expected && pc == expected_pc,
                    (
                        Origin::Statement {
                            statement: actual, ..
                        },
                        Origin::Statement {
                            statement: expected,
                            ..
                        },
                    ) => actual == expected,
                    _ => false,
                })
                .collect::<Vec<_>>();
            assert_eq!(spans.len(), 1, "one nested producer anchor");
            assert!(spans[0].start.0 < spans[0].end.0);
        }
        if let Some(pc) = operation.result_pc {
            assert_eq!(rendered.spans().iter().filter(|span| {
                matches!(span.origin, Origin::ResultCommit { operation: actual, pc: actual_pc, .. }
                    if actual == operation.source && actual_pc == pc)
            }).count(), 1);
        }
    }
    for operation in code.operations() {
        if let Some(operand) = operation.nested_at {
            assert_eq!(
                code.expressions()[operand.0 as usize]
                    .evaluation
                    .expect("owner expression")
                    .operation,
                operation.source
            );
        }
    }
}

#[test]
fn a_condition_owns_each_effectful_argument_once() {
    inspection::inspect(
        "atoms/joint-expression-cover",
        "Lfixture/JointCover;",
        "ordered",
        "(I)I",
        |_, code, rendered| {
            ownership(code, rendered);
            assert!(rendered.text().contains(
                "p0 != 0 || fixture.JointCover.pair(fixture.JointCover.take(1), fixture.JointCover.take(2)) != 0"
            ), "{}", rendered.text());
            assert_eq!(rendered.text().matches("JointCover.take(").count(), 2);
            assert_eq!(rendered.text().matches("JointCover.pair(").count(), 1);
        },
    );
}

#[test]
fn refusing_the_condition_keeps_the_whole_effect_schedule() {
    inspection::inspect(
        "atoms/joint-expression-cover",
        "Lfixture/JointCover;",
        "blocked",
        "(I)I",
        |_, code, rendered| {
            ownership(code, rendered);
            assert!(!rendered.text().contains(" || "));
            let text = rendered.text();
            let first = text.find("JointCover.take(1)").unwrap();
            let second = text.find("JointCover.take(2)").unwrap();
            let pair = text.find("JointCover.pair(").unwrap();
            let mark = text.find("JointCover.mark(").unwrap();
            assert!(first < second && second < mark && pair < mark);
            let statement = code
                .statements()
                .iter()
                .find(|statement| {
                    matches!(statement.kind, StatementKind::Evaluate { expression }
                    if call_name(code, expression).as_deref() == Some("pair"))
                })
                .unwrap();
            let StatementKind::Evaluate { expression } = statement.kind else {
                unreachable!()
            };
            let (result, events) = replay(code, 0, None, Some(expression));
            assert_eq!(result, Ok(1112));
            assert_eq!(
                events,
                vec![Event::Take(1), Event::Take(2), Event::Pair(11, 12)]
            );
            assert_eq!(text.matches("JointCover.take(").count(), 2);
            assert_eq!(text.matches("JointCover.pair(").count(), 1);
            assert_eq!(text.matches("JointCover.mark(").count(), 1);
        },
    );
}

#[test]
fn ordinary_and_reversed_operands_keep_original_producer_ownership() {
    for name in ["ordinary", "reversed"] {
        inspection::inspect(
            "atoms/joint-expression-cover",
            "Lfixture/JointCover;",
            name,
            "(I)I",
            |_, code, rendered| {
                ownership(code, rendered);
                let text = rendered.text();
                assert_eq!(text.matches("JointCover.take(").count(), 2);
                assert_eq!(text.matches("JointCover.pair(").count(), 1);
                assert!(text.contains("p0 + 1"));
            },
        );
    }
}

#[test]
fn grouped_argument_evaluations_keep_the_original_operation_record() {
    inspection::inspect(
        "atoms/joint-expression-cover",
        "Lfixture/JointCover;",
        "constructed",
        "(I)Lfixture/CoverBox;",
        |_, code, rendered| {
            ownership(code, rendered);
            assert!(
                !code.constructions().is_empty(),
                "selected original construction"
            );
            assert_eq!(rendered.text().matches("JointCover.take(").count(), 2);
            assert!(rendered.text().contains("new "));
        },
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Take(i32),
    Pair(i32, i32),
    Mark(i32),
}

fn call_name(code: &MethodCode, expression: ExprId) -> Option<String> {
    let ExprKind::Call(call) = &code.expressions()[expression.0 as usize].kind else {
        return None;
    };
    let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { name, .. })) =
        &code.references()[call.target.0 as usize].declaration
    else {
        return None;
    };
    Some(String::from_utf16(&name.units).unwrap())
}

#[path = "support/selected_execution.rs"]
mod selected_execution;
use selected_execution::{Executor, Fault, Value};

fn replay(
    code: &MethodCode,
    seed: i32,
    fault: Option<usize>,
    expression: Option<ExprId>,
) -> (Result<i32, usize>, Vec<Event>) {
    let mut events = Vec::new();
    let mut run = Executor::new(code, &[Value::Int(seed)], |name, arguments| {
        let (event, value) = match (name, arguments) {
            ("take", &[Value::Int(x)]) => (Event::Take(x), Value::Int(x.wrapping_add(10))),
            ("pair", &[Value::Int(left), Value::Int(right)]) => (
                Event::Pair(left, right),
                Value::Int(left.wrapping_mul(100).wrapping_add(right)),
            ),
            ("mark", &[Value::Int(x)]) => (Event::Mark(x), Value::Unit),
            _ => return Err(Fault::Unsupported("joint helper")),
        };
        let index = events.len();
        events.push(event);
        if fault == Some(index) {
            Err(Fault::Helper(index))
        } else {
            Ok(value)
        }
    });
    let result = match expression {
        Some(expression) => run.expression(expression),
        None => run.run(),
    };
    drop(run);
    (
        result.map(Value::int).map_err(|fault| match fault {
            Fault::Helper(index) => index,
            _ => panic!("unmodeled joint execution {fault:?}"),
        }),
        events,
    )
}

#[test]
fn joint_forests_preserve_results_effects_and_every_reachable_call_fault() {
    for name in ["ordinary", "reversed", "tree", "barrier"] {
        inspection::inspect(
            "atoms/joint-expression-cover",
            "Lfixture/JointCover;",
            name,
            "(I)I",
            |_, code, rendered| {
                ownership(code, rendered);
                for seed in [0_i32, 3, -10, i32::MAX] {
                    let a = seed.wrapping_add(10);
                    let b = seed.wrapping_add(11);
                    let left = a.wrapping_mul(100).wrapping_add(b);
                    let (events, result) = match name {
                        "ordinary" => (
                            vec![
                                Event::Take(seed),
                                Event::Take(seed.wrapping_add(1)),
                                Event::Pair(a, b),
                            ],
                            left,
                        ),
                        "reversed" => (
                            vec![
                                Event::Take(seed),
                                Event::Take(seed.wrapping_add(1)),
                                Event::Pair(b, a),
                            ],
                            b.wrapping_mul(100).wrapping_add(a),
                        ),
                        "barrier" => (
                            vec![
                                Event::Take(seed),
                                Event::Mark(a),
                                Event::Take(seed.wrapping_add(1)),
                                Event::Pair(a, b),
                            ],
                            left,
                        ),
                        "tree" => {
                            let c = seed.wrapping_add(12);
                            let d = seed.wrapping_add(13);
                            let right = c.wrapping_mul(100).wrapping_add(d);
                            (
                                vec![
                                    Event::Take(seed),
                                    Event::Take(seed.wrapping_add(1)),
                                    Event::Pair(a, b),
                                    Event::Take(seed.wrapping_add(2)),
                                    Event::Take(seed.wrapping_add(3)),
                                    Event::Pair(c, d),
                                    Event::Pair(left, right),
                                ],
                                left.wrapping_mul(100).wrapping_add(right),
                            )
                        }
                        _ => unreachable!(),
                    };
                    let (actual, trace) = replay(code, seed, None, None);
                    assert_eq!(actual, Ok(result), "{name}({seed})");
                    assert_eq!(trace, events);
                    for fault in 0..events.len() {
                        let (actual, trace) = replay(code, seed, Some(fault), None);
                        assert_eq!(actual, Err(fault), "{name}({seed}) fault {fault}");
                        assert_eq!(trace, events[..=fault]);
                    }
                }
                if name == "ordinary" || name == "tree" {
                    assert!(
                        !rendered.text().contains("int v"),
                        "complete joint return forest"
                    );
                }
            },
        );
    }
}
