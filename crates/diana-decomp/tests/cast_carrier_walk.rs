//! Cast ordering, source identities and local carrier boundaries.
//! Inputs/scenarios live in cast-carrier-order and cast-carrier-contexts.

use diana_decomp::p3::UseSite;
use diana_decomp::p7::MethodCode;
use diana_decomp::p9::{Origin, Rendered};
#[path = "support/inspection.rs"]
mod inspection;
#[path = "support/selected_execution.rs"]
mod selected_execution;
use selected_execution::{Executor, Fault, Reference, Value};

const NULL: Value = Value::Reference(Reference::Null);
const A: Value = Value::Reference(Reference::Array {
    identity: 1,
    length: 3,
});
const B: Value = Value::Reference(Reference::Array {
    identity: 2,
    length: 5,
});
const OBJECT: Value = Value::Reference(Reference::Object(3));

#[derive(Debug, Eq, PartialEq)]
enum Event {
    Make,
    Sink(Value, Value),
}
fn execute(
    code: &MethodCode,
    parameters: &[Value],
    make: Value,
    fault: bool,
) -> (Result<Value, Fault>, Vec<Event>) {
    let mut events = Vec::new();
    let mut run = Executor::new(code, parameters, |name, arguments| {
        match (name, arguments) {
            ("make", []) => {
                events.push(Event::Make);
                if fault {
                    Err(Fault::Helper(0))
                } else {
                    Ok(make)
                }
            }
            ("sink", &[n, value]) => {
                events.push(Event::Sink(n, value));
                if fault {
                    Err(Fault::Helper(0))
                } else {
                    Ok(Value::Unit)
                }
            }
            _ => Err(Fault::Unsupported("fixture helper")),
        }
    });
    let result = run.run();
    drop(run);
    (result, events)
}
fn origins(code: &MethodCode, rendered: &Rendered) {
    let checks = code
        .operations()
        .iter()
        .filter(|operation| matches!(operation.original, diana_decomp::p2::OpKind::Cast { .. }))
        .collect::<Vec<_>>();
    assert_eq!(checks.len(), 1);
    let operation = checks[0];
    let statement = operation.statement.unwrap();
    assert_eq!(rendered.spans().iter().filter(|span| matches!(span.origin, Origin::Operation { operation: actual, pc, .. } if actual == operation.source && pc == operation.pc)).count(), 1);
    assert_eq!(rendered.spans().iter().filter(|span| matches!(span.origin, Origin::Statement { statement: actual, .. } if actual == statement)).count(), 1);
    for &site in &code.statement(statement).unwrap().sites {
        assert_eq!(rendered.spans().iter().filter(|span| matches!(span.origin, Origin::Site { site: actual, .. } if actual == site)).count(), 1);
    }
}
enum Owner {
    Operand(UseSite),
    Located,
}
fn owner(code: &MethodCode) -> Option<Owner> {
    let operation = code
        .operations()
        .iter()
        .find(|operation| matches!(operation.original, diana_decomp::p2::OpKind::Cast { .. }))
        .unwrap();
    let inline = code
        .emission()
        .inline_checks()
        .iter()
        .filter(|check| check.operation == operation.source)
        .collect::<Vec<_>>();
    if let Some(operand) = operation.nested_at {
        assert!(inline.is_empty(), "one check execution owner");
        assert_eq!(
            code.expressions()[operand.0 as usize]
                .evaluation
                .unwrap()
                .operation,
            operation.source
        );
        let origin = code.expressions()[operand.0 as usize]
            .origin
            .as_ref()
            .unwrap();
        return Some(Owner::Operand(origin.site));
    }
    assert!(inline.len() <= 1);
    inline.first().map(|_| Owner::Located)
}

fn order(name: &str, prototype: &str, check: impl FnOnce(&MethodCode, &Rendered)) {
    inspection::inspect(
        "atoms/cast-carrier-order",
        "Lfixture/CarrierOrder;",
        name,
        prototype,
        |_, code, rendered| {
            origins(code, rendered);
            check(code, rendered);
        },
    );
}
fn context(name: &str, prototype: &str, check: impl FnOnce(&MethodCode, &Rendered)) {
    inspection::inspect(
        "composite/cast-carrier-contexts",
        "Lfixture/CarrierContexts;",
        name,
        prototype,
        |_, code, rendered| {
            origins(code, rendered);
            check(code, rendered);
        },
    );
}

