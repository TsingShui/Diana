//! Surface recovery and independent final-plan fixture execution.
use diana_decomp::p2::OpKind;
use diana_decomp::p7::*;
use diana_decomp::p9::{Origin, Rendered};
#[path = "support/sync_execution.rs"]
mod execution;
#[path = "support/inspection.rs"]
mod inspection;
use execution::{Effect::*, Failure, Model, Value};
use inspection::inspect;
fn original_events(code: &MethodCode, rendered: &Rendered) {
    for operation in code.operations().iter().filter(|op| {
        matches!(
            op.original,
            OpKind::Monitor { .. } | OpKind::Invoke { .. } | OpKind::MoveException
        )
    }) {
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(|span| matches!(span.origin,
            Origin::Operation { operation: id, .. } if id == operation.source))
                .count(),
            1
        );
    }
}
fn implicit_count(code: &MethodCode) -> usize {
    code.controls()
        .iter()
        .filter(|control| {
            matches!(
                control.layout,
                EmissionLayout::Synchronized { implicit: true, .. }
            )
        })
        .count()
}
#[test]
fn simple_monitor_cleanup_is_implicit_and_releases_once_on_every_modeled_path() {
    inspect(
        "sync/implicit-monitor-cleanup",
        "Lfixture/MonitorSurface;",
        "counts",
        "()I",
        |facts, code, rendered| {
            assert_eq!(implicit_count(code), 1);
            assert!(facts.synchronized()[0].implicit().is_some());
            assert!(!rendered.text().contains("release synchronized"));
            assert!(!rendered.text().contains("capture_exception"));
            assert!(!rendered.text().contains("catch ("));
            original_events(code, rendered);
            for exit in facts.synchronized()[0].exits() {
                let span = rendered
                    .spans()
                    .iter()
                    .find(|span| {
                        matches!(span.origin,
                Origin::Operation { operation, .. } if operation == exit.release())
                    })
                    .unwrap();
                assert_eq!(
                    span.start, span.end,
                    "boundary owns the release; source event remains"
                );
            }
            for (queue_a, queue_s, fault, expected, effects) in [
                (
                    10,
                    11,
                    None,
                    Ok(Value::Int(5)),
                    vec![
                        Enter(1),
                        Field("A"),
                        Size("A"),
                        Field("S"),
                        Size("S"),
                        Exit(1),
                    ],
                ),
                (
                    10,
                    11,
                    Some(10),
                    Err(Failure::Thrown(110)),
                    vec![Enter(1), Field("A"), Size("A"), Exit(1)],
                ),
                (
                    10,
                    11,
                    Some(11),
                    Err(Failure::Thrown(111)),
                    vec![
                        Enter(1),
                        Field("A"),
                        Size("A"),
                        Field("S"),
                        Size("S"),
                        Exit(1),
                    ],
                ),
                (
                    0,
                    11,
                    None,
                    Err(Failure::Thrown(99)),
                    vec![Enter(1), Field("A"), Exit(1)],
                ),
                (
                    10,
                    0,
                    None,
                    Err(Failure::Thrown(99)),
                    vec![Enter(1), Field("A"), Size("A"), Field("S"), Exit(1)],
                ),
            ] {
                let mut model = Model::counts();
                model.queue_a = queue_a;
                model.queue_s = queue_s;
                model.size_fault = fault;
                assert_eq!(model.execute(code), expected);
                assert_eq!(model.effects, effects);
                assert!(model.held.is_empty());
            }
        },
    );
}
#[test]
fn catch_header_binds_the_exact_error_before_note_and_preserves_note_failure() {
    inspect(
        "sync/implicit-monitor-cleanup",
        "Lfixture/MonitorSurface;",
        "caught",
        "(I)I",
        |facts, code, rendered| {
            assert!(
                facts
                    .tries()
                    .iter()
                    .any(|p| p.bindings().iter().any(Option::is_some))
            );
            assert!(rendered.text().contains("catch (all v"));
            assert!(!rendered.text().contains("capture_exception"));
            original_events(code, rendered);
            for (mode, note_fault, result, effects) in [
                (0, false, Ok(Value::Int(7)), vec![Read(0)]),
                (1, false, Ok(Value::Int(-1)), vec![Read(1), Note(100)]),
                (1, true, Err(Failure::Thrown(201)), vec![Read(1), Note(100)]),
            ] {
                let mut model = Model::new(vec![Value::Int(mode)]);
                model.note_fault = note_fault;
                assert_eq!(model.execute(code), result);
                assert_eq!(model.effects, effects);
            }
        },
    );
}
#[test]
fn nested_and_reentrant_acquisitions_preserve_each_original_release_order() {
    inspect(
        "sync/monitor-surface-boundaries",
        "Lfixture/MonitorBoundaries;",
        "nested",
        "(Ljava/lang/Object;Ljava/lang/Object;I)I",
        |_, code, rendered| {
            assert_eq!(implicit_count(code), 2);
            original_events(code, rendered);
            assert!(!rendered.text().contains("release synchronized"));
            assert!(!rendered.text().contains("capture_exception"));
            for (outer, inner, mode, result, effects) in [
                (
                    1,
                    2,
                    0,
                    Ok(Value::Int(9)),
                    vec![
                        Enter(1),
                        Read(0),
                        Enter(2),
                        Read(1),
                        Exit(2),
                        Read(2),
                        Exit(1),
                    ],
                ),
                (
                    1,
                    1,
                    0,
                    Ok(Value::Int(9)),
                    vec![
                        Enter(1),
                        Read(0),
                        Enter(1),
                        Read(1),
                        Exit(1),
                        Read(2),
                        Exit(1),
                    ],
                ),
                (
                    1,
                    2,
                    1,
                    Err(Failure::Thrown(100)),
                    vec![Enter(1), Read(0), Exit(1)],
                ),
                (
                    1,
                    2,
                    2,
                    Err(Failure::Thrown(101)),
                    vec![Enter(1), Read(0), Enter(2), Read(1), Exit(2), Exit(1)],
                ),
                (
                    1,
                    2,
                    4,
                    Err(Failure::Thrown(102)),
                    vec![
                        Enter(1),
                        Read(0),
                        Enter(2),
                        Read(1),
                        Exit(2),
                        Read(2),
                        Exit(1),
                    ],
                ),
                (0, 2, 0, Err(Failure::Thrown(99)), vec![Enter(0)]),
                (
                    1,
                    0,
                    0,
                    Err(Failure::Thrown(99)),
                    vec![Enter(1), Read(0), Enter(0), Exit(1)],
                ),
            ] {
                let mut model =
                    Model::new(vec![Value::Ref(outer), Value::Ref(inner), Value::Int(mode)]);
                assert_eq!(model.execute(code), result);
                assert_eq!(model.effects, effects);
                assert!(model.held.is_empty());
            }
        },
    );
}
#[test]
fn typed_recovery_keeps_its_real_catch_and_implicit_cleanup_handles_other_errors() {
    inspect(
        "sync/monitor-surface-boundaries",
        "Lfixture/MonitorBoundaries;",
        "innerCatch",
        "(Ljava/lang/Object;I)I",
        |_, code, rendered| {
            assert_eq!(implicit_count(code), 1);
            original_events(code, rendered);
            assert!(rendered.text().contains("catch ("));
            assert!(rendered.text().contains("IllegalStateException v"));
            assert!(!rendered.text().contains("catch (all"));
            assert!(!rendered.text().contains("exception_chain"));
            for (mode, other, expected) in [
                (0, false, Ok(Value::Int(2))),
                (1, false, Ok(Value::Int(10))),
                (0, true, Err(Failure::Thrown(200))),
            ] {
                let mut model = Model::new(vec![Value::Ref(1), Value::Int(mode)]);
                model.other_read_fault = other;
                assert_eq!(model.execute(code), expected);
                assert_eq!(model.effects, vec![Enter(1), Read(0), Exit(1)]);
                assert!(model.held.is_empty());
            }
        },
    );
}
#[test]
fn extra_work_changed_error_multiple_returns_and_unknown_lock_keep_explicit_events() {
    for (name, proto) in [
        ("extra", "(Ljava/lang/Object;I)I"),
        (
            "otherException",
            "(Ljava/lang/Object;Ljava/lang/Throwable;I)I",
        ),
        ("twoReturns", "(Ljava/lang/Object;I)I"),
        ("wrongLock", "(Ljava/lang/Object;Ljava/lang/Object;I)I"),
    ] {
        inspect(
            "sync/monitor-surface-boundaries",
            "Lfixture/MonitorBoundaries;",
            name,
            proto,
            |_, code, rendered| {
                assert_eq!(implicit_count(code), 0);
                original_events(code, rendered);
                assert!(
                    code.operations()
                        .iter()
                        .filter(|op| matches!(op.original, OpKind::Monitor { enter: false }))
                        .all(|op| !code.emission().origin_only_operation(op.source))
                );
                match name {
                    "extra" => assert!(rendered.text().contains("note(")),
                    "otherException" => assert!(rendered.text().contains("throw p1;")),
                    "twoReturns" => assert!(rendered.text().contains("+ 100")),
                    "wrongLock" => assert!(rendered.text().contains("monitor_exit p1")),
                    _ => unreachable!(),
                }
            },
        );
    }
}
#[test]
fn a_capture_observed_at_the_shared_join_keeps_its_original_assignment() {
    inspect(
        "sync/monitor-surface-boundaries",
        "Lfixture/MonitorBoundaries;",
        "escaped",
        "(I)Ljava/lang/Throwable;",
        |facts, code, rendered| {
            assert!(
                facts
                    .tries()
                    .iter()
                    .all(|p| p.bindings().iter().all(Option::is_none))
            );
            assert!(rendered.text().contains("capture_exception"));
            original_events(code, rendered);
            for (mode, expected) in [(0, Value::Ref(0)), (1, Value::Ref(100))] {
                let mut model = Model::new(vec![Value::Int(mode)]);
                assert_eq!(model.execute(code), Ok(expected));
                assert_eq!(model.effects, vec![Read(0)]);
            }
        },
    );
}
