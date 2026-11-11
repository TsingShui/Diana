use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/classes/direct-initialization/classes.dex")
}

fn get(command: &str, selector: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg(command)
        .arg(fixture())
        .arg(selector)
        .args(extra)
        .output()
        .expect("Diana CLI")
}

#[test]
fn failed_method_publishes_same_text_and_json_with_nonzero_exit() {
    let selector = "Lfixture/DirectInitialization;->broken()Lfixture/Child;";
    let text = get("get-method", selector, &[]);
    let json = get("get-method", selector, &["--format", "json"]);
    for result in [&text, &json] {
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("has no constructor"));
        assert!(!result.stdout.is_empty());
    }
    let document: serde_json::Value = serde_json::from_slice(&json.stdout).expect("complete JSON");
    assert_eq!(document["schema_version"], 5);
    assert_eq!(
        document["result"]["text"].as_str().unwrap().as_bytes(),
        text.stdout
    );
    let source = document["result"]["text"].as_str().unwrap();
    assert!(source.contains("// Diana: WARNING: Failed to decompile this method."));
    assert!(source.contains("https://github.com/TsingShui/Diana/issues"));
    assert!(!source.contains("throw new UnsupportedOperationException"));
    let issue = &document["issues"][0];
    assert_eq!(issue["class"], "Lfixture/DirectInitialization;");
    assert_eq!(issue["name"], "broken");
    assert_eq!(issue["prototype"], "()Lfixture/Child;");
    assert!(issue["method_id"].is_number());
}

#[test]
fn failed_class_retains_recovered_siblings_and_selected_method_succeeds() {
    let class = get(
        "get-class",
        "Lfixture/DirectInitialization;",
        &["--format", "json"],
    );
    assert_eq!(class.status.code(), Some(1));
    let document: serde_json::Value = serde_json::from_slice(&class.stdout).expect("class JSON");
    let text = document["result"]["text"].as_str().unwrap();
    assert!(text.contains("init-direct"), "{text}");
    assert!(text.contains("new Child("), "{text}");
    assert_eq!(document["issues"].as_array().unwrap().len(), 1);
    assert!(document["issues"][0]["method_id"].is_number());

    let method = get(
        "get-method",
        "Lfixture/DirectInitialization;->direct(I)Lfixture/Child;",
        &["--format", "json"],
    );
    assert!(
        method.status.success(),
        "{}",
        String::from_utf8_lossy(&method.stderr)
    );
    assert!(method.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&method.stdout).expect("method JSON");
    assert!(document["issues"].as_array().unwrap().is_empty());
}

#[test]
fn recovery_failure_keeps_published_output_file() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "diana-recovery-{}-{unique}.json",
        std::process::id()
    ));
    let output = get(
        "get-method",
        "Lfixture/DirectInitialization;->broken()Lfixture/Child;",
        &["--format", "json", "-o", path.to_str().unwrap()],
    );
    let bytes = std::fs::read(&path).expect("published partial recovery");
    std::fs::remove_file(&path).expect("remove test output");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("complete file JSON");
    assert_eq!(document["issues"].as_array().unwrap().len(), 1);
}
