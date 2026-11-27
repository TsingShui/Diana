use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/relations/prepare.py");
    let result = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(recipe)
        .arg(directory.path())
        .output()
        .expect("Python and OpenJDK for the fixture");
    assert_exit(&result, 0);
    directory
}

fn call(command: &str, input: &Path, flags: &[&str], exit: i32) -> Output {
    let result = Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg(command)
        .arg(input)
        .args(flags)
        .output()
        .expect("run Diana");
    assert_exit(&result, exit);
    result
}

fn assert_exit(result: &Output, exit: i32) {
    assert_eq!(
        result.status.code(),
        Some(exit),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
    );
}

fn json(command: &str, input: &Path, flags: &[&str], exit: i32) -> Value {
    let mut args = flags.to_vec();
    args.extend(["--format", "json"]);
    serde_json::from_slice(&call(command, input, &args, exit).stdout).unwrap()
}

fn contexts(value: &Value) -> BTreeSet<&str> {
    value["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["context"].as_str().unwrap())
        .collect()
}

#[test]
fn class_and_named_method_outline_preserve_original_declarations_and_calls() {
    let fixture = fixture();
    let input = fixture.path().join("relations.jar");
    let value = json(
        "outline-class",
        &input,
        &["fixture.relations.Relations", "--jobs", "16"],
        0,
    );
    assert_eq!(value["root"], "j1");
    assert_eq!(value["command"], "outline-class");
    let members = value["members"].as_array().unwrap();
    assert_eq!(
        members
            .iter()
            .map(|member| member["at"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["j1", "j1:f0", "j1:m0", "j1:m1", "j1:m2", "j1:m3", "j1:m4"],
    );
    assert_eq!(members[1]["symbol"], "CODE:I");
    assert_eq!(members[2]["symbol"], "<init>()V");
    assert_eq!(value["calls"].as_array().unwrap().len(), 5);
    assert_eq!(
        value,
        json("outline-class", &input, &["j1", "--jobs", "1"], 0)
    );

    let flags = [
        "Lfixture/relations/Relations;->virtualDispatch(Lfixture/relations/Leaf;I)I",
        "--call-depth",
        "1",
    ];
    let named = json("outline-method", &input, &flags, 0);
    let physical = json("outline-method", &input, &["j1:m2", "--call-depth", "1"], 0);
    assert_eq!(named, physical);
    let edge = &named["calls"][0];
    assert_eq!(edge["caller"], "j1:m2");
    assert_eq!(edge["offset_byte"], 2);
    assert_eq!(edge["symbol"], "Lfixture/relations/Leaf;->transform(I)I");
    assert_eq!(edge["resolution"], "declaration");
    assert_eq!(edge["target"], "j7/j0:m2");
    assert_eq!(named["calls"][1]["caller"], "j7/j0:m2");
    let text = call("outline-method", &input, &flags, 0);
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("j1:m2@2 invokevirtual Lfixture/relations/Leaf;->transform(I)I [j7/j0:m2] declaration depth 1"));
}

#[test]
fn caller_expansion_keeps_hidden_frontiers_and_finishes_cycles() {
    let fixture = fixture();
    let input = fixture.path().join("relations.jar");
    let mut args = vec![
        "method",
        "Lfixture/relations/Terminal;->seed(I)I",
        "--exact",
        "--jobs",
        "16",
        "--class",
        "fixture.relations.Terminal",
        "--transitive",
        "100",
    ];
    args.push("--no-limit");
    let all = json("xref", &input, &args, 0);
    assert_eq!(all["complete"], true);
    assert_eq!(all["total"], 5);
    let mut sites: Vec<_> = all["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["context"].as_str().unwrap(),
                row["offset_byte"].as_str().unwrap(),
                row["use"].as_str().unwrap(),
            )
        })
        .collect();
    sites.sort_unstable();
    assert_eq!(
        sites,
        [
            ("j1:m1", "1", "invokestatic depth 3"),
            ("j6:m2", "14", "invokestatic depth 2"),
            ("j6:m2", "5", "invokestatic depth 1"),
            ("j6:m3", "1", "invokestatic depth 2"),
            ("j6:m3", "7", "invokestatic depth 2"),
        ],
    );
    args.pop();
    args.extend(["--limit", "1"]);
    let limited = json("xref", &input, &args, 0);
    assert_eq!(limited["total"], 5);
    assert_eq!(limited["shown"], 1);
    assert_eq!(limited["truncated"], true);
}

