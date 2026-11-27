use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

const RELEASE: &str = "Lfixture/project/Release;->probe()I";

fn fixture(kind: &str) -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/project-archives/prepare.py");
    let output = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(script)
        .arg(kind)
        .arg(directory.path())
        .output()
        .expect("Python 3 for JVM project fixture assembly");
    assert_exit(&output, 0);
    directory
}

fn run(command: &str, path: &Path, flags: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg(command)
        .arg(path)
        .args(flags)
        .output()
        .unwrap()
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

fn json(command: &str, path: &Path, flags: &[&str], exit: i32) -> Value {
    let mut options = vec!["--format", "json"];
    options.extend(flags);
    let output = run(command, path, &options);
    assert_exit(&output, exit);
    serde_json::from_slice(&output.stdout).unwrap()
}

fn classes(path: &Path, exit: i32) -> Value {
    json("classes", path, &["--no-limit", "--jobs", "1"], exit)
}

fn locations(report: &Value) -> BTreeSet<&str> {
    report["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["at"].as_str().unwrap())
        .collect()
}

fn assert_method(path: &Path, selector: &str, value: u8) {
    let output = run("get-method", path, &[selector]);
    assert_exit(&output, 0);
    let java = String::from_utf8(output.stdout).unwrap();
    assert!(java.contains(&format!("return {value};")), "{java}");
}

#[test]
fn finds_physical_nested_spring_and_war_libraries() {
    let directory = fixture("nested");
    use diana::analysis::{input::InputOptions, jvm::JvmRoots, request::RequestScope};
    let apk = RequestScope::open(
        &directory.path().join("resource-part.apk"),
        &InputOptions::default(),
    )
    .unwrap();
    assert!(!JvmRoots::supports(&apk));
    let boot = directory.path().join("boot.jar");
    let report = classes(&boot, 1);
    assert_eq!(locations(&report), BTreeSet::from(["j1/j0"]));
    assert_eq!(report["issues"][0]["context"], "j3");
    assert_method(&boot, "j1/j0:m0", 37);
    let missing = run("get-method", &boot, &["j2/j0:m0"]);
    assert_exit(&missing, 1);
    let web = directory.path().join("libraries.war");
    assert_eq!(
        locations(&classes(&web, 0)),
        BTreeSet::from(["j0/j0", "j1/j0"])
    );
    assert_method(&web, "j1/j0:m0", 47);
}

#[test]
fn project_keeps_independent_artifacts_and_cross_archive_call_coordinates() {
    let directory = fixture("project");
    let path = directory.path().join("project.diana-jvm.json");
    let report = classes(&path, 1);
    assert_eq!(
        locations(&report),
        BTreeSet::from(["j0", "a1/j0/j0", "a1/j1/j0"])
    );
    let issues = report["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| {
            (
                issue["context"].as_str().unwrap(),
                issue["kind"].as_str().unwrap(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        issues,
        BTreeSet::from([
            ("a2", "Unsupported"),
            ("a4", "Unsupported"),
            ("a5", "Malformed"),
            ("a6", "Malformed"),
        ])
    );
    assert_method(&path, "a1/j0/j0:m0", 37);
    assert!(locations(&classes(&directory.path().join("resources.jar"), 0)).is_empty());

    let outline = json("outline-method", &path, &["j0:m0", "--call-depth", "1"], 1);
    let call = &outline["calls"][0];
    assert_eq!(call["caller"], "j0:m0");
    assert_eq!(call["symbol"], "Lfixture/project/Library;->probe()I");
    assert_eq!(call["target"], "a1/j0/j0:m0");
    assert_eq!(call["resolution"], "local-candidate");
}

#[test]
fn chooses_an_effective_class_without_changing_original_locations() {
    let directory = fixture("multi-release");
    for (version, at, value) in [(8, "j1", 8), (11, "j3", 9), (17, "j4", 17), (25, "j5", 21)] {
        let path = directory
            .path()
            .join(format!("java-{version}.diana-jvm.json"));
        let report = classes(&path, 0);
        assert_eq!(locations(&report), BTreeSet::from([at, "j2"]));
        assert_method(&path, RELEASE, value);
        assert_method(&path, "j1:m0", 8);
        assert_method(&path, "j5:m0", 21);
    }
    for name in ["no-selection", "duplicated"] {
        let path = directory.path().join(format!("{name}.diana-jvm.json"));
        assert_exit(&run("get-method", &path, &[RELEASE]), 4);
    }
    let normal = directory.path().join("unversioned.diana-jvm.json");
    assert_method(&normal, RELEASE, 8);
    assert_eq!(
        locations(&classes(&normal, 0)),
        BTreeSet::from(["j1", "j2"])
    );
    let folded = directory.path().join("folded.diana-jvm.json");
    assert_method(&folded, RELEASE, 17);

    let damaged = directory.path().join("bad-manifest.diana-jvm.json");
    let report = classes(&damaged, 1);
    assert_eq!(locations(&report), BTreeSet::from(["j1", "j2"]));
    assert_eq!(report["issues"][0]["context"], "j0");
    assert_eq!(report["issues"][0]["kind"], "Malformed");
}

#[cfg(unix)]
#[test]
fn refuses_a_symlink_that_resolves_outside_the_project_directory() {
    let directory = fixture("nested");
    let outside = tempfile::tempdir().unwrap();
    let external = outside.path().join("outside.jar");
    std::fs::copy(directory.path().join("libraries.war"), &external).unwrap();
    std::os::unix::fs::symlink(&external, directory.path().join("escaped.jar")).unwrap();
    let path = directory.path().join("scope.diana-jvm.json");
    std::fs::write(&path, br#"{"archives":["libraries.war","escaped.jar"]}"#).unwrap();
    let report = classes(&path, 1);
    assert_eq!(locations(&report), BTreeSet::from(["j0/j0", "j1/j0"]));
    assert_eq!(report["issues"][0]["context"], "a1");
    assert_eq!(report["issues"][0]["kind"], "Unsupported");
}
