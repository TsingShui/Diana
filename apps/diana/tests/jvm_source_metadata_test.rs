use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn compile(output: &Path, method_parameters: bool) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/source-metadata/Metadata.java");
    let mut javac = Command::new("java");
    javac.args([
        "-m",
        "jdk.compiler/com.sun.tools.javac.Main",
        "-source",
        "17",
        "-target",
        "17",
        "-Xlint:-options",
        "-g",
    ]);
    if method_parameters {
        javac.arg("-parameters");
    }
    let result = javac
        .arg("-d")
        .arg(output)
        .arg(source)
        .output()
        .expect("compile the fixture with JDK 17 or newer");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    output.join("fixture/jvm/Metadata.class")
}

fn get(file: &Path, format: &str, exit: i32) -> Output {
    let result = Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg("get-class")
        .arg(file)
        .args(["j0", "--format", format])
        .output()
        .expect("run the ordinary class source route");
    assert_eq!(
        result.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    result
}

fn json(file: &Path, exit: i32) -> Value {
    serde_json::from_slice(&get(file, "json", exit).stdout).unwrap()
}

#[test]
fn signatures_and_original_parameter_names_reach_the_same_java_source() {
    for with_parameters in [true, false] {
        let fixture = TempDir::new().unwrap();
        let path = compile(fixture.path(), with_parameters);
        let document = json(&path, 0);
        assert_eq!(document["issues"], serde_json::json!([]));
        let text = document["result"]["text"].as_str().unwrap();
        for expected in [
            "class Metadata<T extends Number>",
            "List<T> history;",
            "<U extends T> U pick(long window, U candidate)",
            "return candidate;",
            "double delta(long ticks, double amount)",
        ] {
            assert!(text.contains(expected), "{text}");
        }
        if with_parameters {
            assert!(text.contains("T declared(T source);"), "{text}");
        }
        assert_eq!(get(&path, "text", 0).stdout, text.as_bytes());
    }
}

fn replace(original: &[u8], before: &[u8], after: &[u8]) -> Vec<u8> {
    assert_eq!(before.len(), after.len());
    let mut matches = original
        .windows(before.len())
        .enumerate()
        .filter(|(_, bytes)| *bytes == before)
        .map(|(offset, _)| offset);
    let offset = matches.next().expect("original signature constant");
    assert!(matches.next().is_none(), "the input has one such signature");
    let mut corrupt = original.to_vec();
    corrupt[offset..offset + after.len()].copy_from_slice(after);
    corrupt
}

#[test]
fn invalid_optional_signatures_leave_erased_declarations_and_original_bodies() {
    let fixture = TempDir::new().unwrap();
    let path = compile(fixture.path(), true);
    let original = std::fs::read(path).unwrap();

    let bad_class = fixture.path().join("bad-class.class");
    std::fs::write(
        &bad_class,
        replace(
            &original,
            b"<T:Ljava/lang/Number;>Ljava/lang/Object;",
            b"!T:Ljava/lang/Number;>Ljava/lang/Object;",
        ),
    )
    .unwrap();
    let report = json(&bad_class, 1);
    let text = report["result"]["text"].as_str().unwrap();
    assert!(text.contains("class Metadata {"), "{text}");
    assert!(text.contains("List history;"), "{text}");
    assert!(text.contains("return candidate;"), "{text}");
    assert!(text.contains("// Diana: JVM declaration at j0 (Malformed)"));
    assert_eq!(get(&bad_class, "text", 1).stdout, text.as_bytes());

    let bad_method = fixture.path().join("bad-method.class");
    std::fs::write(
        &bad_method,
        replace(&original, b"<U:TT;>(JTU;)TU;", b"<U:TT;>(ITU;)TU;"),
    )
    .unwrap();
    let report = json(&bad_method, 1);
    let text = report["result"]["text"].as_str().unwrap();
    assert!(
        text.contains("Number pick(long window, Number candidate)"),
        "{text}"
    );
    assert!(text.contains("return candidate;"), "{text}");
    assert_eq!(report["issues"][0]["context"], "j0:m2");
    assert_eq!(report["issues"][0]["kind"], "Malformed");
    assert_eq!(get(&bad_method, "text", 1).stdout, text.as_bytes());
}
