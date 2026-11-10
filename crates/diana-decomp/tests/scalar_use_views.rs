//! Range-proved use spellings retain the original scalar binding, not a new cast.
//! Inputs and independent scenarios: quality/fixtures/atoms/scalar-use-views/.

use diana_decomp::p4::Interpretation;
use diana_decomp::p7::{
    Arguments, ExprId, ExprKind, MemberDeclaration, MethodCode, ReferenceDeclaration,
};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn argument(code: &MethodCode, name: &str, parameters: &[&str], index: usize) -> ExprId {
    for expression in code.expressions() {
        let ExprKind::Call(call) = &expression.kind else {
            continue;
        };
        let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method {
            name: original,
            prototype,
            ..
        })) = &code.references()[call.target.0 as usize].declaration
        else {
            continue;
        };
        if original.units != name.encode_utf16().collect::<Vec<_>>()
            || prototype.parameters.len() != parameters.len()
            || !prototype
                .parameters
                .iter()
                .zip(parameters)
                .all(|(id, expected)| {
                    code.types()[id.0 as usize].units == expected.encode_utf16().collect::<Vec<_>>()
                })
        {
            continue;
        }
        let Arguments::Known(args) = &call.arguments else {
            panic!("known arguments")
        };
        return args[index];
    }
    panic!("original {name}{parameters:?}");
}

fn text(rendered: &Rendered, id: ExprId) -> &str {
    let spans: Vec<_> = rendered
        .spans()
        .iter()
        .filter(
            |span| matches!(span.origin, Origin::Expression { expression, .. } if expression == id),
        )
        .collect();
    assert_eq!(spans.len(), 1, "one supplied argument occurrence");
    &rendered.text()[spans[0].start.0..spans[0].end.0]
}

fn selected_view(code: &MethodCode, id: ExprId, expected: &str) {
    let (ExprKind::Read { view, .. } | ExprKind::Literal { view, .. }) =
        code.expressions()[id.0 as usize].kind
    else {
        panic!("leaf use")
    };
    let Interpretation::IntegralView { declared } = view else {
        panic!("range-proved view")
    };
    assert_eq!(
        code.types()[declared.0 as usize].units,
        expected.encode_utf16().collect::<Vec<_>>()
    );
}

#[test]
fn one_word_keeps_separate_character_and_integer_uses() {
    inspect(
        "atoms/scalar-use-views",
        "Lfixture/ScalarViews;",
        "carrier",
        "(Ljava/lang/String;)I",
        |_, code, rendered| {
            let character = argument(code, "probe", &["C"], 0);
            let integer = argument(code, "probe", &["I"], 0);
            selected_view(code, character, "C");
            let ExprKind::Read {
                storage: char_storage,
                ..
            } = code.expressions()[character.0 as usize].kind
            else {
                panic!()
            };
            let ExprKind::Read {
                storage: int_storage,
                ..
            } = code.expressions()[integer.0 as usize].kind
            else {
                panic!()
            };
            assert_eq!(char_storage, int_storage, "same original runtime word");
            assert!(text(rendered, character).contains("via char"));
            assert!(!text(rendered, integer).contains("via char"));
            assert!(text(rendered, character).contains(text(rendered, integer)));
        },
    );
}

#[test]
fn char_array_load_does_not_change_the_selected_int_append() {
    inspect(
        "atoms/scalar-use-views",
        "Lfixture/ScalarViews;",
        "array",
        "([C)Ljava/lang/String;",
        |_, code, rendered| {
            let character = argument(code, "append", &["C"], 0);
            let integer = argument(code, "append", &["I"], 0);
            selected_view(code, character, "C");
            assert!(text(rendered, character).contains("via char"));
            assert!(!text(rendered, integer).contains("via char"));
            assert!(rendered.text().contains("Char load"));
        },
    );
}

#[test]
fn exact_character_literals_are_not_unqualified_decimal_arguments() {
    inspect(
        "atoms/scalar-use-views",
        "Lfixture/ScalarViews;",
        "literal",
        "(Ljava/lang/String;)Ljava/lang/String;",
        |_, code, rendered| {
            for (index, spelling) in [(0, "'/'"), (1, "'-'")] {
                let id = argument(code, "replace", &["C", "C"], index);
                selected_view(code, id, "C");
                assert_eq!(text(rendered, id), spelling);
            }
        },
    );
}

#[test]
fn quote_and_isolated_surrogate_units_keep_exact_character_escaping() {
    inspect(
        "atoms/scalar-use-views",
        "Lfixture/ScalarViews;",
        "escaped",
        "(Ljava/lang/String;)Ljava/lang/String;",
        |_, code, rendered| {
            assert_eq!(
                text(rendered, argument(code, "replace", &["C", "C"], 0)),
                r"'\''"
            );
            assert_eq!(
                text(rendered, argument(code, "replace", &["C", "C"], 1)),
                r"'\ud800'"
            );
        },
    );
}

