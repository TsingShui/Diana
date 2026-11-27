use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

#[test]
fn original_static_invocation_and_ordered_effects_establish_jvm_fragments() {
    let folder = tempfile::tempdir().unwrap();
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures/jvm/inline-comparison/prepare.py");
    let prepare = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(recipe)
        .arg(folder.path())
        .output()
        .unwrap();
    assert!(
        prepare.status.success(),
        "{}",
        String::from_utf8_lossy(&prepare.stderr)
    );
    for side in ["before", "after"] {
        let execution = Command::new("java")
            .arg("-cp")
            .arg(folder.path().join(side))
            .args(["Verify", side])
            .output()
            .unwrap();
        assert!(
            execution.status.success(),
            "{side}: {}",
            String::from_utf8_lossy(&execution.stderr)
        );
    }
    let binary = env!("CARGO_BIN_EXE_diana");
    let args = |format: &str| {
        Command::new(binary)
            .arg("diff")
            .arg(folder.path().join("before.jar"))
            .arg(folder.path().join("after.jar"))
            .args(["--jobs", "2", "--format", format])
            .output()
            .unwrap()
    };
    let result = args("json");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["issues"], json!([]));
    let fragments = report["fragments"].as_array().unwrap();
    assert_eq!(fragments.len(), 2, "{report}");
    for (old, new, count) in [
        ("noArguments()I", "run()I", 7),
        ("arguments(IJ)J", "forwarded(IJ)J", 7),
    ] {
        let row = fragments
            .iter()
            .find(|row| row["old"]["signature"].as_str().unwrap().ends_with(old))
            .unwrap_or_else(|| panic!("missing {old}: {fragments:?}"));
        assert!(
            row["new"]["signature"].as_str().unwrap().ends_with(new),
            "{row}"
        );
        assert_eq!(row["old_offset"], 0);
        assert_eq!(row["new_offset"], 3);
        assert_eq!(row["instruction_count"], count, "{row}");
    }
    for name in ["originalOrder(II)I", "protectedHelper()I"] {
        assert!(report["unmatched"].as_array().unwrap().iter().any(|group| {
            group["old"]
                .as_array()
                .unwrap()
                .iter()
                .any(|method| method["signature"].as_str().unwrap().ends_with(name))
        }));
    }
    let result = args("text");
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    assert_eq!(text.matches("\nfragment (").count(), 2);
    assert!(text.contains("    offset 3"));
}
