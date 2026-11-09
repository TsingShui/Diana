//! Ordinary-Engine concat presentation checks and independent content scenarios.
//! The content evaluator covers parameter/constant projections, not whole methods.

use diana_decomp::p2::{InvokeKind, OpKind};
use diana_decomp::p3::EntryRole;
use diana_decomp::p5::LocalKind;
use diana_decomp::p7::*;

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn custom(code: &MethodCode) -> &Call {
    assert!(
        code.issues()
            .iter()
            .all(|issue| !matches!(issue, Issue::Input { .. })),
        "fixture input must be structurally valid"
    );
    let calls: Vec<_> = code
        .expressions()
        .iter()
        .filter_map(|e| match &e.kind {
            ExprKind::Call(call) if call.kind == InvokeKind::Custom => Some(call),
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 1, "one original custom invocation");
    calls[0]
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    String(Vec<u16>),
    NullString,
    Number(i64),
    Char(u16),
    Boolean(bool),
}
fn string(text: &str) -> Value {
    Value::String(text.encode_utf16().collect())
}
fn content(value: &Value) -> Vec<u16> {
    match value {
        Value::String(units) => units.clone(),
        Value::NullString => "null".encode_utf16().collect(),
        Value::Number(value) => value.to_string().encode_utf16().collect(),
        Value::Char(value) => vec![*value],
        Value::Boolean(value) => value.to_string().encode_utf16().collect(),
    }
}
fn add(left: Value, right: Value) -> Result<Value, &'static str> {
    if matches!(left, Value::String(_) | Value::NullString)
        || matches!(right, Value::String(_) | Value::NullString)
    {
        let mut units = content(&left);
        units.extend(content(&right));
        return Ok(Value::String(units));
    }
    let number = |value| match value {
        Value::Number(n) => Some(n),
        Value::Char(n) => Some(i64::from(n)),
        _ => None,
    };
    match (number(left), number(right)) {
        (Some(a), Some(b)) => Ok(Value::Number(a.wrapping_add(b))),
        _ => Err("unsupported numeric/boolean addition"),
    }
}
fn value(code: &MethodCode, id: ExprId, parameters: &[Value]) -> Result<Value, &'static str> {
    if let Some(composed) = code.emission().composed_use(id) {
        return value(code, composed.expression, parameters);
    }
    match &code.expressions()[id.0 as usize].kind {
        ExprKind::Read { storage, .. } => {
            let LocalKind::Parameter {
                role: EntryRole::Parameter(index),
                ..
            } = code.locals()[storage.0 as usize].original.kind()
            else {
                return Err("projection evaluator does not execute local stores");
            };
            parameters
                .get(index as usize)
                .cloned()
                .ok_or("missing scenario parameter")
        }
        ExprKind::String(reference) => {
            let Ok(ReferenceDeclaration::String { units }) =
                &code.references()[reference.0 as usize].declaration
            else {
                return Err("missing String reference");
            };
            Ok(Value::String(units.clone()))
        }
        _ => Err("unmodeled operand expression"),
    }
}
fn projected(code: &MethodCode, parameters: &[Value]) -> Result<Vec<u16>, &'static str> {
    let Some(ConcatOutcome::Inferred(plan)) = custom(code).concatenation() else {
        return Err("explicit call has no inferred projection");
    };
    let mut result = plan.empty_prefix().then(|| string(""));
    for part in plan.parts() {
        let item = match part {
            ConcatPart::Argument {
                expression,
                conversion,
            } => {
                let v = value(code, *expression, parameters)?;
                match (conversion, v) {
                    (ConcatConversion::Value, v) => v,
                    (ConcatConversion::Char, Value::Char(v)) => Value::Char(v),
                    (ConcatConversion::Char, Value::Number(v)) => Value::Char(v as u16),
                    (ConcatConversion::Int, Value::Char(v)) => Value::Number(i64::from(v)),
                    _ => return Err("unmodeled conversion"),
                }
            }
            ConcatPart::Text {
                reference,
                units: range,
            } => {
                let Ok(ReferenceDeclaration::String { units }) =
                    &code.references()[reference.0 as usize].declaration
                else {
                    return Err("missing recipe text");
                };
                Value::String(
                    units
                        .get(range.clone())
                        .ok_or("invalid text range")?
                        .to_vec(),
                )
            }
            ConcatPart::Constant { units, .. } => Value::String(units.clone()),
        };
        result = Some(match result {
            Some(left) => add(left, item)?,
            None => item,
        });
    }
    match result {
        Some(Value::String(units)) => Ok(units),
        _ => Err("printed expression is not a String result"),
    }
}