#[test]
fn hierarchy_relations_keep_receivers_separate_from_the_declaring_body() {
    let fixture = fixture();
    let input = fixture.path().join("relations.jar");
    let hierarchy = json(
        "xref",
        &input,
        &[
            "impl",
            "fixture.relations.Signal",
            "--recursive",
            "--exact",
            "--no-limit",
            "--jobs",
            "16",
        ],
        0,
    );
    assert_eq!(contexts(&hierarchy), BTreeSet::from(["j3", "j5", "j7/j0"]));
    assert!(hierarchy["items"].as_array().unwrap().iter().any(|item| {
        item["context"] == "j7/j0"
            && item["offset_byte"] == ""
            && item["use"] == "implements-indirect via Lfixture/relations/Middle; → Lfixture/relations/Child; depth 3"
    }));
    let implementations = json(
        "xref",
        &input,
        &[
            "method",
            "Lfixture/relations/Signal;->send(I)I",
            "--relation",
            "implementations",
            "--exact",
            "--no-limit",
        ],
        0,
    );
    assert!(
        implementations["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item["context"] == "j2:m0"
                    && item["use"]
                        .as_str()
                        .unwrap()
                        .contains("default-candidate for Lfixture/relations/Middle;")
            })
    );
    assert!(
        implementations["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item["context"] == "j7/j0:m1"
                    && item["use"]
                        .as_str()
                        .unwrap()
                        .contains("implements-method for Lfixture/relations/Leaf;")
            })
    );
    let overrides = json(
        "xref",
        &input,
        &[
            "method",
            "Lfixture/relations/Base;->transform(I)I",
            "--relation",
            "overrides",
            "--exact",
            "--no-limit",
        ],
        0,
    );
    assert_eq!(contexts(&overrides), BTreeSet::from(["j5:m1", "j7/j0:m2"]));
    for row in overrides["items"].as_array().unwrap() {
        assert_eq!(row["match"], "Lfixture/relations/Base;->transform(I)I");
    }
}

#[test]
fn inherited_dispatch_duplicates_and_damage_do_not_invent_targets() {
    let fixture = fixture();
    let input = fixture.path().join("relations.jar");
    let inherited = json("outline-method", &input, &["j1:m4", "--expand-all"], 0);
    let edge = &inherited["calls"][0];
    assert_eq!(edge["symbol"], "Lfixture/relations/Leaf;->step(I)I");
    assert_eq!(edge["target"], "j4:m2");
    assert_eq!(edge["resolution"], "local");

    let dispatch = json("outline-method", &input, &["j1:m3", "--expand-all"], 0);
    let edge = &dispatch["calls"][0];
    assert_eq!(edge["symbol"], "Lfixture/relations/Child;->send(I)I");
    assert_eq!(edge["target"], "j2:m0");
    assert_eq!(edge["resolution"], "declaration");

    let duplicate = fixture.path().join("duplicate.jar");
    call(
        "outline-class",
        &duplicate,
        &["fixture.relations.Terminal"],
        4,
    );
    let output = json("outline-method", &duplicate, &["j1:m1", "--expand-all"], 0);
    assert_eq!(output["calls"][0]["target"], Value::Null);
    assert_eq!(output["calls"][0]["resolution"], "ambiguous");

    let damaged = fixture.path().join("damaged.jar");
    let output = json("outline-method", &damaged, &["j1:m4", "--expand-all"], 1);
    assert_eq!(output["calls"][0]["target"], "j4:m2");
    assert_eq!(output["calls"][0]["resolution"], "local-candidate");
    assert_eq!(output["issues"][0]["context"], "j8");
}

#[test]
fn interface_diamond_keeps_the_more_specific_decoded_declaration() {
    let fixture = fixture();
    let input = fixture.path().join("diamond.jar");
    let call = json("outline-method", &input, &["j0:m1", "--call-depth", "1"], 0);
    let edge = &call["calls"][0];
    assert_eq!(edge["symbol"], "Lfixture/relations/Tip;->pick()I");
    assert_eq!(edge["target"], "j3:m0");
    assert_eq!(edge["resolution"], "declaration");

    let inheritance = json(
        "xref",
        &input,
        &[
            "method",
            "Lfixture/relations/Common;->pick()I",
            "--exact",
            "--relation",
            "implementations",
            "--class",
            "fixture.relations.Winning",
            "--no-limit",
        ],
        0,
    );
    assert_eq!(contexts(&inheritance), BTreeSet::from(["j3:m0"]));
    assert!(
        inheritance["items"][0]["use"]
            .as_str()
            .unwrap()
            .contains("default-candidate for Lfixture/relations/Winning;")
    );
}
