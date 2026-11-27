use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn checked(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn compile(output: &Path, files: &[&Path]) {
    std::fs::create_dir_all(output).unwrap();
    checked(
        Command::new("java")
            .args([
                "-m",
                "jdk.compiler/com.sun.tools.javac.Main",
                "-source",
                "17",
                "-target",
                "17",
                "-Xlint:-options",
                "-parameters",
                "-g",
                "-d",
            ])
            .arg(output)
            .args(files)
            .output()
            .expect("JDK 17+ to build the JVM fixture"),
    );
}

fn execute(class_path: &Path, test: &str) {
    checked(
        Command::new("java")
            .arg("-cp")
            .arg(class_path)
            .arg(test)
            .output()
            .expect("JDK 17+ to execute the JVM fixture"),
    );
}

fn source(class_file: &Path, exit: i32) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg("get-class")
        .arg(class_file)
        .args(["j0", "--format", "json"])
        .output()
        .unwrap();
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
fn retains_both_checked_noncapturing_synthetic_helpers() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../quality/fixtures/jvm/lambda-capture");
    let scratch = tempfile::tempdir().unwrap();
    let original = scratch.path().join("original");
    let java = fixture.join("SyntheticLambda.java");
    let check = fixture.join("SyntheticLambdaCheck.java");
    compile(&original, &[&java, &check]);
    execute(&original, "fixture.jvm.SyntheticLambdaCheck");

    let class_file = original.join("fixture/jvm/SyntheticLambda.class");
    let result = source(&class_file, 0);
    let java = result["result"]["text"].as_str().unwrap();
    let methods: Vec<_> = result["result"]["methods"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|method| method["name"].as_str())
        .filter(|name| name.starts_with("lambda$"))
        .collect();
    assert_eq!(methods.len(), 2, "{java}");
    for method in methods {
        assert!(
            java.contains(&format!("SyntheticLambda::{method}")),
            "{java}"
        );
        assert!(java.contains(&format!("{method}(")), "{java}");
    }
    assert!(result["issues"].as_array().unwrap().is_empty(), "{result}");

    let recovered = scratch.path().join("SyntheticLambda.java");
    std::fs::write(&recovered, java).unwrap();
    let classes = scratch.path().join("recovered");
    compile(&classes, &[&recovered, &check]);
    execute(&classes, "fixture.jvm.SyntheticLambdaCheck");
}

#[test]
fn forwards_immutable_parameters_in_their_original_capture_order() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../quality/fixtures/jvm/lambda-capture");
    let scratch = tempfile::tempdir().unwrap();
    let original = scratch.path().join("original");
    let input = fixture.join("CapturedLambda.java");
    let check = fixture.join("CapturedLambdaCheck.java");
    compile(&original, &[&input, &check]);
    execute(&original, "fixture.jvm.CapturedLambdaCheck");

    let result = source(&original.join("fixture/jvm/CapturedLambda.class"), 0);
    assert!(result["issues"].as_array().unwrap().is_empty(), "{result}");
    let java = result["result"]["text"].as_str().unwrap();
    let methods = result["result"]["methods"].as_array().unwrap();
    let helpers: Vec<_> = methods
        .iter()
        .filter_map(|method| method["name"].as_str())
        .filter(|name| name.starts_with("lambda$"))
        .collect();
    assert_eq!(helpers.len(), 2, "{java}");
    let mut compilable = java.to_owned();
    for (index, helper) in helpers.into_iter().enumerate() {
        let arguments = if helper.starts_with("lambda$captured$") {
            "digit, distance, "
        } else {
            assert!(helper.starts_with("lambda$joined$"), "{helper}");
            "entry, "
        };
        assert!(
            java.contains(&format!("-> CapturedLambda.{helper}({arguments}")),
            "{java}"
        );
        compilable = compilable.replace(helper, &format!("dianaRecoveredHelper{index}"));
    }

    // A compiler may reserve the same lambda$ name for its own recompile
    // artifact. The fixture checks execution with that helper disambiguated.
    let recovered = scratch.path().join("CapturedLambda.java");
    std::fs::write(&recovered, compilable).unwrap();
    let classes = scratch.path().join("recovered");
    compile(&classes, &[&recovered, &check]);
    execute(&classes, "fixture.jvm.CapturedLambdaCheck");
}

#[test]
fn retains_the_eager_effects_and_gaps_for_unsupported_captures() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../quality/fixtures/jvm/lambda-capture");
    let scratch = tempfile::tempdir().unwrap();
    let original = scratch.path().join("original");
    let input = fixture.join("UnsupportedCapture.java");
    let check = fixture.join("UnsupportedCaptureCheck.java");
    compile(&original, &[&input, &check]);
    execute(&original, "fixture.jvm.UnsupportedCaptureCheck");

    let result = source(&original.join("fixture/jvm/UnsupportedCapture.class"), 1);
    let java = result["result"]["text"].as_str().unwrap();
    let issues = result["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 3, "{result}");
    for issue in issues {
        assert!(
            issue["reason"]
                .as_str()
                .unwrap()
                .contains("LambdaMetafactory"),
            "{issue}"
        );
    }
    let computed = java
        .split("IntUnaryOperator computed(")
        .nth(1)
        .unwrap()
        .split("IntUnaryOperator overwritten(")
        .next()
        .unwrap();
    let (before, operands) = computed.split_once("<unavailable>").unwrap();
    assert!(
        before.contains("read(digit)") || operands.starts_with("(UnsupportedCapture.read(digit))"),
        "{computed}"
    );
    let overwritten = java
        .split("IntUnaryOperator overwritten(")
        .nth(1)
        .unwrap()
        .split("int trace(")
        .next()
        .unwrap();
    assert!(overwritten.contains("read("), "{overwritten}");
    assert!(
        overwritten.find("read(") < overwritten.find("<unavailable>"),
        "{overwritten}"
    );
    let selected = java.split("IntUnaryOperator selected(").nth(1).unwrap();
    assert!(selected.contains("<unavailable>"), "{selected}");
}

#[test]
fn casts_erased_reference_slots_at_invocation_before_entering_the_helper() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../quality/fixtures/jvm/lambda-capture");
    let scratch = tempfile::tempdir().unwrap();
    let original = scratch.path().join("original");
    let input = fixture.join("AdaptedLambda.java");
    let check = fixture.join("AdaptedLambdaCheck.java");
    compile(&original, &[&input, &check]);
    execute(&original, "fixture.jvm.AdaptedLambdaCheck");

    let result = source(&original.join("fixture/jvm/AdaptedLambda.class"), 0);
    assert!(result["issues"].as_array().unwrap().is_empty(), "{result}");
    let java = result["result"]["text"].as_str().unwrap();
    assert!(java.contains("(String)"), "{java}");
    assert!(java.contains("(String[])"), "{java}");
    let mut compilable = java.to_owned();
    let methods = result["result"]["methods"].as_array().unwrap();
    for (index, helper) in methods
        .iter()
        .filter_map(|method| method["name"].as_str())
        .filter(|name| name.starts_with("lambda$"))
        .enumerate()
    {
        compilable = compilable.replace(helper, &format!("dianaAdaptedHelper{index}"));
    }
    let recovered = scratch.path().join("AdaptedLambda.java");
    std::fs::write(&recovered, compilable).unwrap();
    let classes = scratch.path().join("recovered");
    compile(&classes, &[&recovered, &check]);
    execute(&classes, "fixture.jvm.AdaptedLambdaCheck");
}