#[test]
fn signed_narrow_literals_keep_their_value_and_boolean_uses_stay_boolean() {
    inspect(
        "atoms/scalar-use-views",
        "Lfixture/ScalarViews;",
        "boundaries",
        "()I",
        |_, code, rendered| {
            for (descriptor, value, view) in
                [("B", "-128", "via byte"), ("S", "-32768", "via short")]
            {
                let id = argument(code, "probe", &[descriptor], 0);
                selected_view(code, id, descriptor);
                assert!(text(rendered, id).contains(value));
                assert!(text(rendered, id).contains(view));
            }
            assert!(rendered.text().contains("true"));
            assert!(rendered.text().contains("false"));
        },
    );
}

#[test]
fn a_declared_char_parameter_keeps_its_original_header_and_binding() {
    inspect(
        "atoms/scalar-use-views",
        "Lfixture/ScalarViews;",
        "parameter",
        "(C)I",
        |_, code, rendered| {
            let id = argument(code, "probe", &["C"], 0);
            selected_view(code, id, "C");
            assert!(rendered.text().contains("char p0"));
            assert!(!rendered.text().contains("unavailable"));
        },
    );
}

#[test]
fn a_protected_loop_phi_retains_its_distinct_char_and_int_call_phases() {
    inspect(
        "exceptions/scalar-views-protected-joins",
        "Lfixture/ProtectedViews;",
        "route",
        "([CIZ)I",
        |facts, code, rendered| {
            let character = argument(code, "record", &["C"], 0);
            let integer = argument(code, "record", &["I"], 0);
            selected_view(code, character, "C");
            assert!(text(rendered, character).contains("via char"));
            assert!(!text(rendered, integer).contains("via char"));
            assert!(text(rendered, character).contains(text(rendered, integer)));
            assert!(
                facts
                    .throws()
                    .iter()
                    .any(|site| !site.snapshots().is_empty())
            );
            assert!(code.copies().iter().any(|bundle| !bundle.steps.is_empty()));
            assert!(rendered.text().contains("IllegalStateException"));
            // Every actual call/capture has exactly one origin (inspection helper).
            // Protection may remain explicit; no new finally/while is an output gate.
        },
    );
}

#[test]
fn a_view_encloses_a_composed_producer_once_without_extending_its_operation_span() {
    inspect(
        "exceptions/scalar-views-protected-joins",
        "Lfixture/ProtectedViews;",
        "composed",
        "(I)I",
        |_, code, rendered| {
            let id = argument(code, "record", &["C"], 0);
            let composition = code.evaluation(id).expect("original adjacent producer");
            let producer = text(rendered, composition.expression);
            assert!(producer.contains("unit("));
            assert!(!producer.contains("via char"));
            assert!(text(rendered, id).ends_with(" via char)"));
            assert_eq!(rendered.text().matches("unit(").count(), 1);
            let spans: Vec<_> = rendered.spans().iter().filter(|span|
                matches!(span.origin, Origin::Operation { operation, .. } if operation == composition.operation)
            ).collect();
            assert_eq!(spans.len(), 1);
            let operation = &rendered.text()[spans[0].start.0..spans[0].end.0];
            assert_eq!(
                operation, producer,
                "producer range is inside the consumer view"
            );
        },
    );
}

#[test]
fn a_numeric_only_join_does_not_acquire_an_unrequested_character_view() {
    inspect(
        "exceptions/scalar-views-protected-joins",
        "Lfixture/ProtectedViews;",
        "raw",
        "([CZ)I",
        |_, code, rendered| {
            assert_eq!(
                code.expressions()
                    .iter()
                    .filter(|e| matches!(e.kind, ExprKind::Array { .. }))
                    .count(),
                2,
                "both selected source array reads remain; the Phi may coalesce"
            );
            assert!(!rendered.text().contains("via char"));
            assert!(rendered.text().contains("return "));
        },
    );
}

#[test]
fn a_char_declaration_without_range_evidence_does_not_invent_a_conversion() {
    inspect(
        "exceptions/scalar-views-protected-joins",
        "Lfixture/ProtectedViews;",
        "unproved",
        "(I)I",
        |_, code, rendered| {
            let id = argument(code, "record", &["C"], 0);
            let ExprKind::Read { view, .. } = code.expressions()[id.0 as usize].kind else {
                panic!()
            };
            assert!(!matches!(view, Interpretation::IntegralView { .. }));
            assert!(!rendered.text().contains("via char"));
            assert!(rendered.text().contains("record<(char)>"));
            assert!(
                !code
                    .expressions()
                    .iter()
                    .any(|e| matches!(e.kind, ExprKind::Unary { .. }))
            );
        },
    );
}

