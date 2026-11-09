//! Independent helper/lock scenarios for composite/operation-records.

use diana_decomp::p7::{InitializationSyntax, MethodCode, StatementKind};
use diana_decomp::p9::{Origin, Rendered};
#[path = "support/inspection.rs"]
mod inspection;
#[path = "support/selected_execution.rs"]
mod selected_execution;
use selected_execution::{Executor, Fault, Reference, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Event {
    First(i32),
    Allocate,
    Later,
    Gate(i32, i32),
    Acquire,
    Release,
}

fn run(
    code: &MethodCode,
    parameters: &[Value],
    fault: Option<Event>,
) -> (Result<Value, Fault>, Vec<Event>) {
    let mut events = Vec::new();
    let mut executor = Executor::new(code, parameters, |name, values| {
        let (event, value) = match (name, values) {
            ("first", &[Value::Int(seed)]) => {
                (Event::First(seed), Value::Int(seed.wrapping_add(10)))
            }
            ("later", []) => (Event::Later, Value::Int(20)),
            ("$allocate", []) => (Event::Allocate, Value::Reference(Reference::Object(1))),
            ("$construct", &[Value::Int(a), Value::Int(b)]) => (Event::Gate(a, b), Value::Unit),
            ("$acquire", &[Value::Reference(Reference::Null)]) => return Err(Fault::NullLock),
            ("$acquire", &[Value::Reference(Reference::Object(2))]) => {
                (Event::Acquire, Value::Unit)
            }
            ("$release", &[Value::Reference(Reference::Object(2))]) => {
                (Event::Release, Value::Unit)
            }
            _ => return Err(Fault::Unsupported("operation-record helper")),
        };
        events.push(event);
        if fault == Some(event) {
            Err(if event == Event::Allocate {
                Fault::Allocation
            } else {
                Fault::Runtime
            })
        } else {
            Ok(value)
        }
    });
    let result = executor.run();
    drop(executor);
    (result, events)
}

fn origins(code: &MethodCode, rendered: &Rendered) {
    for operation in code.operations() {
        assert_eq!(rendered.spans().iter().filter(|span| matches!(span.origin,
            Origin::Operation { operation: source, pc, .. } if source == operation.source && pc == operation.pc)).count(),1);
    }
}

fn main_method(name: &str, prototype: &str, check: impl FnOnce(&MethodCode, &Rendered)) {
    inspection::inspect(
        "composite/operation-records",
        "Lfixture/OperationRecords;",
        name,
        prototype,
        |_, code, rendered| {
            origins(code, rendered);
            check(code, rendered);
        },
    );
}

#[test]
fn a_constructor_failure_observes_the_value_committed_before_the_group() {
    main_method("protectedNew", "(I)I", |code, _| {
        assert_eq!(
            run(code, &[Value::Int(0)], None),
            (
                Ok(Value::Int(11)),
                vec![
                    Event::First(0),
                    Event::Allocate,
                    Event::Later,
                    Event::Gate(10, 20)
                ]
            )
        );
        assert_eq!(
            run(code, &[Value::Int(1)], None),
            (
                Ok(Value::Int(12)),
                vec![
                    Event::First(1),
                    Event::Allocate,
                    Event::Later,
                    Event::Gate(11, 20)
                ]
            )
        );
        assert_eq!(
            run(code, &[Value::Int(0)], Some(Event::First(0))),
            (Ok(Value::Int(0)), vec![Event::First(0)])
        );
        assert_eq!(
            run(code, &[Value::Int(0)], Some(Event::Later)),
            (
                Ok(Value::Int(10)),
                vec![Event::First(0), Event::Allocate, Event::Later]
            )
        );
        assert_eq!(
            run(code, &[Value::Int(0)], Some(Event::Gate(10, 20))),
            (
                Ok(Value::Int(10)),
                vec![
                    Event::First(0),
                    Event::Allocate,
                    Event::Later,
                    Event::Gate(10, 20)
                ]
            )
        );
        assert_eq!(
            run(code, &[Value::Int(0)], Some(Event::Allocate)),
            (
                Err(Fault::Allocation),
                vec![Event::First(0), Event::Allocate]
            )
        );
    });
}

#[test]
fn lock_completion_releases_once_after_results_and_helper_failures() {
    main_method("locked", "(Ljava/lang/Object;I)I", |code, rendered| {
        assert!(rendered.text().contains("synchronized ("));
        let lock = Value::Reference(Reference::Object(2));
        assert_eq!(
            run(code, &[lock, Value::Int(-11)], None),
            (
                Ok(Value::Int(-1)),
                vec![Event::Acquire, Event::First(-11), Event::Release]
            )
        );
        assert_eq!(
            run(code, &[lock, Value::Int(0)], None),
            (
                Ok(Value::Int(20)),
                vec![
                    Event::Acquire,
                    Event::First(0),
                    Event::Later,
                    Event::Release
                ]
            )
        );
        assert_eq!(
            run(code, &[lock, Value::Int(0)], Some(Event::First(0))),
            (
                Err(Fault::Runtime),
                vec![Event::Acquire, Event::First(0), Event::Release]
            )
        );
        assert_eq!(
            run(code, &[lock, Value::Int(0)], Some(Event::Later)),
            (
                Err(Fault::Runtime),
                vec![
                    Event::Acquire,
                    Event::First(0),
                    Event::Later,
                    Event::Release
                ]
            )
        );
        assert_eq!(
            run(
                code,
                &[Value::Reference(Reference::Null), Value::Int(0)],
                None
            ),
            (Err(Fault::NullLock), vec![])
        );
    });
}

#[test]
fn super_delegation_still_precedes_the_constructor_body() {
    inspection::inspect(
        "composite/operation-records",
        "Lfixture/RecordBox;",
        "<init>",
        "(II)V",
        |_, code, rendered| {
            origins(code, rendered);
            assert!(code.statements().any(|body| matches!(
                body.kind,
                StatementKind::Initialize {
                    syntax: InitializationSyntax::Super,
                    ..
                }
            )));
            assert!(
                rendered.text().find("super()").unwrap() < rendered.text().find("gate(").unwrap()
            );
        },
    );
}
