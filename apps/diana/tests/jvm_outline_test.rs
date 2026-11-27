use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let output = tempfile::tempdir().unwrap();
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/physical-inventory/prepare.py");
    let script = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(recipe)
        .arg(output.path())
        .output()
        .expect("Python 3 for JVM fixture assembly");
    assert!(
        script.status.success(),
        "{}",
        String::from_utf8_lossy(&script.stderr)
    );
    output
}

fn outline(input: &Path, selector: &str, depth: usize, exit: i32) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_diana"))
        .args(["outline-method"])
        .arg(input)
        .args([
            selector,
            "--call-depth",
            &depth.to_string(),
            "--format",
            "json",
        ])
        .output()
        .expect("run Diana");
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn outlines_a_static_call_across_an_original_nested_archive_boundary() {
    let dir = fixture();
    let archive = dir.path().join("bundle.jar");
    let selector = "j1/j0:m0";
    let default = outline(&archive, selector, 0, 0);
    let calls = default["calls"].as_array().unwrap();
    assert_eq!(default["root"], selector);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["caller"], selector);
    assert_eq!(calls[0]["offset_byte"], 1);
    assert_eq!(calls[0]["mnemonic"], "invokestatic");
    assert_eq!(calls[0]["symbol"], "Lfixture/jvm/Outer;->increment(I)I");
    assert_eq!(calls[0]["resolution"], "depth-limit");
    assert!(calls[0]["target"].is_null());

    let expanded = outline(&archive, selector, 1, 0);
    let calls = expanded["calls"].as_array().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["target"], "j2:m0");
    assert_eq!(calls[0]["resolution"], "local");
    assert!(expanded["issues"].as_array().unwrap().is_empty());
}

#[test]
fn a_damaged_nested_declaration_preserves_an_independent_local_call_candidate() {
    let dir = fixture();
    let output = outline(&dir.path().join("damaged-bundle.jar"), "j1/j0:m0", 1, 1);
    let calls = output["calls"].as_array().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["resolution"], "local-candidate");
    assert_eq!(calls[0]["target"], "j2:m0");
    assert_eq!(output["issues"].as_array().unwrap()[0]["context"], "j0");
}

#[test]
fn constructors_follow_original_declarations_and_preserve_missing_external_symbols() {
    let dir = fixture();
    let archive = dir.path().join("inventory.jar");
    let output = outline(&archive, "j1:m0", 2, 0);
    let calls = output["calls"].as_array().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["symbol"], "Lfixture/jvm/Base;-><init>()V");
    assert_eq!(calls[0]["target"], "j2:m0");
    assert_eq!(calls[0]["resolution"], "local");
    assert_eq!(calls[1]["caller"], "j2:m0");
    assert_eq!(calls[1]["symbol"], "Ljava/lang/Object;-><init>()V");
    assert_eq!(calls[1]["resolution"], "external-or-unavailable");
    assert!(calls[1]["target"].is_null());
}