#[test]
fn the_second_comparison_operand_carries_one_original_check() {
    order(
        "right",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        |code, rendered| {
            assert!(owner(code).is_some());
            assert!(matches!(
                owner(code),
                Some(Owner::Operand(UseSite::Terminal { .. }))
            ));
            assert!(!rendered.text().contains("check-cast, value not reassigned"));
            for (left, right, expected) in [
                (A, A, Ok(Value::Int(1))),
                (NULL, NULL, Ok(Value::Int(1))),
                (A, B, Ok(Value::Int(0))),
                (A, NULL, Ok(Value::Int(0))),
                (A, OBJECT, Err(Fault::Cast)),
            ] {
                assert_eq!(execute(code, &[left, right], B, false), (expected, vec![]));
            }
        },
    );
}
#[test]
fn literals_before_a_checked_argument_do_not_create_an_effect_boundary() {
    order("argument", "(Ljava/lang/Object;)V", |code, rendered| {
        assert!(owner(code).is_some());
        assert!(matches!(
            owner(code),
            Some(Owner::Operand(UseSite::Operation { .. }))
        ));
        assert!(rendered.text().contains("sink(0, ((int[]) p0))"));
        for reference in [A, NULL] {
            assert_eq!(
                execute(code, &[reference], B, false),
                (Ok(Value::Unit), vec![Event::Sink(Value::Int(0), reference)])
            );
            assert_eq!(
                execute(code, &[reference], B, true),
                (
                    Err(Fault::Helper(0)),
                    vec![Event::Sink(Value::Int(0), reference)]
                )
            );
        }
        assert_eq!(
            execute(code, &[OBJECT], B, false),
            (Err(Fault::Cast), vec![])
        );
    });
}
#[test]
fn a_check_remains_before_an_early_exit_or_helper_effect() {
    order(
        "guarded",
        "(Ljava/lang/Object;Ljava/lang/Object;I)I",
        |code, rendered| {
            assert!(owner(code).is_none());
            assert!(
                rendered.text().find("((int[]) p1)").unwrap()
                    < rendered.text().find("if (").unwrap()
            );
            for (right, flag, expected) in [
                (A, 0, Ok(Value::Int(1))),
                (B, 0, Ok(Value::Int(0))),
                (A, 1, Ok(Value::Int(99))),
                (OBJECT, 1, Err(Fault::Cast)),
            ] {
                assert_eq!(
                    execute(code, &[A, right, Value::Int(flag)], B, false),
                    (expected, vec![])
                );
            }
        },
    );
    order(
        "afterEffect",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        |code, _| {
            assert!(owner(code).is_none());
            for reference in [A, NULL] {
                assert_eq!(
                    execute(code, &[reference], B, false),
                    (Ok(reference), vec![Event::Make])
                );
                assert_eq!(
                    execute(code, &[reference], B, true),
                    (Err(Fault::Helper(0)), vec![Event::Make])
                );
            }
            assert_eq!(
                execute(code, &[OBJECT], B, false),
                (Err(Fault::Cast), vec![])
            );
        },
    );
}
#[test]
fn earlier_carriers_keep_cast_and_null_failure_order() {
    order(
        "left",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        |code, _| {
            assert!(owner(code).is_some());
            for (left, right, expected) in [
                (A, A, Ok(Value::Int(1))),
                (NULL, NULL, Ok(Value::Int(1))),
                (OBJECT, A, Err(Fault::Cast)),
            ] {
                assert_eq!(execute(code, &[left, right], B, false), (expected, vec![]));
            }
        },
    );
    order("arrayLength", "(Ljava/lang/Object;)I", |code, _| {
        assert!(owner(code).is_some());
        for (reference, expected) in [
            (A, Ok(Value::Int(3))),
            (B, Ok(Value::Int(5))),
            (NULL, Err(Fault::NullArray)),
            (OBJECT, Err(Fault::Cast)),
        ] {
            assert_eq!(execute(code, &[reference], B, false), (expected, vec![]));
        }
    });
}
#[test]
fn short_circuit_contexts_evaluate_the_check_on_the_first_test_only() {
    context(
        "triplet",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)I",
        |code, rendered| {
            assert!(owner(code).is_some());
            assert_eq!(rendered.text().matches("((int[]) p1)").count(), 1);
            for (a, b, c, expected) in [
                (A, A, A, Ok(Value::Int(1))),
                (NULL, NULL, NULL, Ok(Value::Int(1))),
                (A, B, B, Ok(Value::Int(0))),
                (A, A, B, Ok(Value::Int(0))),
                (A, NULL, NULL, Ok(Value::Int(0))),
                (A, OBJECT, A, Err(Fault::Cast)),
            ] {
                assert_eq!(execute(code, &[a, b, c], B, false), (expected, vec![]));
            }
        },
    );
}
#[test]
fn catch_membership_and_nested_call_carriers_remain_boundaries() {
    context(
        "caught",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        |code, _| {
            assert!(owner(code).is_none());
            for (left, right, expected) in [
                (A, A, Value::Int(1)),
                (NULL, NULL, Value::Int(1)),
                (A, B, Value::Int(0)),
                (A, OBJECT, Value::Int(-1)),
            ] {
                assert_eq!(
                    execute(code, &[left, right], B, false),
                    (Ok(expected), vec![])
                );
            }
        },
    );
    context("acting", "(Ljava/lang/Object;)I", |code, rendered| {
        assert!(owner(code).is_none());
        assert!(
            rendered.text().find("((int[]) p0)").unwrap() < rendered.text().find("make()").unwrap()
        );
        for (reference, make, expected) in [
            (A, A, Ok(Value::Int(1))),
            (A, B, Ok(Value::Int(0))),
            (NULL, NULL, Ok(Value::Int(1))),
            (OBJECT, A, Err(Fault::Cast)),
        ] {
            let events = if reference == OBJECT {
                vec![]
            } else {
                vec![Event::Make]
            };
            assert_eq!(execute(code, &[reference], make, false), (expected, events));
        }
        assert_eq!(
            execute(code, &[A], B, true),
            (Err(Fault::Helper(0)), vec![Event::Make])
        );
    });
}
