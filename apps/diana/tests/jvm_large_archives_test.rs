use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn run(args: &[&str], path: &Path, flags: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_diana"))
        .args(args)
        .arg(path)
        .args(flags)
        .output()
        .unwrap()
}

fn fixture(kind: &str) -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/large-archives/prepare.py");
    let output = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(script)
        .arg(kind)
        .arg(directory.path())
        .output()
        .expect("Python 3 for ZIP64 fixture assembly");
    assert_exit(&output, 0);
    directory
}

fn assert_exit(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn classes(path: &Path, exit: i32) -> Value {
    let output = run(
        &["classes"],
        path,
        &["--format", "json", "--jobs", "1", "--no-limit"],
    );
    assert_exit(&output, exit);
    serde_json::from_slice(&output.stdout).unwrap()
}

fn assert_method(path: &Path, locator: &str, value: u8) {
    let output = run(&["get-method"], path, &[locator]);
    assert_exit(&output, 0);
    let java = String::from_utf8(output.stdout).unwrap();
    assert!(java.contains(&format!("return {value};")), "{java}");
}

#[test]
fn finds_classes_after_the_classic_entry_count() {
    let directory = fixture("count");
    let jar = directory.path().join("large.jar");
    let result = classes(&jar, 0);
    assert_eq!(
        result["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| (
                item["at"].as_str().unwrap(),
                item["descriptor"].as_str().unwrap()
            ))
            .collect::<Vec<_>>(),
        [
            ("j70000", "Lfixture/archive/Large;"),
            ("j70001", "Lfixture/archive/Neighbor;"),
        ],
    );
    assert_method(&jar, "j70000:m0", 41);
}

#[test]
fn reads_wide_entry_fields_and_localizes_bad_local_metadata() {
    let directory = fixture("fields");
    let jar = directory.path().join("wide.jar");
    assert_method(&jar, "j0:m0", 41);
    assert_method(&jar, "j1:m0", 22);
    for name in ["bad-local.jar", "bad-extra.jar", "bad-legacy.jar"] {
        let malformed = directory.path().join(name);
        assert_method(&malformed, "j1:m0", 22);
        let result = classes(&malformed, 1);
        assert_eq!(result["items"].as_array().unwrap().len(), 1);
        assert_eq!(result["items"][0]["at"], "j1");
        assert_eq!(result["issues"][0]["context"], "j0");
        assert_eq!(result["issues"][0]["kind"], "Malformed");
    }
    for name in ["huge-count.jar", "bad-locator.jar"] {
        let output = run(&["classes"], &directory.path().join(name), &["--no-limit"]);
        assert_exit(&output, 1);
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn accepts_zeroed_local_zip64_sizes_before_a_data_descriptor() {
    let directory = fixture("stream");
    assert_method(&directory.path().join("stream.jar"), "j0:m0", 41);
}
