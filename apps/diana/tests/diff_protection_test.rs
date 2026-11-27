use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn compare(slug: &str) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/comparison")
        .join(slug);
    let out = Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg("diff")
        .arg(root.join("before/classes.dex"))
        .arg(root.join("after/classes.dex"))
        .args(["--format", "json"])
        .output()
        .expect("run Diana");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: Value = serde_json::from_slice(&out.stdout).expect("JSON diff");
    assert_eq!(result["issues"], serde_json::json!([]));
    assert_eq!(result["unmatched"], serde_json::json!([]));
    result
}

fn pair<'a>(result: &'a Value, name: &str) -> &'a Value {
    result["matches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["old"]["signature"].as_str().unwrap().contains(name))
        .expect("expected matching method")
}

fn row(pair: &Value, before: u32, after: u32) -> &Value {
    assert_eq!(pair["status"], "changed");
    pair["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["kind"] == "exception_handlers"
                && row["old"]["offset"] == before
                && row["new"]["offset"] == after
        })
        .unwrap_or_else(|| panic!("missing protected site {before} -> {after}: {pair}"))
}

#[test]
fn throw_relocated_out_of_an_unchanged_physical_try_interval_is_reported() {
    let result = compare("protected-terminal-permutation");
    let delta = row(pair(&result, "->run("), 6, 9);
    assert!(
        delta["old"]["text"]
            .as_str()
            .unwrap()
            .contains("Ljava/lang/RuntimeException;")
    );
    assert_eq!(delta["new"]["text"], "unprotected");
}

#[test]
fn switch_arms_exchange_effective_handlers_while_an_intact_region_can_move() {
    let result = compare("switch-protection-permutation");
    let dispatch = pair(&result, "->dispatch(");
    let lost = row(dispatch, 8, 12);
    let gained = row(dispatch, 12, 8);
    assert_eq!(lost["new"]["text"], "unprotected");
    assert_eq!(gained["old"]["text"], "unprotected");
    for text in [&lost["old"]["text"], &gained["new"]["text"]] {
        let text = text.as_str().unwrap();
        assert!(
            text.find("Ljava/lang/IllegalArgumentException;").unwrap()
                < text.find("catch-all").unwrap()
        );
    }
    let same = pair(&result, "->preserved(");
    assert_eq!(same["status"], "unchanged");
    assert_eq!(same["changes"], serde_json::json!([]));
}
