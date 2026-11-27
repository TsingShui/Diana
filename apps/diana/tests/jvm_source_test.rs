use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

const OWNER: &str = "Lfixture/jvm/SourceRecovery;";

fn fixture() -> TempDir {
    let output = tempfile::tempdir().unwrap();
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/source-recovery/prepare.py");
    let script = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(recipe)
        .arg(output.path())
        .output()
        .expect("run the JVM source fixture recipe with Python 3 and JDK 17+");
    assert!(
        script.status.success(),
        "{}",
        String::from_utf8_lossy(&script.stderr)
    );
    output
}

fn source(command: &str, input: &Path, selector: &str, format: &str, exit: i32) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg(command)
        .arg(input)
        .args([selector, "--format", format])
        .output()
        .expect("run Diana");
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{command} {selector}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn json_source(command: &str, input: &Path, selector: &str, exit: i32) -> Value {
    serde_json::from_slice(&source(command, input, selector, "json", exit).stdout).unwrap()
}

fn text(result: &Value) -> &str {
    result["result"]["text"].as_str().unwrap()
}

fn method(input: &Path, name_and_prototype: &str, exit: i32) -> Value {
    json_source(
        "get-method",
        input,
        &format!("{OWNER}->{name_and_prototype}"),
        exit,
    )
}

fn coordinate(value: &Value) -> &str {
    let at = value.as_str().unwrap();
    at.split_once("::").map_or(at, |(position, _)| position)
}

fn assert_contains(text: &str, parts: &[&str]) {
    for part in parts {
        assert!(text.contains(part), "missing {part:?} in:\n{text}");
    }
}