#[test]
fn a_captureless_retry_keeps_the_shared_original_handler_entry() {
    inspect(
        "exceptions/handler-retry-rejoin",
        "Lfixture/RetryRejoin;",
        "run",
        "(III)I",
        |facts, code, rendered| {
            assert_eq!(facts.handler_entries().len(), 2);
            assert!(
                facts
                    .handler_entries()
                    .iter()
                    .all(|h| h.capture().is_none())
            );
            let shared = facts
                .handler_entries()
                .iter()
                .find(|h| facts.links().iter().any(|link| link.to() == h.node()))
                .expect("the update is both a normal and exceptional destination");
            assert!(code.handlers().iter().any(|h| h.node == shared.node()));
            assert!(
                !code
                    .operations()
                    .iter()
                    .any(|op| matches!(op.original, diana_decomp::p2::OpKind::MoveException))
            );
            assert_eq!(rendered.text().matches("RetryRejoin.attempt(").count(), 1);
            assert_eq!(rendered.text().matches("RetryRejoin.reset(").count(), 1);
            // This preserves facts without freezing today's explicit graph shape.
        },
    );
}

fn call_operation(code: &MethodCode, name: &str) -> diana_decomp::p2::OpId {
    for operation in code.operations() {
        let Some(statement) = operation.statement else {
            continue;
        };
        let ExprKind::Call(call) = code.statement(statement).unwrap().operator(code) else {
            continue;
        };
        if let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method {
            name: original, ..
        })) = &code.references()[call.target.0 as usize].declaration
            && original.units == name.encode_utf16().collect::<Vec<_>>()
        {
            return operation.source;
        }
    }
    panic!("original call {name}");
}

#[test]
fn response_failures_snapshot_null_before_acquisition_and_the_handle_after_it() {
    inspect(
        "exceptions/response-close-snapshots",
        "Lfixture/ResponseClose;",
        "response",
        "(I)I",
        |_, code, rendered| {
            let chain = code
                .chains()
                .iter()
                .find(|c| c.clauses.len() == 3)
                .expect("ordered original handlers");
            let types: Vec<_> = chain
                .clauses
                .iter()
                .filter_map(|c| c.catch.as_ref())
                .map(|c| {
                    let id = c.descriptor.as_ref().unwrap();
                    String::from_utf16(&code.types()[id.0 as usize].units).unwrap()
                })
                .collect();
            assert_eq!(
                types,
                ["Ljava/io/InterruptedIOException;", "Ljava/io/IOException;"]
            );
            let acquire = call_operation(code, "acquire");
            let ready = call_operation(code, "ready");
            let statement = code
                .operations()
                .iter()
                .find(|op| op.source == acquire)
                .unwrap()
                .statement
                .unwrap();
            let diana_decomp::p5::Materialization::Store(handle) =
                code.statement(statement).unwrap().outputs[0].materialization()
            else {
                panic!("handle commit")
            };
            let acquire_sources: Vec<_> = code.emission().operations()[acquire.0 as usize]
                .snapshots
                .iter()
                .flat_map(|id| &code.copies()[id.0 as usize].steps)
                .filter_map(|step| {
                    if let diana_decomp::p7::CopyStatement::Assign { expression, .. } = step {
                        Some(*expression)
                    } else {
                        None
                    }
                })
                .collect();
            assert!(!acquire_sources.is_empty());
            assert!(acquire_sources.iter().all(|id|
                matches!(code.expressions()[id.0 as usize].kind, ExprKind::Literal { bits, view: Interpretation::Null } if bits.value == 0)));
            let ready_sources: Vec<_> = code.emission().operations()[ready.0 as usize]
                .snapshots
                .iter()
                .flat_map(|id| &code.copies()[id.0 as usize].steps)
                .filter_map(|step| {
                    if let diana_decomp::p7::CopyStatement::Assign { expression, .. } = step {
                        Some(*expression)
                    } else {
                        None
                    }
                })
                .collect();
            assert!(!ready_sources.is_empty());
            assert!(ready_sources.iter().all(|id|
                matches!(code.expressions()[id.0 as usize].kind, ExprKind::Read { storage, .. } if storage.0 == handle.0)));
            assert!(
                !rendered.text().contains("addSuppressed"),
                "ordinary finally replaces a pending failure"
            );
        },
    );
}

#[test]
fn resource_suppression_keeps_primary_and_secondary_reads_distinct() {
    inspect(
        "exceptions/response-close-snapshots",
        "Lfixture/ResponseClose;",
        "resource",
        "(I)I",
        |_, code, rendered| {
            let secondary = argument(code, "addSuppressed", &["Ljava/lang/Throwable;"], 0);
            let call = code
                .expressions()
                .iter()
                .find_map(|e| {
                    let ExprKind::Call(call) = &e.kind else {
                        return None;
                    };
                    let Arguments::Known(args) = &call.arguments else {
                        return None;
                    };
                    (args.as_slice() == [secondary]).then_some(call)
                })
                .unwrap();
            let primary = call.receiver.expect("original primary throwable receiver");
            assert_ne!(text(rendered, primary), text(rendered, secondary));
            assert_eq!(rendered.text().matches(".addSuppressed(").count(), 1);
            assert!(
                rendered
                    .text()
                    .contains(&format!("throw {}", text(rendered, primary)))
            );
        },
    );
}
