use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::tempdir;

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures")
        .join(path)
}

fn diff(old: &Path, new: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_diana"));
    command.arg("diff").arg(old).arg(new);
    command
}

fn exit(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn both_presentations_deliver_all_methods_across_six_roots() {
    let scratch = tempdir().unwrap();
    let old = scratch.path().join("old.apk");
    let new = scratch.path().join("new.apk");
    let output = scratch.path().join("report.json");
    let archive = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(fixture("comparison/cli-multidex/prepare.py"))
        .arg(&old)
        .output()
        .unwrap();
    exit(&archive, 0);
    fs::copy(&old, &new).unwrap();
    fs::write(&output, b"previous report").unwrap();
    let json = diff(&old, &new)
        .args(["--jobs", "2", "--format", "json", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    exit(&json, 0);
    assert!(json.stdout.is_empty());
    let report: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    let text = diff(&old, &new).args(["--jobs", "1"]).output().unwrap();
    exit(&text, 0);
    let text = String::from_utf8(text.stdout).unwrap();
    let lines: HashSet<_> = text.lines().collect();
    assert_eq!(report["schema"], 1);
    assert_eq!(report["command"], "diff");
    for (side, path) in [("old", &old), ("new", &new)] {
        assert_eq!(report["inputs"][side], path.to_str().unwrap());
        assert_eq!(report["summary"][format!("{side}_methods")], 52);
        let mut methods = HashSet::new();
        let pairs = report["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| &pair[side]);
        let groups = report["unmatched"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|group| group[side].as_array().unwrap());
        for method in pairs.chain(groups) {
            let at = method["at"].as_str().unwrap();
            assert!(methods.insert(at), "repeated {side} method {at}");
            assert!(lines.contains(
                format!("  {side} {} at {}", method["signature"], method["at"]).as_str()
            ));
        }
        assert_eq!(methods.len(), 52);
        assert!(methods.iter().any(|locator| locator.starts_with("e5:l0:")));
    }
}

#[test]
fn output_cannot_replace_either_input() {
    let scratch = tempdir().unwrap();
    let old = scratch.path().join("old.dex");
    let new = scratch.path().join("new.dex");
    let bytes = fs::read(fixture("atoms/expr-arith/classes.dex")).unwrap();
    for path in [&old, &new] {
        fs::write(path, &bytes).unwrap();
    }
    for input in [&old, &new] {
        let result = diff(&old, &new).arg("-o").arg(input).output().unwrap();
        exit(&result, 1);
        assert!(String::from_utf8_lossy(&result.stderr).contains("aliases the input"));
        assert_eq!(fs::read(input).unwrap(), bytes);
        #[cfg(unix)]
        {
            let alias = input.with_extension("link");
            fs::hard_link(input, &alias).unwrap();
            let result = diff(&old, &new).arg("-o").arg(&alias).output().unwrap();
            exit(&result, 1);
            assert!(String::from_utf8_lossy(&result.stderr).contains("aliases the input"));
            assert_eq!(fs::read(&alias).unwrap(), bytes);
        }
    }
}

#[test]
fn failed_input_open_identifies_the_side_and_preserves_output() {
    let scratch = tempdir().unwrap();
    let missing = scratch.path().join("missing.dex");
    let valid = fixture("atoms/expr-arith/classes.dex");
    let output = scratch.path().join("report.txt");
    fs::write(&output, b"previous report").unwrap();
    for (old, new, side) in [(&missing, &valid, "old"), (&valid, &missing, "new")] {
        let result = diff(old, new).arg("-o").arg(&output).output().unwrap();
        exit(&result, 1);
        assert!(String::from_utf8_lossy(&result.stderr).contains(&format!("{side} input")));
        assert!(result.stdout.is_empty());
        assert_eq!(fs::read(&output).unwrap(), b"previous report");
    }
    let invalid_jobs = diff(&missing, &missing)
        .args(["--jobs", "0"])
        .output()
        .unwrap();
    exit(&invalid_jobs, 2);
}
