use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures")
        .join(name)
}

fn archive(directory: &Path, scenario: &str) -> PathBuf {
    let path = directory.join("test.apk");
    let result = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(fixture("export/multidex-resources/prepare.py"))
        .arg(&path)
        .arg(scenario)
        .output()
        .expect("Python 3 for APK fixture assembly");
    assert_exit(&result, 0);
    path
}

fn cli(command: &str, input: &Path) -> Command {
    let mut cli = Command::new(env!("CARGO_BIN_EXE_diana"));
    cli.arg(command).arg(input);
    cli
}

fn export(input: &Path, output: &Path, jobs: &str) -> Output {
    cli("export", input)
        .arg("-o")
        .arg(output)
        .args(["--jobs", jobs])
        .output()
        .expect("run Diana")
}

fn assert_exit(result: &Output, code: i32) {
    assert_eq!(
        result.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn class_file_matches_get(input: &Path, output: &Path, class: &str, code: i32) {
    let single = cli("get-class", input)
        .arg(format!("Lfixture/{class};"))
        .output()
        .unwrap();
    assert_exit(&single, code);
    let path = output.join("sources/fixture").join(format!("{class}.java"));
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains(&format!("class {class}")));
    assert_eq!(text, String::from_utf8(single.stdout).unwrap());
}

#[test]
fn bare_dex_exports_class_and_refuses_existing_destination() {
    let scratch = tempdir().unwrap();
    let input = fixture("atoms/expr-arith/classes.dex");
    let out = scratch.path().join("out");
    assert_exit(&export(&input, &out, "1"), 0);
    class_file_matches_get(&input, &out, "ExprArith", 0);
    let sentinel = out.join("user.txt");
    fs::write(&sentinel, b"user data").unwrap();
    assert_exit(&export(&input, &out, "2"), 1);
    assert_eq!(fs::read(sentinel).unwrap(), b"user data");
}

#[test]
fn competing_exports_publish_exactly_one_complete_tree() {
    let scratch = tempdir().unwrap();
    let out = scratch.path().join("out");
    let inputs = [
        ("atoms/expr-arith/classes.dex", "ExprArith"),
        ("branches/branch-ifz/classes.dex", "BranchIfz"),
    ];
    let children = inputs.map(|(file, class)| {
        let dex = fixture(file);
        let child = cli("export", &dex)
            .arg("-o")
            .arg(&out)
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        (child, dex, class)
    });
    let mut successes = 0;
    for (child, dex, class) in children {
        let result = child.wait_with_output().unwrap();
        if result.status.success() {
            successes += 1;
            class_file_matches_get(&dex, &out, class, 0);
        } else {
            assert_exit(&result, 1);
        }
    }
    assert_eq!(successes, 1);
    assert_eq!(
        fs::read_dir(out.join("sources/fixture")).unwrap().count(),
        1
    );
    assert_eq!(fs::read_dir(scratch.path()).unwrap().count(), 1);
}

#[test]
fn multidex_includes_manifest_and_byte_exact_resources() {
    let scratch = tempdir().unwrap();
    let input = archive(scratch.path(), "complete");
    let out = scratch.path().join("out");
    assert_exit(&export(&input, &out, "3"), 0);
    class_file_matches_get(&input, &out, "ExprArith", 0);
    class_file_matches_get(&input, &out, "BranchIfz", 0);
    assert_eq!(
        fs::read_dir(out.join("sources/fixture")).unwrap().count(),
        2
    );

    let resources = out.join("resources");
    assert_eq!(
        fs::read(resources.join("lib/arm64-v8a/libsample.so")).unwrap(),
        b"\x7fELF\x02\x01\x00\xff"
    );
    assert!(!resources.join("classes.dex").exists());
    assert!(!resources.join("classes2.dex").exists());
    assert_eq!(
        fs::read(resources.join("resources.arsc")).unwrap(),
        b"opaque table\0\xff"
    );
    let mut expected = Vec::from_iter((0..=255u8).cycle().take(256 * 2048));
    expected.extend(b"end");
    assert_eq!(
        fs::read(resources.join("res/raw/payload")).unwrap(),
        expected
    );
    assert_eq!(
        fs::read(resources.join("assets/nested.dex")).unwrap(),
        fs::read(fixture("classes/direct-initialization/classes.dex")).unwrap()
    );
    let raw_manifest = scratch.path().join("manifest.axml");
    let single = cli("extract", &input)
        .args(["AndroidManifest.xml", "--raw", "-o"])
        .arg(&raw_manifest)
        .output()
        .unwrap();
    assert_exit(&single, 0);
    assert_eq!(
        fs::read(raw_manifest).unwrap(),
        fs::read(resources.join("AndroidManifest.xml")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(out.join("AndroidManifest.xml"))
            .unwrap()
            .trim(),
        "<manifest>\n</manifest>"
    );
}

#[test]
fn partial_export_retains_recovered_methods_sibling_images_and_resources() {
    let scratch = tempdir().unwrap();
    let input = archive(scratch.path(), "partial");
    let out = scratch.path().join("out");
    assert_exit(&export(&input, &out, "2"), 1);
    // The truncated root prevents name-based get-class across this APK.
    // Use the same original DEX directly to compare the publications.
    class_file_matches_get(
        &fixture("atoms/expr-arith/classes.dex"),
        &out,
        "ExprArith",
        0,
    );
    class_file_matches_get(
        &fixture("branches/branch-ifz/classes.dex"),
        &out,
        "BranchIfz",
        0,
    );
    let partial_dex = fixture("classes/direct-initialization/classes.dex");
    class_file_matches_get(&partial_dex, &out, "DirectInitialization", 1);
    class_file_matches_get(&partial_dex, &out, "Child", 0);
    class_file_matches_get(&partial_dex, &out, "Parent", 0);
    assert_eq!(
        fs::read(out.join("resources/resources.arsc")).unwrap(),
        b"opaque table\0\xff"
    );
    let report = fs::read_to_string(out.join("export-report.jsonl")).unwrap();
    assert!(report.contains("has no constructor"));
    assert!(report.contains("res/raw/unsupported"));
    assert!(report.contains("assets/damaged"));
    let source_failures: Vec<_> = report
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|row| row["kind"] == "source")
        .collect();
    assert_eq!(source_failures.len(), 2);
    assert!(
        source_failures
            .iter()
            .any(|row| row["reason"].as_str().unwrap().contains("Truncated"))
    );
    assert!(
        source_failures
            .iter()
            .any(|row| row["reason"].as_str().unwrap().contains("DamagedEntry"))
    );
    assert!(!out.join("source-definitions").exists());
    assert!(!out.join("resources/assets/damaged").exists());
    assert!(!out.join("resources/res/raw/unsupported").exists());
}

#[test]
fn duplicate_descriptors_keep_distinct_constructor_bodies_and_original_coordinates() {
    let scratch = tempdir().unwrap();
    let input = archive(scratch.path(), "duplicate");
    let out = scratch.path().join("out");
    assert_exit(&export(&input, &out, "2"), 1);
    let report = fs::read_to_string(out.join("export-report.jsonl")).unwrap();
    let rows: Vec<serde_json::Value> = report
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .filter(|row: &serde_json::Value| row["kind"] == "class-path")
        .collect();
    assert_eq!(rows.len(), 2);
    for class in ["Parent", "Child"] {
        let descriptor: Vec<_> = format!("Lfixture/{class};").encode_utf16().collect();
        let alternate = rows
            .iter()
            .find(|row| row["descriptor_units"] == serde_json::json!(descriptor))
            .expect("original alternate descriptor");
        assert_eq!(alternate["logical"], 0);
        let alternate_locator = format!(
            "e{}:l0:c{}::Lfixture/{class};",
            alternate["entry"].as_u64().unwrap(),
            alternate["class_def"].as_u64().unwrap()
        );
        let by_location = cli("get-class", &input)
            .arg(alternate_locator)
            .output()
            .unwrap();
        assert_exit(&by_location, 0);
        let alternate_path = out.join(alternate["path"].as_str().unwrap());
        assert_eq!(fs::read(&alternate_path).unwrap(), by_location.stdout);
        // Compare the exact class output for each physical definition against
        // its file regardless of which root owns the conventional package path.
        for (index, file) in [
            (6, "classes/direct-initialization/classes.dex"),
            (7, "exceptions/direct-initialization-effects/classes.dex"),
        ] {
            let path = if alternate["entry"] == index {
                out.join(alternate["path"].as_str().unwrap())
            } else {
                out.join(format!("sources/fixture/{class}.java"))
            };
            let get = cli("get-class", &fixture(file))
                .arg(format!("fixture.{class}"))
                .output()
                .unwrap();
            assert_exit(&get, 0);
            assert_eq!(fs::read(path).unwrap(), get.stdout);
        }
    }
}

#[test]
fn nested_definition_keeps_its_own_package_file() {
    let scratch = tempdir().unwrap();
    let input = archive(scratch.path(), "nested");
    let out = scratch.path().join("out");
    assert_exit(&export(&input, &out, "2"), 0);
    let package = out.join("sources/fixture");
    let outer = fs::read_to_string(package.join("ClassInnerStatic.java")).unwrap();
    let inner = fs::read_to_string(package.join("ClassInnerStatic$Holder.java")).unwrap();
    assert!(outer.contains("useHolder("));
    assert!(outer.contains(".doubled()"));
    assert!(inner.contains("int doubled()"));
    assert!(!outer.contains("int doubled()"));
}

#[test]
fn unsafe_resource_name_cannot_write_outside_export() {
    let scratch = tempdir().unwrap();
    let input = archive(scratch.path(), "unsafe-path");
    let out = scratch.path().join("out");
    let sentinel = scratch.path().join("escape");
    fs::write(&sentinel, b"original").unwrap();
    let result = export(&input, &out, "2");
    assert_exit(&result, 0);
    assert_eq!(
        fs::read(out.join("resource-entries/6")).unwrap(),
        b"outside"
    );
    assert_eq!(fs::read(sentinel).unwrap(), b"original");
    assert_eq!(
        fs::read(out.join("resources/resources.arsc")).unwrap(),
        b"opaque table\0\xff"
    );
}
