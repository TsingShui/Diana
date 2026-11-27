use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};

fn fixture() -> tempfile::TempDir {
    let output = tempfile::tempdir().unwrap();
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/version-diff/prepare.py");
    let result = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(recipe)
        .arg(output.path())
        .output()
        .expect("assemble comparison fixture");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    output
}

fn diff(input: &Path, old: &str, new: &str, format: &str, status: i32) -> Output {
    let result = Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg("diff")
        .arg(input.join(old))
        .arg(input.join(new))
        .args(["--format", format, "--jobs", "2"])
        .output()
        .expect("run ordinary diff command");
    assert_eq!(
        result.status.code(),
        Some(status),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    result
}

fn report(input: &Path, old: &str, new: &str, status: i32) -> Value {
    serde_json::from_slice(&diff(input, old, new, "json", status).stdout).unwrap()
}

fn method<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["matches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["old"]["signature"].as_str().is_some_and(|signature| {
                signature
                    .split_once("->")
                    .unwrap()
                    .1
                    .starts_with(&format!("{name}("))
            })
        })
        .unwrap_or_else(|| panic!("no pair for {name}: {report}"))
}

fn change<'a>(pair: &'a Value, kind: &str, offset: u32) -> &'a Value {
    pair["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == kind && row["old"]["offset"] == offset)
        .unwrap_or_else(|| panic!("no {kind} change at byte {offset}: {pair}"))
}

#[test]
fn pool_and_bootstrap_renumbering_keep_method_facts_equal() {
    let inputs = fixture();
    for (before, after, count) in [
        ("old.class", "width.class", 7),
        ("linkage-old.class", "linkage-width.class", 2),
        ("old.class", "old.class", 7),
    ] {
        let report = report(inputs.path(), before, after, 0);
        assert_eq!(report["offset_unit"], "byte");
        let pairs = report["matches"].as_array().unwrap();
        assert_eq!(pairs.len(), count);
        assert_eq!(report["issues"], json!([]));
        assert_eq!(report["unmatched"], json!([]));
        assert!(
            pairs
                .iter()
                .all(|pair| { pair["status"] == "unchanged" && pair["changes"] == json!([]) }),
            "{report}"
        );
    }
}

#[test]
fn changed_instructions_coverage_and_declarations_keep_original_positions() {
    let inputs = fixture();
    let report = report(inputs.path(), "old.jar", "changed.jar", 0);
    assert_eq!(report["issues"], json!([]));
    assert_eq!(report["unmatched"], json!([]));
    assert_eq!(method(&report, "same")["status"], "unchanged");
    let message = method(&report, "message");
    assert_eq!(message["old"]["at"], "j0:m1");
    assert_eq!(message["new"]["at"], "j2:m1");
    let value = change(message, "instruction", 0);
    assert_eq!(value["new"]["offset"], 0);
    assert!(value["old"]["text"].as_str().unwrap().contains("before"));
    assert!(value["new"]["text"].as_str().unwrap().contains("after"));

    let branch = change(method(&report, "branch"), "instruction", 1);
    assert!(branch["old"]["text"].as_str().unwrap().contains("@0x9"));
    assert!(branch["new"]["text"].as_str().unwrap().contains("@0xb"));

    let route = change(method(&report, "route"), "instruction", 1);
    assert!(route["old"]["text"].as_str().unwrap().contains("@0x28"));
    assert!(route["new"]["text"].as_str().unwrap().contains("@0x2a"));

    let handler = change(method(&report, "caught"), "exception_handlers", 1);
    for side in ["old", "new"] {
        let text = handler[side]["text"].as_str().unwrap();
        let npe = text.find("NullPointerException").unwrap();
        let throwable = text.find("Throwable").unwrap();
        assert_eq!(npe < throwable, side == "old");
    }
    let generic = method(&report, "generic");
    let changes = generic["changes"].as_array().unwrap();
    assert!(
        changes
            .iter()
            .any(|change| change["kind"] == "generic_signature")
    );
    assert!(
        changes
            .iter()
            .any(|change| change["kind"] == "declared_exceptions")
    );

    let argument = change(method(&report, "argument"), "instruction", 0);
    assert!(
        argument["old"]["text"]
            .as_str()
            .unwrap()
            .contains("old argument")
    );
    assert!(
        argument["new"]["text"]
            .as_str()
            .unwrap()
            .contains("new argument")
    );
    let sharing = change(method(&report, "sharing"), "instruction", 3);
    assert_eq!(sharing["new"]["offset"], 4);
    assert!(
        sharing["old"]["text"]
            .as_str()
            .unwrap()
            .contains("constant-ref")
    );
    assert!(
        sharing["new"]["text"]
            .as_str()
            .unwrap()
            .contains("constant-def")
    );

    let text = diff(inputs.path(), "old.jar", "changed.jar", "text", 0);
    let text = String::from_utf8(text.stdout).unwrap();
    for expected in [
        "method offset unit byte",
        "\"j2:m1\"",
        "change exception_handlers",
        "old argument",
    ] {
        assert!(text.contains(expected), "{text}");
    }
}

#[test]
fn local_damage_and_duplicate_declarations_keep_independent_results() {
    let inputs = fixture();
    let damaged = report(inputs.path(), "old.jar", "damaged.jar", 1);
    assert!(
        damaged["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["at"] == "j0")
    );
    assert!(
        damaged["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["at"] == "j1:m6")
    );
    assert_eq!(method(&damaged, "same")["status"], "unchanged");
    let partial = method(&damaged, "partial");
    assert_eq!(partial["status"], "unknown");
    assert!(
        change(partial, "instruction", 0)["new"]["text"]
            .as_str()
            .unwrap()
            .contains("iconst_5")
    );

    let duplicates = report(inputs.path(), "duplicates-old.jar", "duplicates-new.jar", 0);
    assert_eq!(duplicates["matches"], json!([]));
    let unresolved = duplicates["unmatched"].as_array().unwrap();
    assert!(
        unresolved
            .iter()
            .all(|group| group["status"] == "ambiguous")
    );
    for side in ["old", "new"] {
        let locations: std::collections::BTreeSet<_> = unresolved
            .iter()
            .flat_map(|group| group[side].as_array().unwrap())
            .map(|method| method["at"].as_str().unwrap())
            .collect();
        assert_eq!(locations.len(), 14);
        assert!(locations.contains("j0:m0"));
        assert!(locations.contains("j1:m0"));
    }
}

#[test]
fn bundle_diff_locations_can_be_used_to_read_the_original_methods() {
    let inputs = fixture();
    let report = report(inputs.path(), "bundle-old.jar", "bundle-new.jar", 0);
    let pair = method(&report, "message");
    for (side, archive, expected) in [
        ("old", "bundle-old.jar", "before"),
        ("new", "bundle-new.jar", "after"),
    ] {
        assert_eq!(pair[side]["at"], "j1/j0:m1");
        let selected = Command::new(env!("CARGO_BIN_EXE_diana"))
            .arg("get-method")
            .arg(inputs.path().join(archive))
            .arg(pair[side]["at"].as_str().unwrap())
            .output()
            .unwrap();
        assert!(
            selected.status.success(),
            "{}",
            String::from_utf8_lossy(&selected.stderr)
        );
        assert!(
            String::from_utf8(selected.stdout)
                .unwrap()
                .contains(expected)
        );
    }
}
