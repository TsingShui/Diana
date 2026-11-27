use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/annotations/prepare.py");
    let result = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(recipe)
        .arg(directory.path())
        .output()
        .expect("Python and OpenJDK for the fixture");
    assert_exit(&result, 0);
    directory
}

fn call(command: &str, input: &Path, flags: &[&str], exit: i32) -> Output {
    let result = Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg(command)
        .arg(input)
        .args(flags)
        .output()
        .expect("run Diana");
    assert_exit(&result, exit);
    result
}

fn assert_exit(result: &Output, exit: i32) {
    assert_eq!(
        result.status.code(),
        Some(exit),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
    );
}

fn json(command: &str, input: &Path, flags: &[&str], exit: i32) -> Value {
    let mut args = flags.to_vec();
    args.extend(["--format", "json"]);
    serde_json::from_slice(&call(command, input, &args, exit).stdout).unwrap()
}

fn find(input: &Path, kind: &str, target: &str) -> Value {
    json(
        "xref",
        input,
        &[kind, target, "--exact", "--no-limit", "--jobs", "4"],
        0,
    )
}

fn row(value: &Value, at: &str, role: &str) {
    assert!(
        value["items"].as_array().unwrap().iter().any(|row| {
            row["context"] == at
                && row["offset_byte"] == ""
                && row["use"].as_str().unwrap().contains(role)
                && row["use"].as_str().unwrap().contains("class-file byte ")
        }),
        "missing {at} {role}: {value:#}"
    );
}

#[test]
fn source_keeps_visible_invisible_nested_and_parameter_annotations() {
    let fixture = fixture();
    let input = fixture.path().join("annotations.jar");
    let doc = json("get-class", &input, &["j0"], 0);
    let source = doc["result"]["text"].as_str().unwrap();
    for part in [
        "@Rich(",
        "text = \"class-visible-marker\"",
        "mode = Mode.BETA",
        "int[].class",
        "numbers = {1, -2, 3}",
        "@Nested(label = \"nested-marker\")",
        "123456789012L",
        "enabled = true",
        "class-invisible-marker",
        "field-constant-marker",
        "field-marker",
        "method-marker",
        "first-parameter-marker",
        "second-parameter-marker",
        "return ",
    ] {
        assert!(source.contains(part), "{part} not in {source}");
    }
    assert_eq!(doc["issues"].as_array().unwrap().len(), 0);
    let selected = json("get-method", &input, &["j0:m1"], 0);
    let source = selected["result"]["text"].as_str().unwrap();
    assert!(
        source.contains("first-parameter-marker") && source.contains("second-parameter-marker")
    );
}

#[test]
fn declaration_xrefs_distinguish_annotation_paths_and_constant_values() {
    let fixture = fixture();
    let input = fixture.path().join("annotations.jar");
    row(
        &find(&input, "string", "class-visible-marker"),
        "j0",
        "runtime-visible annotation-string",
    );
    row(
        &find(&input, "string", "class-invisible-marker"),
        "j0",
        "runtime-invisible annotation-string",
    );
    row(
        &find(&input, "string", "nested-marker"),
        "j0",
        "annotation-string",
    );
    row(
        &find(&input, "type", "Lfixture/annotations/Nested;"),
        "j0",
        "annotation-type",
    );
    row(
        &find(
            &input,
            "field",
            "Lfixture/annotations/Mode;->BETA:Lfixture/annotations/Mode;",
        ),
        "j0",
        "annotation-enum-field",
    );
    row(
        &find(&input, "string", "field-constant-marker"),
        "j0:f0",
        "static-value",
    );
    row(
        &find(&input, "string", "field-marker"),
        "j0:f1",
        "annotation-string",
    );
    row(
        &find(&input, "string", "second-parameter-marker"),
        "j0:m1",
        "parameter-annotation[1]-string",
    );
    row(
        &find(&input, "string", "annotation-default-marker"),
        "j2:m0",
        "annotation-default-string",
    );
    row(
        &find(&input, "string", "type-use-marker"),
        "j4:m1",
        "type-use-annotation[0x14]-string",
    );

    let typed = json("get-class", &input, &["j4"], 1);
    assert!(
        typed["result"]["text"]
            .as_str()
            .unwrap()
            .contains("plain-method-body")
    );
    assert!(typed["issues"].as_array().unwrap().iter().any(|issue| {
        issue["reason"]
            .as_str()
            .unwrap()
            .contains("type-use annotation placement")
    }));
    let default = json("get-class", &input, &["j2"], 1);
    assert!(default["issues"].as_array().unwrap().iter().any(|issue| {
        issue["reason"]
            .as_str()
            .unwrap()
            .contains("annotation default")
    }));
}

#[test]
fn malformed_value_keeps_its_independent_siblings_and_body() {
    let fixture = fixture();
    let input = fixture.path().join("damaged.jar");
    let doc = json("get-class", &input, &["j0"], 1);
    let issue = &doc["issues"][0];
    assert_eq!(issue["context"], "j0");
    assert!(
        issue["reason"]
            .as_str()
            .unwrap()
            .contains("InvalidModifiedUtf8")
    );
    let source = doc["result"]["text"].as_str().unwrap();
    assert!(source.contains("class-invisible-marker"));
    assert!(source.contains("nested-marker"));
    assert!(source.contains("calculate("));
    assert!(source.contains("return "));
    assert!(!source.contains("class-visible-marker"));

    let xref = find(&input, "string", "nested-marker");
    assert_eq!(xref["complete"], false);
    row(&xref, "j0", "annotation-string");
}

#[test]
fn omitted_enum_constructor_slots_are_aligned_only_from_original_flags() {
    let fixture = fixture();
    let input = fixture.path().join("annotations.jar");
    let qualified = find(&input, "string", "enum-parameter-marker");
    let rows = qualified["items"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    let member = rows[0]["context"].as_str().unwrap();
    row(&qualified, member, "parameter-annotation[2]-string");

    let input = fixture.path().join("unaligned.jar");
    let unresolved = find(&input, "string", "enum-parameter-marker");
    row(
        &unresolved,
        member,
        "encoded 0, descriptor slot unavailable",
    );
    assert_eq!(unresolved["complete"], false);
    assert!(
        unresolved["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| {
                issue["context"] == member
                    && issue["reason"]
                        .as_str()
                        .unwrap()
                        .contains("UnalignedAnnotationParameters")
            })
    );
}