#[test]
fn original_concat_calls_keep_their_binding_and_readable_body() {
    for (name, prototype, parts) in [
        ("simple", "(Ljava/lang/String;I)Ljava/lang/String;", 3),
        (
            "many",
            "(Ljava/lang/String;Ljava/lang/String;IJC)Ljava/lang/String;",
            9,
        ),
        ("insideLoop", "([Ljava/lang/String;)Ljava/lang/String;", 4),
    ] {
        inspect(
            "composite/complex-indy-concat",
            "Lfixture/ComplexIndyConcat;",
            name,
            prototype,
            |_, code, rendered| {
                let call = custom(code);
                let Some(ConcatOutcome::Inferred(plan)) = call.concatenation() else {
                    panic!("{name}: expected concat presentation");
                };
                assert_eq!(plan.parts().len(), parts);
                assert!(matches!(
                    code.references()[call.target.0 as usize].declaration,
                    Ok(ReferenceDeclaration::CallSite { .. })
                ));
                assert!(
                    rendered
                        .text()
                        .contains("Diana: inferred StringConcatFactory")
                );
                assert!(rendered.text().contains("call_site r"));
                assert!(!rendered.text().contains("invoke<Custom>"));
                if name == "many" {
                    assert!(rendered.text().contains("((char) "));
                }
                if name == "insideLoop" {
                    assert!(rendered.text().contains("while ("));
                }
            },
        );
    }
}

#[test]
fn parameter_and_recipe_content_matches_independent_scenarios() {
    let mut lone_surrogate = "tag[null:-3]".encode_utf16().collect::<Vec<_>>();
    lone_surrogate.extend([0xd800, 1]);
    let cases = [
        (
            "numeric",
            "(IICZJ)Ljava/lang/String;",
            vec![
                Value::Number(1),
                Value::Number(2),
                Value::Char(65),
                Value::Boolean(true),
                Value::Number(3),
            ],
            "12Atrue3".encode_utf16().collect(),
        ),
        (
            "recipe",
            "(Ljava/lang/String;IC)Ljava/lang/String;",
            vec![string("x"), Value::Number(2), Value::Char(65)],
            "tag[x:2]A\u{1}".encode_utf16().collect(),
        ),
        (
            "recipe",
            "(Ljava/lang/String;IC)Ljava/lang/String;",
            vec![Value::NullString, Value::Number(-3), Value::Char(0xd800)],
            lone_surrogate,
        ),
        (
            "constants",
            "(Ljava/lang/String;)Ljava/lang/String;",
            vec![string("x")],
            "x7trueA".encode_utf16().collect(),
        ),
        (
            "single",
            "(Ljava/lang/String;)Ljava/lang/String;",
            vec![Value::NullString],
            "null".encode_utf16().collect(),
        ),
        (
            "charAsInt",
            "(C)Ljava/lang/String;",
            vec![Value::Char(65)],
            "65".encode_utf16().collect(),
        ),
    ];
    for (name, prototype, values, expected) in cases {
        inspect(
            "composite/concat-capture-recipes",
            "Lfixture/ConcatCapture;",
            name,
            prototype,
            |_, code, rendered| {
                assert_eq!(projected(code, &values).unwrap(), expected, "{name}");
                assert!(
                    rendered
                        .text()
                        .contains("Diana: inferred StringConcatFactory")
                );
            },
        );
    }
}

#[test]
fn the_content_model_rejects_numeric_addition_and_wrong_char_interpretation() {
    assert_ne!(
        content(&add(Value::Number(1), Value::Number(2)).unwrap()),
        content(&add(string("1"), Value::Number(2)).unwrap())
    );
    assert_ne!(content(&Value::Number(65)), content(&Value::Char(65)));
    assert_ne!(Value::NullString, string("null"));
}

