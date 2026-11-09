//! Selected ownership and refused joint forests use the ordinary Engine entry.
//! Inputs and independent scenarios: atoms/joint-expression-cover/README.md.

use diana_decomp::p7::{MethodCode, StatementKind};
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
            assert!(first < second && second < pair && pair < mark);
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
