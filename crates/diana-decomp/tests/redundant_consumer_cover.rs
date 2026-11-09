//! Independent allocation/constructor/barrier scenarios for an omitted consumer.

use diana_decomp::p9::Origin;
#[path = "support/inspection.rs"]
mod inspection;
#[path = "support/selected_execution.rs"]
mod selected_execution;
use selected_execution::{Executor, Fault, Reference, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Event {
    Allocate,
    Construct,
    After,
}

type Scenario = (Option<Event>, Result<Value, Fault>, Vec<Event>);

fn check(name: &str, scenarios: &[Scenario]) {
    inspection::inspect(
        "atoms/redundant-consumer-cover",
        "Lfixture/RedundantConsumer;",
        name,
        "()Lfixture/Container;",
        |_, code, rendered| {
            for (fault, expected, events) in scenarios {
                let mut actual = Vec::new();
                let mut executor = Executor::new(code, &[], |name, values| {
                    if !values.is_empty() {
                        return Err(Fault::Unsupported("unexpected fixture arguments"));
                    }
                    let (event, value) = match name {
                        "$allocate" => (Event::Allocate, Value::Reference(Reference::Object(1))),
                        "$construct" => (Event::Construct, Value::Unit),
                        "after" => (Event::After, Value::Unit),
                        _ => return Err(Fault::Unsupported("unexpected fixture helper")),
                    };
                    actual.push(event);
                    if *fault == Some(event) {
                        Err(if event == Event::Allocate {
                            Fault::Allocation
                        } else {
                            Fault::Runtime
                        })
                    } else {
                        Ok(value)
                    }
                });
                assert_eq!(&executor.run(), expected);
                drop(executor);
                assert_eq!(&actual, events);
            }
            for operation in code.operations() {
                assert_eq!(
                    rendered
                        .spans()
                        .iter()
                        .filter(|span| matches!(span.origin,
                Origin::Operation { operation: original, pc, .. }
                    if original == operation.source && pc == operation.pc))
                        .count(),
                    1
                );
            }
        },
    );
}

#[test]
fn a_redundant_check_does_not_take_ownership_of_the_alias_result() {
    check(
        "immediate",
        &[
            (
                None,
                Ok(Value::Reference(Reference::Object(1))),
                vec![Event::Allocate, Event::Construct],
            ),
            (
                Some(Event::Allocate),
                Err(Fault::Allocation),
                vec![Event::Allocate],
            ),
            (
                Some(Event::Construct),
                Err(Fault::Runtime),
                vec![Event::Allocate, Event::Construct],
            ),
        ],
    );
}

#[test]
fn an_intervening_effect_stays_after_construction_and_before_return() {
    check(
        "barrier",
        &[
            (
                None,
                Ok(Value::Reference(Reference::Object(1))),
                vec![Event::Allocate, Event::Construct, Event::After],
            ),
            (
                Some(Event::Construct),
                Err(Fault::Runtime),
                vec![Event::Allocate, Event::Construct],
            ),
            (
                Some(Event::After),
                Err(Fault::Runtime),
                vec![Event::Allocate, Event::Construct, Event::After],
            ),
        ],
    );
}