#[test]
fn captured_helpers_are_not_relocated_into_concat_operands() {
    inspect(
        "composite/concat-capture-recipes",
        "Lfixture/ConcatCapture;",
        "effectful",
        "(I)Ljava/lang/String;",
        |_, code, rendered| {
            assert!(matches!(
                custom(code).concatenation(),
                Some(ConcatOutcome::Inferred(_))
            ));
            let helpers: Vec<_> = code
                .operations()
                .iter()
                .filter(|op| {
                    matches!(
                        op.original,
                        OpKind::Invoke {
                            kind: InvokeKind::Static,
                            ..
                        }
                    )
                })
                .collect();
            assert_eq!(helpers.len(), 2);
            assert!(
                helpers
                    .iter()
                    .all(|op| !code.emission().composed_operation(op.source))
            );
            let text = rendered.text();
            assert!(
                text.find("ConcatCapture.read(0, p0)").unwrap()
                    < text.find("ConcatCapture.read(1, p0)").unwrap()
            );
            assert!(
                text.find("ConcatCapture.read(1, p0)").unwrap()
                    < text.find("Diana: inferred StringConcatFactory").unwrap()
            );
        },
    );
}

#[test]
fn loop_catch_and_machine_word_views_keep_their_original_work() {
    for (name, prototype) in [
        ("loop", "(II)Ljava/lang/String;"),
        ("moved", "(CZ)Ljava/lang/String;"),
        ("floating", "(FD)Ljava/lang/String;"),
    ] {
        inspect(
            "composite/concat-capture-recipes",
            "Lfixture/ConcatCapture;",
            name,
            prototype,
            |_, code, rendered| {
                assert!(
                    matches!(
                        custom(code).concatenation(),
                        Some(ConcatOutcome::Inferred(_))
                    ),
                    "{name}"
                );
                if name == "loop" {
                    assert!(
                        rendered
                            .text()
                            .contains("catch (java.lang.IllegalStateException ")
                            || rendered
                                .text()
                                .contains("catch (java.lang.IllegalStateException)")
                    );
                    assert!(rendered.text().contains("while ("));
                }
                if name == "moved" {
                    assert!(rendered.text().contains("((char) "));
                    assert!(rendered.text().contains(" != 0)"));
                }
            },
        );
    }
}

#[test]
fn unavailable_protocol_or_argument_views_remain_local_explicit_calls() {
    for (name, prototype, reason) in [
        (
            "object",
            "(Ljava/lang/Object;I)Ljava/lang/String;",
            ConcatUnavailable::ParameterType,
        ),
        (
            "nonfiniteLiteral",
            "()Ljava/lang/String;",
            ConcatUnavailable::ArgumentView,
        ),
        (
            "badRecipe",
            "(Ljava/lang/String;I)Ljava/lang/String;",
            ConcatUnavailable::RecipeArity,
        ),
        (
            "badBootstrap",
            "(Ljava/lang/String;)Ljava/lang/String;",
            ConcatUnavailable::BootstrapSignature,
        ),
    ] {
        inspect(
            "composite/concat-capture-recipes",
            "Lfixture/ConcatCapture;",
            name,
            prototype,
            |_, code, rendered| {
                let Some(ConcatOutcome::Explicit(actual)) = custom(code).concatenation() else {
                    panic!("{name}: explicit reason required");
                };
                assert_eq!(
                    std::mem::discriminant(actual),
                    std::mem::discriminant(&reason)
                );
                assert!(rendered.text().contains("invoke<Custom>"));
                assert!(!rendered.text().contains("Diana: inferred"));
            },
        );
    }
    inspect(
        "composite/concat-capture-recipes",
        "Lfixture/ConcatCapture;",
        "unknownFactory",
        "(Ljava/lang/String;I)Ljava/lang/String;",
        |_, code, rendered| {
            assert!(custom(code).concatenation().is_none());
            assert!(rendered.text().contains("invoke<Custom>"));
        },
    );
}
