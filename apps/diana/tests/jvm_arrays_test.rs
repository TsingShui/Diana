use std::path::Path;
use std::process::Command;

#[test]
fn multidimensional_arrays_preserve_rank_evaluation_order_and_exception_scope() {
    let scratch = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/multidimensional-array");
    let compiler = Command::new("java")
        .args([
            "-m",
            "jdk.compiler/com.sun.tools.javac.Main",
            "-source",
            "17",
            "-target",
            "17",
            "-Xlint:-options",
            "-g",
            "-parameters",
            "-d",
        ])
        .arg(scratch.path())
        .arg(fixture.join("Tensor.java"))
        .output()
        .unwrap();
    assert!(
        compiler.status.success(),
        "{}",
        String::from_utf8_lossy(&compiler.stderr)
    );
    let output = Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg("get-class")
        .arg(scratch.path().join("fixture/arrays/Tensor.class"))
        .arg("j0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let source = String::from_utf8(output.stdout).unwrap();
    assert!(
        source.contains("new String[this.dimension(1, rows)][this.dimension(2, columns)][]"),
        "{source}"
    );
    for expected in [
        "this.dimension(1, rows)",
        "this.dimension(2, columns)",
        "this.dimension(3, depth)",
        "catch (NegativeArraySizeException",
    ] {
        assert!(source.contains(expected), "{source}");
    }
    let rendered = scratch.path().join("Tensor.java");
    std::fs::write(&rendered, source).unwrap();
    let compiler = Command::new("java")
        .args(["-m", "jdk.compiler/com.sun.tools.javac.Main", "-d"])
        .arg(scratch.path())
        .arg(rendered)
        .arg(fixture.join("CheckTensor.java"))
        .output()
        .unwrap();
    assert!(
        compiler.status.success(),
        "{}",
        String::from_utf8_lossy(&compiler.stderr)
    );
    let checked = Command::new("java")
        .arg("-cp")
        .arg(scratch.path())
        .arg("fixture.arrays.CheckTensor")
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}
