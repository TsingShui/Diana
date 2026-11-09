//! Required checks and preparations share ordered operation ownership.
//! Expectations: composite/cast-preparation-order/README.md.

use diana_decomp::p7::MethodCode;
use diana_decomp::p9::{Origin, Rendered};
#[path = "support/inspection.rs"]
mod inspection;
#[path = "support/selected_execution.rs"]
mod selected_execution;
use selected_execution::{Executor, Fault, Reference, Value};

const NULL: Value = Value::Reference(Reference::Null);
const ARRAY: Value = Value::Reference(Reference::Array {
    identity: 1,
    length: 3,
});
const OBJECT: Value = Value::Reference(Reference::Object(2));
const STRING: Value = Value::Reference(Reference::Object(3));
#[derive(Debug, Eq, PartialEq)]
enum Event {
    String,
    Equals(Value, Value),
    Produce,
    Consume(Value),
}

fn origins(code: &MethodCode, rendered: &Rendered) {
    for operation in code.operations() {
        if matches!(operation.original, diana_decomp::p2::OpKind::Cast { .. }) {
            assert_eq!(rendered.spans().iter().filter(|span| matches!(span.origin, Origin::Operation { operation: actual, pc, .. } if actual == operation.source && pc == operation.pc)).count(), 1);
            let statement = operation.statement.unwrap();
            assert_eq!(rendered.spans().iter().filter(|span| matches!(span.origin, Origin::Statement { statement: actual, .. } if actual == statement)).count(), 1);
            for &site in &code.statement(statement).unwrap().sites {
                assert_eq!(rendered.spans().iter().filter(|span| matches!(span.origin, Origin::Site { site: actual, .. } if actual == site)).count(), 1);
            }
        }
    }
}
fn execute(
    code: &MethodCode,
    parameters: &[Value],
    string_fault: bool,
    producer: Value,
    fault: Option<usize>,
) -> (Result<Value, Fault>, Vec<Event>) {
    let mut events = Vec::new();
    let mut run = Executor::new(code, parameters, |name, arguments| {
        match (name, arguments) {
            ("$string", &[Value::Int(120)]) => {
                events.push(Event::String);
                if string_fault {
                    Err(Fault::Helper(0))
                } else {
                    Ok(STRING)
                }
            }
            ("equals", &[receiver, argument]) => {
                events.push(Event::Equals(receiver, argument));
                if receiver == NULL {
                    Err(Fault::NullReceiver)
                } else {
                    Ok(Value::Int(0))
                }
            }
            ("produce", []) => {
                events.push(Event::Produce);
                if fault == Some(0) {
                    Err(Fault::Helper(0))
                } else {
                    Ok(producer)
                }
            }
            ("consume", &[value]) => {
                events.push(Event::Consume(value));
                if fault == Some(1) {
                    Err(Fault::Helper(1))
                } else {
                    Ok(Value::Int(7))
                }
            }
            _ => Err(Fault::Unsupported("preparation helper")),
        }
    });
    let result = run.run();
    drop(run);
    (result, events)
}
fn inspect(name: &str, prototype: &str, check: impl FnOnce(&MethodCode, &Rendered)) {
    inspection::inspect(
        "composite/cast-preparation-order",
        "Lfixture/CarrierPreparation;",
        name,
        prototype,
        |_, code, rendered| {
            origins(code, rendered);
            check(code, rendered);
        },
    );
}

#[test]
fn string_resolution_and_check_faults_keep_their_original_precedence() {
    for name in ["ordered", "castFirst"] {
        inspect(name, "(Ljava/lang/Object;I)I", |code, rendered| {
            for receiver in [ARRAY, OBJECT, NULL] {
                for string_fault in [false, true] {
                    assert_eq!(
                        execute(code, &[receiver, Value::Int(1)], string_fault, ARRAY, None),
                        (Ok(Value::Int(1)), vec![])
                    );
                }
            }
            for (receiver, string_fault) in [
                (ARRAY, false),
                (NULL, false),
                (ARRAY, true),
                (NULL, true),
                (OBJECT, false),
                (OBJECT, true),
            ] {
                let (expected, events) = if receiver == OBJECT && name == "castFirst" {
                    (Err(Fault::Cast), vec![])
                } else if string_fault {
                    (Err(Fault::Helper(0)), vec![Event::String])
                } else if receiver == OBJECT {
                    (Err(Fault::Cast), vec![Event::String])
                } else {
                    (
                        if receiver == NULL {
                            Err(Fault::NullReceiver)
                        } else {
                            Ok(Value::Int(0))
                        },
                        vec![Event::String, Event::Equals(receiver, STRING)],
                    )
                };
                assert_eq!(
                    execute(code, &[receiver, Value::Int(0)], string_fault, ARRAY, None),
                    (expected, events),
                    "{name} {receiver:?} string_fault={string_fault}"
                );
            }
            if name == "castFirst" {
                assert!(
                    rendered.text().contains(" || "),
                    "valid operand ordering retains compact recovery"
                );
            }
        });
    }
}
#[test]
fn a_produced_reference_reaches_one_check_and_one_consumer() {
    inspect("produced", "()I", |code, _| {
        for reference in [ARRAY, NULL] {
            assert_eq!(
                execute(code, &[], false, reference, None),
                (
                    Ok(Value::Int(7)),
                    vec![Event::Produce, Event::Consume(reference)]
                )
            );
        }
        assert_eq!(
            execute(code, &[], false, OBJECT, None),
            (Err(Fault::Cast), vec![Event::Produce])
        );
        assert_eq!(
            execute(code, &[], false, ARRAY, Some(0)),
            (Err(Fault::Helper(0)), vec![Event::Produce])
        );
        assert_eq!(
            execute(code, &[], false, ARRAY, Some(1)),
            (
                Err(Fault::Helper(1)),
                vec![Event::Produce, Event::Consume(ARRAY)]
            )
        );
    });
}
