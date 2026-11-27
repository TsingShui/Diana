use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

const MAIN: &str = "Lfixture/jvm/Main;";

fn fixture() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/physical-inventory/prepare.py");
    let result = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(recipe)
        .arg(directory.path())
        .output()
        .expect("Python 3 for JVM fixture assembly");
    assert_exit(&result, 0);
    directory
}

fn assert_exit(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}

fn query(command: &str, input: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg(command)
        .arg(input)
        .args(args)
        .output()
        .expect("run Diana")
}

fn json_query(command: &str, input: &Path, args: &[&str], exit: i32) -> Value {
    let mut flags = vec!["--format", "json", "--jobs", "16"];
    flags.extend(args);
    let output = query(command, input, &flags);
    assert_exit(&output, exit);
    serde_json::from_slice(&output.stdout).unwrap()
}

fn rows(output: &Value, column: &str) -> BTreeMap<String, String> {
    let items = output["items"].as_array().unwrap();
    let entries = items
        .iter()
        .map(|row| {
            (
                row["at"].as_str().unwrap().to_owned(),
                row[column].as_str().unwrap().to_owned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(entries.len(), items.len(), "duplicate original locator");
    entries
}

fn indexed_classes(output: &Value) -> BTreeMap<String, String> {
    rows(output, "descriptor")
}

fn expected_classes(
    entries: impl IntoIterator<Item = (&'static str, &'static str)>,
) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .map(|(at, descriptor)| (at.to_owned(), descriptor.to_owned()))
        .collect()
}

fn expected_main_members(
    entries: &[usize],
    kind: &str,
    signatures: &[&str],
) -> BTreeMap<String, String> {
    entries
        .iter()
        .flat_map(|entry| {
            signatures.iter().enumerate().map(move |(ordinal, member)| {
                (
                    format!("j{entry}:{kind}{ordinal}"),
                    format!("{MAIN}->{member}"),
                )
            })
        })
        .collect()
}

#[test]
fn classes_follow_original_names_and_keep_multi_release_definitions() {
    let scratch = fixture();
    let input = scratch.path().join("inventory.jar");
    let all = json_query("classes", &input, &["--no-limit"], 0);
    let expected = expected_classes([
        ("j1", MAIN),
        ("j2", "Lfixture/jvm/Base;"),
        ("j3", "Lfixture/jvm/Marker;"),
        ("j4", "Lfixture/jvm/Helper;"),
        ("j5", MAIN),
    ]);
    assert_eq!(indexed_classes(&all), expected);
    let named = json_query(
        "classes",
        &input,
        &["--filter", MAIN, "--exact", "--no-limit"],
        0,
    );
    assert_eq!(
        indexed_classes(&named),
        expected_classes([("j1", MAIN), ("j5", MAIN)])
    );
    let derived = json_query(
        "classes",
        &input,
        &[
            "--extends",
            "fixture.jvm.Base",
            "--implements",
            "Lfixture/jvm/Marker;",
            "--kind",
            "class",
            "--no-limit",
        ],
        0,
    );
    assert_eq!(indexed_classes(&derived), indexed_classes(&named));
    let interfaces = json_query("classes", &input, &["--kind", "interface", "--no-limit"], 0);
    assert_eq!(
        indexed_classes(&interfaces),
        expected_classes([("j3", "Lfixture/jvm/Marker;")])
    );

    let tree = json_query("tree", &input, &["--classes", "--prefix", "fixture.jvm"], 0);
    let tree_members = tree["tree"]["packages"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|package| package["classes"].as_array().unwrap())
        .map(|class| {
            let at = class["at"].as_str().unwrap();
            at.split("::").next().unwrap()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(tree_members, expected.keys().map(String::as_str).collect());
}

#[test]
fn declared_members_and_strings_keep_physical_positions() {
    let scratch = fixture();
    let input = scratch.path().join("inventory.jar");

    let methods = [
        "<init>()V",
        "forward(I)I",
        "forward(J)J",
        "message()Ljava/lang/String;",
        "read()I",
    ];
    let expected = expected_main_members(&[1, 5], "m", &methods);
    let result = json_query(
        "methods",
        &input,
        &["--class", "fixture.jvm.Main", "--no-limit"],
        0,
    );
    assert_eq!(rows(&result, "member"), expected);
    let text = query(
        "methods",
        &input,
        &["--jobs", "16", "--class", MAIN, "--no-limit"],
    );
    assert_exit(&text, 0);
    let text = String::from_utf8(text.stdout).unwrap();
    let published = text
        .lines()
        .skip(1)
        .map(|line| {
            let (at, member) = line.split_once('\t').unwrap();
            (at.to_owned(), member.to_owned())
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(published, expected);

    let matches = json_query(
        "methods",
        &input,
        &["--exact", "--filter", "forward", "--no-limit"],
        0,
    );
    assert_eq!(
        rows(&matches, "member"),
        expected
            .into_iter()
            .filter(|(_, member)| member.contains("->forward("))
            .collect()
    );
    let abstract_member = json_query(
        "methods",
        &input,
        &["--class", "fixture.jvm.Marker", "--no-limit"],
        0,
    );
    assert_eq!(
        rows(&abstract_member, "member"),
        BTreeMap::from([(
            "j3:m0".to_owned(),
            "Lfixture/jvm/Marker;->read()I".to_owned()
        )])
    );

    let fields = [
        "MESSAGE:Ljava/lang/String;",
        "FIXTURE_LONG:J",
        "NEGATIVE_ZERO:D",
        "value:I",
    ];
    let result = json_query("fields", &input, &["--class", MAIN, "--no-limit"], 0);
    assert_eq!(
        rows(&result, "member"),
        expected_main_members(&[1, 5], "f", &fields)
    );

    let strings = json_query("strings", &input, &["--no-limit"], 0);
    assert_eq!(
        rows(&strings, "value"),
        BTreeMap::from([
            ("j1:s8".to_owned(), r"root\0🌱".to_owned()),
            ("j5:s8".to_owned(), r"overlay\0🌱".to_owned()),
        ])
    );
}

#[test]
fn bare_class_and_local_archive_failures_preserve_valid_declarations() {
    let scratch = fixture();
    let bare = scratch.path().join("Main.class");
    let result = json_query("methods", &bare, &["--no-limit"], 0);
    assert_eq!(
        rows(&result, "member"),
        expected_main_members(
            &[0],
            "m",
            &[
                "<init>()V",
                "forward(I)I",
                "forward(J)J",
                "message()Ljava/lang/String;",
                "read()I"
            ],
        )
    );
    let damaged = scratch.path().join("damaged.jar");
    let result = json_query("classes", &damaged, &["--no-limit"], 1);
    assert_eq!(result["complete"], false);
    assert_eq!(
        indexed_classes(&result),
        expected_classes([
            ("j1", MAIN),
            ("j2", "Lfixture/jvm/Base;"),
            ("j3", "Lfixture/jvm/Marker;"),
            ("j4", "Lfixture/jvm/Helper;"),
            ("j5", MAIN),
            ("j9", "Lfixture/jvm/After;"),
        ])
    );
    let issues = result["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 2);
    for location in ["j7", "j8"] {
        assert!(
            issues.iter().any(|issue| {
                issue["context"] == location
                    && issue["kind"] == "Malformed"
                    && issue["reason"]
                        .as_str()
                        .is_some_and(|reason| !reason.is_empty())
            }),
            "{location} missing from {issues:?}"
        );
    }
}

#[test]
fn declared_bundle_class_has_an_independent_locator() {
    let scratch = fixture();
    let input = scratch.path().join("bundle.jar");
    let result = json_query("classes", &input, &["--no-limit"], 0);
    assert_eq!(
        indexed_classes(&result),
        expected_classes([
            ("j1/j0", "Lfixture/jvm/Nested;"),
            ("j2", "Lfixture/jvm/Outer;")
        ])
    );
    let undeclared = query("bytecode", &input, &["j3/j0:m0", "--format", "json"]);
    assert!(undeclared.stdout.is_empty());
    assert!(!undeclared.status.success());
    assert!(String::from_utf8_lossy(&undeclared.stderr).contains("j3"));
}

#[test]
fn invalid_member_identity_keeps_other_method_ordinals() {
    let scratch = fixture();
    let input = scratch.path().join("DamagedMember.class");
    let result = json_query("methods", &input, &["--no-limit"], 1);
    assert_eq!(result["complete"], false);
    let mut expected = expected_main_members(
        &[0],
        "m",
        &[
            "<init>()V",
            "forward(I)I",
            "forward(J)J",
            "message()Ljava/lang/String;",
            "read()I",
        ],
    );
    expected.remove("j0:m1");
    assert_eq!(rows(&result, "member"), expected);
    let issues = result["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0]["context"], "j0:m1");
    assert_eq!(issues[0]["kind"], "Malformed");
}

#[test]
fn malformed_nested_catalog_row_keeps_the_declared_sibling() {
    let scratch = fixture();
    let input = scratch.path().join("damaged-bundle.jar");
    let result = json_query("classes", &input, &["--no-limit"], 1);
    assert_eq!(result["complete"], false);
    assert_eq!(
        indexed_classes(&result),
        expected_classes([
            ("j1/j0", "Lfixture/jvm/Nested;"),
            ("j2", "Lfixture/jvm/Outer;")
        ])
    );
    let issues = result["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0]["context"], "j0");
    assert_eq!(issues[0]["kind"], "Malformed");
}

fn reference_sites(result: &Value) -> BTreeSet<(&str, &str, &str)> {
    result["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| {
            (
                site["context"].as_str().unwrap(),
                site["match"].as_str().unwrap(),
                site["offset_byte"].as_str().unwrap(),
            )
        })
        .collect()
}

#[test]
fn bytecode_and_xref_share_original_method_and_instruction_positions() {
    let scratch = fixture();
    let input = scratch.path().join("inventory.jar");
    let target = "Lfixture/jvm/Helper;->twice(I)I";
    let call = json_query(
        "xref",
        &input,
        &["method", target, "--exact", "--no-limit"],
        0,
    );
    assert_eq!(call["complete"], true);
    assert_eq!(call["total"], 2);
    assert_eq!(
        reference_sites(&call),
        BTreeSet::from([("j1:m1", target, "5"), ("j5:m1", target, "5")])
    );
    let unused = json_query(
        "xref",
        &input,
        &[
            "method",
            "Lfixture/jvm/Helper;->unused()V",
            "--exact",
            "--no-limit",
        ],
        0,
    );
    assert_eq!(unused["total"], 0);
    let target = "Lfixture/jvm/Main;->value:I";
    let field = json_query(
        "xref",
        &input,
        &["field", target, "--exact", "--no-limit"],
        0,
    );
    assert_eq!(
        reference_sites(&field),
        BTreeSet::from([
            ("j1:m0", target, "6"),
            ("j1:m4", target, "1"),
            ("j5:m0", target, "6"),
            ("j5:m4", target, "1"),
        ])
    );

    let result = query("bytecode", &input, &["j1:m1", "--format", "json"]);
    assert_exit(&result, 0);
    let listing: Value = serde_json::from_slice(&result.stdout).unwrap();
    let instructions = listing["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["offset_byte"] != "")
        .map(|entry| {
            (
                entry["offset_byte"].as_str().unwrap(),
                entry["mnemonic"].as_str().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        instructions,
        [
            ("0", "iload_0"),
            ("1", "ifeq"),
            ("4", "iload_0"),
            ("5", "invokestatic"),
            ("8", "ireturn"),
            ("9", "iconst_0"),
            ("10", "ireturn"),
        ]
    );
    let branch = listing["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["mnemonic"] == "ifeq")
        .unwrap();
    assert_eq!(
        branch["operands"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .next(),
        Some("9")
    );
}

#[test]
fn unknown_opcode_keeps_the_prefix_and_its_resolved_reference() {
    let scratch = fixture();
    let input = scratch.path().join("DamagedCode.class");
    let target = "Lfixture/jvm/Helper;->twice(I)I";
    let refs = json_query(
        "xref",
        &input,
        &["method", target, "--exact", "--no-limit"],
        0,
    );
    assert_eq!(refs["complete"], false);
    assert_eq!(
        reference_sites(&refs),
        BTreeSet::from([("j0:m1", target, "5")])
    );

    let result = query("bytecode", &input, &["j0:m1", "--format", "json"]);
    assert_exit(&result, 0);
    let listing: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(listing["complete"], false);
    assert!(
        listing["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["mnemonic"] == "ireturn" && entry["offset_byte"] == "10" })
    );
    for result in [refs, listing] {
        let issues = result["issues"].as_array().unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0]["context"], "j0:m1");
        assert_eq!(issues[0]["kind"], "Unsupported");
        assert_eq!(issues[0]["range_bytes"], serde_json::json!([11, 12]));
    }
}

#[test]
fn a_nested_caller_keeps_its_physical_path_in_reference_results() {
    let scratch = fixture();
    let input = scratch.path().join("bundle.jar");
    let target = "Lfixture/jvm/Outer;->increment(I)I";
    let result = json_query(
        "xref",
        &input,
        &["method", target, "--exact", "--no-limit"],
        0,
    );
    assert_eq!(
        reference_sites(&result),
        BTreeSet::from([("j1/j0:m0", target, "1")])
    );

    let listing = query("bytecode", &input, &["j1/j0:m0", "--format", "json"]);
    assert_exit(&listing, 0);
    let listing: Value = serde_json::from_slice(&listing.stdout).unwrap();
    let call = listing["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["mnemonic"] == "invokestatic")
        .unwrap();
    assert_eq!(call["context"], "j1/j0:m0");
    assert_eq!(call["offset_byte"], "1");
    assert!(call["operands"].as_str().unwrap().contains(target));
}

#[test]
fn dynamic_reference_sites_follow_original_bootstrap_arguments() {
    let scratch = fixture();
    let input = scratch.path().join("Dynamic.class");
    let target = "Lfixture/jvm/Bootstrap;->seed(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/String;";
    let output = json_query(
        "xref",
        &input,
        &["method", target, "--exact", "--no-limit"],
        0,
    );
    assert_eq!(output["complete"], true);
    assert_eq!(
        reference_sites(&output),
        BTreeSet::from([("j0:m0", target, "0"), ("j0:m1", target, "0")])
    );
    assert_eq!(output["items"].as_array().unwrap().len(), 2);

    let output = json_query(
        "xref",
        &input,
        &["string", "nested-bootstrap-note", "--exact", "--no-limit"],
        0,
    );
    // load's own row supplies the string directly and again through the
    // shared nested row. invoke reaches only the nested row.
    let sites = output["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["context"].as_str().unwrap(),
                row["offset_byte"].as_str().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(sites, [("j0:m0", "0"), ("j0:m0", "0"), ("j0:m1", "0")]);

    let output = json_query(
        "xref",
        &scratch.path().join("DamagedDynamic.class"),
        &["method", target, "--exact", "--no-limit"],
        0,
    );
    assert_eq!(output["complete"], false);
    assert_eq!(output["items"].as_array().unwrap().len(), 2);
    let issues = output["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0]["context"], "j0:m0");
    assert_eq!(issues[0]["kind"], "Malformed");
    assert_eq!(issues[0]["range_bytes"], serde_json::json!([0, 3]));
    assert!(
        issues[0]["reason"]
            .as_str()
            .unwrap()
            .contains("bootstrap method handle")
    );
}
