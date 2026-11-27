use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

fn fixture() -> tempfile::TempDir {
    let output = tempfile::tempdir().unwrap();
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/record-bootstrap/prepare.py");
    let result = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(recipe)
        .arg(output.path())
        .output()
        .expect("compile the JVM record fixture");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    output
}

fn get(command: &str, input: &Path, selected: &str, format: &str, status: i32) -> Output {
    let result = Command::new(env!("CARGO_BIN_EXE_diana"))
        .args([command])
        .arg(input)
        .args([selected, "--format", format])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(status),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    result
}

fn publication(output: Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn compiled_records_retain_the_original_operation_and_ordered_field_getters() {
    let fixture = fixture();
    let jar = fixture.path().join("records.jar");
    let report = publication(get(
        "get-class",
        &jar,
        "fixture.record.RecordProbe",
        "json",
        0,
    ));
    assert!(report["issues"].as_array().unwrap().is_empty());
    let source = report["result"]["text"].as_str().unwrap();
    for glyph in ["record-equals", "record-hash-code", "record-to-string"] {
        let body = source.split_once(glyph).unwrap().1;
        let mut last = 0;
        for component in [
            "->count:I",
            "->reading:F",
            "->label:Ljava/lang/String;",
            "->values:[I",
        ] {
            let index = body.find(component).unwrap();
            assert!(index >= last, "{body}");
            last = index;
        }
    }
    assert!(
        source
            .lines()
            .any(|line| { line.contains("return record-equals<") && line.contains("](this, ") }),
        "{source}"
    );
    assert!(source.contains("incremented("));
    let empty = publication(get(
        "get-method",
        &jar,
        "Lfixture/record/Empty;->hashCode()I",
        "json",
        0,
    ));
    assert!(
        empty["result"]["text"]
            .as_str()
            .unwrap()
            .contains("record-hash-code<Lfixture/record/Empty;>[](this)")
    );
    let plain = get("get-class", &jar, "fixture.record.RecordProbe", "text", 0);
    assert_eq!(
        String::from_utf8(plain.stdout).unwrap().trim_end(),
        source.trim_end()
    );
}

#[test]
fn inconsistent_bootstrap_retains_typed_gaps_and_other_member_code() {
    let fixture = fixture();
    for file in ["contradictory.class", "malformed.class"] {
        let input = fixture.path().join(file);
        let report = publication(get("get-class", &input, "j0", "json", 1));
        let issues = report["issues"].as_array().unwrap();
        assert_eq!(issues.len(), 3, "{report}");
        for issue in issues {
            assert!(issue["reason"].as_str().unwrap().contains("ObjectMethods"));
            assert!(issue["range_bytes"].is_array());
        }
        let source = report["result"]["text"].as_str().unwrap();
        assert!(source.contains("<unavailable>"), "{source}");
        assert!(
            source.contains("incremented(") && source.contains("+ 1"),
            "{source}"
        );
        assert!(!source.contains("record-hash-code"), "{source}");
    }
}
