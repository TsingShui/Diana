use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::{TempDir, tempdir};

fn fixture() -> TempDir {
    let directory = tempdir().unwrap();
    let prepare = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/archive-export/prepare.py");
    let process = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(prepare)
        .arg(directory.path())
        .output()
        .expect("assemble JVM export fixtures using Python and OpenJDK");
    assert_code(&process, 0);
    directory
}

fn cli(command: &str, input: &Path) -> Command {
    let mut cli = Command::new(env!("CARGO_BIN_EXE_diana"));
    cli.arg(command).arg(input);
    cli
}

fn assert_code(process: &Output, code: i32) {
    assert_eq!(
        process.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&process.stderr)
    );
}

fn export(input: &Path, destination: &Path, jobs: &str, code: i32) {
    let process = cli("export", input)
        .args(["--jobs", jobs, "-o"])
        .arg(destination)
        .output()
        .unwrap();
    assert_code(&process, code);
}

fn get(input: &Path, at: &str) -> Vec<u8> {
    let process = cli("get-class", input).arg(at).output().unwrap();
    assert_code(&process, 0);
    process.stdout
}

fn report(out: &Path) -> Vec<Value> {
    fs::read_to_string(out.join("export-report.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn assert_definition(input: &Path, out: &Path, path: &str, at: &str, expression: &str) {
    let bytes = fs::read(out.join(path)).unwrap();
    assert_eq!(bytes, get(input, at));
    let source = String::from_utf8(bytes).unwrap();
    assert!(source.contains(expression), "{source}");
}

#[test]
fn jvm_exports_outer_nested_and_versioned_definitions_with_original_resources() {
    let scratch = fixture();
    let input = scratch.path().join("complete.jar");
    let out = scratch.path().join("out");
    export(&input, &out, "3", 0);

    assert_definition(
        &input,
        &out,
        "sources/fixture/exported/Edition.java",
        "j1",
        "input * 2 + 1",
    );
    assert_definition(
        &input,
        &out,
        "source-definitions/j2.java",
        "j2",
        "input * 3 + 7",
    );
    assert_definition(
        &input,
        &out,
        "sources/fixture/exported/Helper.java",
        "j4/j0",
        "input * input",
    );

    assert_eq!(
        fs::read(out.join("resources/META-INF/MANIFEST.MF")).unwrap(),
        b"Manifest-Version: 1.0\r\nMulti-Release: true\r\n\r\n"
    );
    assert_eq!(
        fs::read(out.join("resources/native/linux/libfixture.so")).unwrap(),
        b"\x7fELF\x00fixture"
    );
    let mut nested: Vec<_> = (0..=255u8).cycle().take(256 * 1024).collect();
    nested.extend(b"nested");
    assert_eq!(
        fs::read(out.join("archive-resources/j4/assets/nested.bin")).unwrap(),
        nested
    );
    assert_eq!(
        fs::read(out.join("archive-resources/j4/META-INF/services/fixture.Service")).unwrap(),
        b"fixture.exported.Helper\n"
    );
    assert!(out.join("resources/dependencies/included.jar").is_file());
    assert!(
        !out.join("resources/entry-name-does-not-match.class")
            .exists()
    );
    assert!(
        !out.join("archive-resources/j4/fixture/exported/Helper.class")
            .exists()
    );
    assert!(report(&out).iter().any(|row| row["kind"] == "class-path"
        && row["at"] == "j2"
        && row["path"] == "source-definitions/j2.java"));
}

#[test]
fn jvm_export_contains_damage_and_refuses_to_replace_previous_tree() {
    let scratch = fixture();
    let input = scratch.path().join("partial.jar");
    let out = scratch.path().join("out");
    let sentinel = scratch.path().join("sentinel.bin");
    fs::write(&sentinel, b"original").unwrap();
    export(&input, &out, "2", 1);

    assert_definition(
        &input,
        &out,
        "sources/fixture/exported/Edition.java",
        "j1",
        "input * 2 + 1",
    );
    assert_definition(
        &input,
        &out,
        "sources/fixture/exported/Helper.java",
        "j4/j0",
        "input * input",
    );
    assert_eq!(
        fs::read(out.join("resource-entries/j7")).unwrap(),
        b"alternate path"
    );
    assert_eq!(fs::read(&sentinel).unwrap(), b"original");
    let records = report(&out);
    assert!(records.iter().any(|row| row["kind"] == "source"
        && row["at"] == "j6"
        && row["classification"] == "Malformed"));
    assert!(
        records.iter().any(|row| row["kind"] == "resource"
            && row["at"] == "j7"
            && row["notice"] == "Relocated")
    );
    assert!(!out.join("resources/assets/damaged.bin").exists());
    let damaged = scratch.path().join("damaged.bin");
    fs::write(&damaged, b"keep existing").unwrap();
    let result = cli("extract", &input)
        .args(["j8", "--at", "--force", "-o"])
        .arg(&damaged)
        .output()
        .unwrap();
    assert_code(&result, 1);
    assert_eq!(fs::read(damaged).unwrap(), b"keep existing");
    fs::write(out.join("user-file"), b"keep").unwrap();
    export(&input, &out, "1", 1);
    assert_eq!(fs::read(out.join("user-file")).unwrap(), b"keep");
}

#[test]
fn standalone_class_publishes_the_same_recovered_source() {
    let scratch = fixture();
    let input: PathBuf = scratch.path().join("base/fixture/exported/Edition.class");
    let out = scratch.path().join("out");
    export(&input, &out, "4", 0);
    assert_definition(
        &input,
        &out,
        "sources/fixture/exported/Edition.java",
        "j0",
        "input * 2 + 1",
    );
    assert!(!out.join("resources").exists());
}

#[test]
fn project_resources_and_entries_keep_secondary_physical_roots() {
    let scratch = fixture();
    let project = scratch.path().join("app.diana-jvm.json");
    fs::write(&project, br#"{"archives":["complete.jar","extras.jar"]}"#).unwrap();
    let resources = cli("resources", &project)
        .args([
            "--filter",
            "assets/secondary.bin",
            "--exact",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_code(&resources, 0);
    let resources: Value = serde_json::from_slice(&resources.stdout).unwrap();
    assert_eq!(resources["items"][0]["at"], "a1/j0");

    let entries = cli("entries", &project)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert_code(&entries, 0);
    let entries: Value = serde_json::from_slice(&entries.stdout).unwrap();
    assert!(
        entries["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["at"] == "a1/j0")
    );
    let extracted = scratch.path().join("extra.bin");
    let copy = cli("extract", &project)
        .args(["a1/j0", "--at", "-o"])
        .arg(&extracted)
        .output()
        .unwrap();
    assert_code(&copy, 0);
    assert_eq!(fs::read(&extracted).unwrap(), b"second artifact");

    let out = scratch.path().join("out");
    export(&project, &out, "3", 0);
    assert_eq!(
        fs::read(out.join("archive-resources/a1/assets/secondary.bin")).unwrap(),
        b"second artifact"
    );
    let artifact = scratch.path().join("extras.jar");
    let original = fs::read(&artifact).unwrap();
    let aliases = cli("get-class", &project)
        .args(["j1", "-o"])
        .arg(&artifact)
        .output()
        .unwrap();
    assert_code(&aliases, 1);
    assert_eq!(fs::read(artifact).unwrap(), original);
}

#[test]
fn jar_manifest_resources_and_path_references_keep_original_entries() {
    let scratch = fixture();
    let input = scratch.path().join("complete.jar");
    let manifest = cli("manifest", &input).output().unwrap();
    assert_code(&manifest, 0);
    assert_eq!(
        manifest.stdout,
        b"Manifest-Version: 1.0\r\nMulti-Release: true\r\n\r\n"
    );
    let json = cli("manifest", &input)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert_code(&json, 0);
    let json: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(json["at"], "j0");
    assert_eq!(json["text"], String::from_utf8(manifest.stdout).unwrap());

    let resources = cli("resources", &input)
        .args([
            "--filter",
            "assets/nested.bin",
            "--exact",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_code(&resources, 0);
    let resources: Value = serde_json::from_slice(&resources.stdout).unwrap();
    assert_eq!(resources["total"], 1);
    assert_eq!(resources["items"][0]["at"], "j4/j1");
    assert_eq!(resources["items"][0]["uncompressed_size"], "262150");

    for (target, exact) in [("assets/nested.bin", true), ("j4/j1", false)] {
        let mut command = cli("xref", &input);
        command.args(["resource", target, "--format", "json"]);
        if exact {
            command.arg("--exact");
        }
        let xref = command.output().unwrap();
        assert_code(&xref, 0);
        let xref: Value = serde_json::from_slice(&xref.stdout).unwrap();
        assert_eq!(xref["total"], 1);
        assert_eq!(xref["items"][0]["match"], "/assets/nested.bin");
        assert_eq!(xref["items"][0]["context"], "j4/j0:m2");
        assert_eq!(xref["items"][0]["use"], "resource-path depth 1");
    }
    let extracted = scratch.path().join("nested.bin");
    let copy = cli("extract", &input)
        .args(["j4/j1", "--at", "-o"])
        .arg(&extracted)
        .output()
        .unwrap();
    assert_code(&copy, 0);
    let mut expected: Vec<_> = (0..=255u8).cycle().take(256 * 1024).collect();
    expected.extend(b"nested");
    assert_eq!(fs::read(&extracted).unwrap(), expected);
}

#[test]
fn runtime_view_exports_the_chosen_variant_as_the_package_definition() {
    let scratch = fixture();
    let project = scratch.path().join("app.diana-jvm.json");
    fs::write(
        &project,
        br#"{"archives":["runtime.jar"],"java_version":17}"#,
    )
    .unwrap();
    let out = scratch.path().join("out");
    export(&project, &out, "2", 0);
    assert_definition(
        &project,
        &out,
        "sources/fixture/exported/Edition.java",
        "j2",
        "input * 3 + 7",
    );
    assert!(!out.join("source-definitions").exists());
    let unchanged = cli("diff", &project)
        .arg(&project)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert_code(&unchanged, 0);
    let unchanged: Value = serde_json::from_slice(&unchanged.stdout).unwrap();
    assert_eq!(unchanged["offset_unit"], "byte");
    assert_eq!(unchanged["matches"].as_array().unwrap().len(), 2);
    assert!(
        unchanged["matches"]
            .as_array()
            .unwrap()
            .iter()
            .all(|pair| pair["status"] == "unchanged")
    );
}

#[test]
fn jar_manifest_preserves_original_bytes_across_a_folded_value() {
    let scratch = fixture();
    let input = scratch.path().join("folded.jar");
    let raw = cli("manifest", &input).output().unwrap();
    assert_code(&raw, 0);
    assert_eq!(
        raw.stdout,
        b"Manifest-Version: 1.0\r\nX-Label: \xe2\x82\r\n \xac\r\n\r\n"
    );
    let encoded = cli("manifest", &input)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert_code(&encoded, 0);
    let encoded: Value = serde_json::from_slice(&encoded.stdout).unwrap();
    assert_eq!(encoded["text"], Value::Null);
    assert_eq!(encoded["bytes"], serde_json::json!(raw.stdout));
}

#[test]
fn equal_project_descriptors_resolve_each_inputs_own_relative_archives() {
    let scratch = fixture();
    let mut paths = Vec::new();
    for (side, source) in [("old", "base"), ("new", "variant")] {
        let directory = scratch.path().join(side);
        fs::create_dir(&directory).unwrap();
        fs::copy(
            scratch
                .path()
                .join(source)
                .join("fixture/exported/Edition.class"),
            directory.join("entry.class"),
        )
        .unwrap();
        let project = directory.join("app.diana-jvm.json");
        fs::write(&project, br#"{"archives":["entry.class"]}"#).unwrap();
        paths.push(project);
    }
    let output = cli("diff", &paths[0])
        .arg(&paths[1])
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert_code(&output, 0);
    let comparison: Value = serde_json::from_slice(&output.stdout).unwrap();
    let changes: Vec<_> = comparison["matches"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|pair| pair["status"] == "changed")
        .collect();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0]["old"]["at"], "j0:m1");
    assert_eq!(changes[0]["new"]["at"], "j0:m1");
}