#[test]
fn get_method_recovers_the_same_original_method_by_name_and_position() {
    let fixture = fixture();
    let inputs = [
        ("classes/fixture/jvm/SourceRecovery.class", "j0"),
        ("source.jar", "j1"),
        ("bundle.jar", "j1/j0"),
    ];
    for (file, class_at) in inputs {
        let input = fixture.path().join(file);
        let by_name = method(&input, "calculate(I)I", 0);
        let method_at = format!("{class_at}:m1");
        let by_position = json_source("get-method", &input, &method_at, 0);
        assert_eq!(coordinate(&by_name["selection"]["class_at"]), class_at);
        assert_eq!(coordinate(&by_name["selection"]["method_at"]), method_at);
        assert_eq!(by_name["selection"]["input_kind"], "jvm");
        assert_eq!(by_name["selection"]["class"], OWNER);
        assert_eq!(by_name["selection"]["method"], 1);
        let original = by_name["result"]["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["ordinal"] == 1)
            .unwrap();
        assert_eq!(original["name"], "calculate");
        assert_eq!(original["prototype"], "(I)I");
        assert_eq!(original["access_flags"], 0x0001);
        assert_eq!(coordinate(&original["at"]), method_at);
        assert_eq!(text(&by_name), text(&by_position));
        assert!(by_name["issues"].as_array().unwrap().is_empty());
        assert_contains(
            text(&by_name),
            &["calculate(", ".value", "+", "twice(", "- 1", "return"],
        );
        let display = source("get-method", &input, &method_at, "text", 0);
        assert_eq!(
            String::from_utf8(display.stdout).unwrap().trim_end(),
            text(&by_name).trim_end()
        );
    }
}

#[test]
fn get_method_recovers_constructor_branches_loop_and_typed_catch() {
    let fixture = fixture();
    let input = fixture.path().join("source.jar");

    let constructor = method(&input, "<init>(I)V", 0);
    assert_contains(text(&constructor), &["SourceRecovery(", ".value", " = "]);
    if let Some(super_call) = text(&constructor).find("super(") {
        assert!(super_call < text(&constructor).find(".value").unwrap());
    }

    let branch = method(&input, "choose(I)I", 0);
    assert_contains(text(&branch), &["if (", ".value", " + ", " - ", "return"]);

    let loop_body = method(&input, "descendingSum(I)I", 0);
    assert_contains(text(&loop_body), &["while (", "return"]);
    assert!(text(&loop_body).contains("+=") || text(&loop_body).contains(" + "));
    assert!(text(&loop_body).contains("--") || text(&loop_body).contains(" - 1"));

    let handler = method(&input, "divideOrDefault(II)I", 0);
    assert_contains(
        text(&handler),
        &["try", " / ", "catch (", "ArithmeticException", "-7"],
    );

    let concat = method(&input, "update(I)I", 0);
    assert_contains(
        text(&concat),
        &[".value", ".last", "\"value=\"", "+", "return"],
    );
}

#[test]
fn get_method_follows_the_switch_table_default_and_shared_targets() {
    let fixture = fixture();
    let input = fixture.path().join("source.jar");
    let switch = method(&input, "classify(I)I", 0);
    assert_contains(
        text(&switch),
        &[
            "switch (", "case -7", "40", "case 6", "case 100", "default", " + 200",
        ],
    );
}

#[test]
fn get_method_preserves_paths_from_overlapping_jvm_exception_ranges() {
    let fixture = fixture();
    let input = fixture
        .path()
        .join("classes/fixture/jvm/ExceptionChains.class");
    for (name, normal_effect) in [("guarded", "mark(4)"), ("guardedFinally", "mark(3)")] {
        let selector = format!("Lfixture/jvm/ExceptionChains;->{name}([II)I");
        let result = json_source("get-method", &input, &selector, 0);
        assert!(result["issues"].as_array().unwrap().is_empty());
        assert_contains(
            text(&result),
            &[
                "[0]",
                " / ",
                "ArithmeticException",
                "ArrayIndexOutOfBoundsException",
                "mark(2)",
                normal_effect,
                "mark(5)",
                "-7",
                "-8",
            ],
        );
    }
}

#[test]
fn get_class_and_get_method_publish_local_gaps_and_keep_nearby_effects() {
    let fixture = fixture();
    let input = fixture.path().join("opaque-bundle.jar");
    let selected = method(&input, "update(I)I", 1);
    let whole = json_source("get-class", &input, "fixture.jvm.SourceRecovery", 1);
    let by_position = json_source("get-class", &input, "j1/j0", 1);
    assert_eq!(text(&whole), text(&by_position));
    assert_eq!(coordinate(&whole["selection"]["class_at"]), "j1/j0");
    assert!(whole["selection"]["method_at"].is_null());
    assert_contains(
        text(&whole),
        &["class SourceRecovery", "divideOrDefault(", "last()"],
    );

    for report in [&selected, &whole] {
        let issues = report["issues"].as_array().unwrap();
        assert_eq!(issues.len(), 1, "unexpected coverage loss: {issues:?}");
        assert!(
            issues.iter().any(|issue| {
                issue["context"]
                    .as_str()
                    .is_some_and(|context| context.starts_with("j1/j0:m6"))
                    && issue["range_bytes"] == serde_json::json!([15, 20])
                    && issue["reason"].as_str().is_some_and(|reason| {
                        reason.contains("Lfixture/jvm/CustomBootstrap;->makeConcatWithConstants")
                    })
            }),
            "missing local concatenation issue: {issues:?}"
        );
    }

    let body = text(&selected);
    assert!(!body.contains("Failed to decompile this method"));
    let mut rest = body;
    for effect in [".value", "this.last =", ".value", "return"] {
        rest = rest
            .split_once(effect)
            .unwrap_or_else(|| panic!("missing ordered effect {effect} in:\n{body}"))
            .1;
    }
    assert_contains(body, &[" + ", " - 2"]);

    let display = source("get-class", &input, "j1/j0", "text", 1);
    let display = String::from_utf8(display.stdout).unwrap();
    assert_eq!(display, text(&whole));
}

#[test]
fn physical_selection_keeps_each_variant_and_avoids_unrelated_catalog_failures() {
    let fixture = fixture();
    let multi_release = fixture.path().join("multi-release.jar");
    let ambiguous = source("get-class", &multi_release, OWNER, "json", 4);
    assert!(ambiguous.stdout.is_empty());
    for class_at in ["j1", "j2"] {
        let method_at = format!("{class_at}:m1");
        let data = json_source("get-method", &multi_release, &method_at, 0);
        assert_eq!(coordinate(&data["selection"]["method_at"]), method_at);
    }

    let damaged = fixture.path().join("damaged-bundle.jar");
    let named = method(&damaged, "calculate(I)I", 1);
    let physical = json_source("get-method", &damaged, "j1/j0:m1", 0);
    assert!(text(&named).starts_with(text(&physical)));
    assert!(text(&named).contains("selection at j0"));
    assert_eq!(named["issues"][0]["context"], "j0");
    assert!(physical["issues"].as_array().unwrap().is_empty());
}
