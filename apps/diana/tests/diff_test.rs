use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../quality/fixtures")
        .join(name)
}

fn versions(name: &str) -> (PathBuf, PathBuf) {
    let path = fixture("comparison").join(name);
    (
        path.join("before/classes.dex"),
        path.join("after/classes.dex"),
    )
}

fn diff(before: &Path, after: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_diana"))
        .arg("diff")
        .arg(before)
        .arg(after)
        .args(args)
        .output()
        .expect("run Diana CLI")
}

fn report(before: &Path, after: &Path) -> Value {
    let output = diff(before, after, &["--format", "json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("complete JSON report");
    assert_eq!(report["schema"], 1);
    assert!(rows(&report, "issues").is_empty(), "{report}");
    report
}

fn rows<'a>(value: &'a Value, field: &str) -> &'a [Value] {
    value[field]
        .as_array()
        .unwrap_or_else(|| panic!("missing array {field} in {value}"))
}

fn matched<'a>(report: &'a Value, old: &str, new: &str) -> &'a Value {
    let pair = rows(report, "matches")
        .iter()
        .find(|pair| pair["old"]["signature"] == old && pair["new"]["signature"] == new)
        .unwrap_or_else(|| panic!("missing pair {old} -> {new} in {report}"));
    for side in ["old", "new"] {
        assert!(
            pair[side]["at"].as_str().is_some_and(|at| !at.is_empty()),
            "{pair}"
        );
    }
    assert!(!rows(pair, "evidence").is_empty(), "{pair}");
    pair
}

#[test]
fn moved_helper_and_changed_dispatch_preserve_instruction_sites() {
    let (before, after) = versions("renamed-dispatch");
    let result = report(&before, &after);
    assert_eq!(result["summary"]["old_methods"], 6);
    assert_eq!(result["summary"]["new_methods"], 6);

    let helper = matched(&result, "Llast/w;->v(II)I", "Lnext/h;->j(II)I");
    assert_eq!(helper["status"], "unchanged");
    assert!(rows(helper, "changes").is_empty());

    let preserved = matched(&result, "Llast/c;->u(II)I", "Lnext/m;->r(II)I");
    assert_eq!(preserved["status"], "unchanged");
    assert!(rows(preserved, "changes").is_empty());

    let caller = matched(&result, "Llast/c;->s(II)I", "Lnext/m;->f(II)I");
    assert_eq!(caller["status"], "changed");
    // Offsets are measured in DEX code units from the start of each method.
    // Branch (2 units), invoke (3), move-result (1), return (1), const/16.
    for (offset, old_text, new_text) in [
        (0, "if-lt", "if-le"),
        (2, "v1 v2 Llast/w;->v(II)I", "v4 v3 Lnext/h;->j(II)I"),
        (7, "#0x11", "#0x13"),
    ] {
        for (side, expected) in [("old", old_text), ("new", new_text)] {
            assert!(
                rows(caller, "changes").iter().any(|change| {
                    change[side]["offset"] == offset
                        && change[side]["text"]
                            .as_str()
                            .is_some_and(|text| text.contains(expected))
                }),
                "missing {side} PC {offset} with {expected}: {caller}"
            );
        }
    }
}

#[test]
fn identical_unreferenced_twins_are_ambiguous_and_unrelated_forwards_stay_distinct() {
    let (before, after) = versions("renamed-dispatch");
    let result = report(&before, &after);
    for (side, names) in [
        ("old", ["Llast/c;->x(I)I", "Llast/c;->y(I)I"]),
        ("new", ["Lnext/m;->b(I)I", "Lnext/m;->d(I)I"]),
    ] {
        for name in names {
            assert!(
                rows(&result, "unmatched")
                    .iter()
                    .any(|group| group["status"] == "ambiguous"
                        && rows(group, side)
                            .iter()
                            .any(|endpoint| endpoint["signature"] == name)),
                "missing ambiguous {name}: {result}"
            );
            assert!(
                !rows(&result, "matches")
                    .iter()
                    .any(|pair| pair[side]["signature"] == name)
            );
        }
    }
    // Reverse and population count are different operations despite identical short forwarder topology.
    assert!(
        !rows(&result, "matches")
            .iter()
            .any(|pair| pair["old"]["signature"] == "Llast/c;->n(I)I"
                && pair["new"]["signature"] == "Lnext/m;->p(I)I")
    );
}

#[test]
fn inlined_helper_and_widened_handler_retain_independent_evidence() {
    let (before, after) = versions("handler-inline-challenge");
    let result = report(&before, &after);
    let entry = "Ledge/Gate;->evaluate(I[II)I";
    let helper = "Ledge/Kernel;->mix(I)I";
    let gate = matched(&result, entry, entry);
    assert_eq!(gate["status"], "changed");
    let change = rows(gate, "changes")
        .iter()
        .find(|change| change["kind"] == "exception_handlers")
        .unwrap_or_else(|| panic!("missing handler delta: {gate}"));
    assert!(
        change["old"]["text"]
            .as_str()
            .unwrap()
            .contains("ArithmeticException"),
        "{change}"
    );
    assert!(
        change["new"]["text"]
            .as_str()
            .unwrap()
            .contains("RuntimeException"),
        "{change}"
    );
    let fragment = rows(&result, "fragments")
        .iter()
        .find(|fragment| {
            fragment["old"]["signature"] == helper && fragment["new"]["signature"] == entry
        })
        .unwrap_or_else(|| panic!("missing inlined region: {result}"));
    assert_eq!(fragment["old_offset"], 0);
    assert_eq!(fragment["new_offset"], 0);
    assert!(fragment["instruction_count"].as_u64().unwrap() >= 4);
    assert!(
        !rows(&result, "matches")
            .iter()
            .any(|pair| pair["old"]["signature"] == helper)
    );
}

#[test]
fn compiled_r8_pair_recovers_the_three_independently_recorded_method_identities() {
    let result = report(
        &fixture("algorithms/algo-dp/classes.dex"),
        &fixture("obfuscated/obf-dp-r8/classes.dex"),
    );
    // The R8 mapping beside the second DEX supplies these hand-checked expectations.
    // Only the two prepared DEX inputs are passed to the application.
    for (old, new) in [
        (
            "editDistance(Ljava/lang/String;Ljava/lang/String;)I",
            "a(Ljava/lang/String;Ljava/lang/String;)I",
        ),
        ("knapsack([I[II)I", "b([I[II)I"),
        ("longestIncreasing([I)I", "c([I)I"),
    ] {
        matched(
            &result,
            &format!("Lfixture/AlgoDp;->{old}"),
            &format!("La;->{new}"),
        );
    }
}

#[test]
fn multidex_damage_publishes_both_healthy_root_matches_and_side_specific_issues() {
    let scratch = tempfile::tempdir().unwrap();
    let build = |scenario| {
        let archive = scratch.path().join(format!("{scenario}.apk"));
        let output = Command::new(if cfg!(windows) { "python" } else { "python3" })
            .arg(fixture("export/multidex-resources/prepare.py"))
            .arg(&archive)
            .arg(scenario)
            .output()
            .expect("assemble public fixture APK");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        archive
    };
    let complete = build("complete");
    let partial = build("partial");
    let path = scratch.path().join("diff.json");
    let output = diff(
        &complete,
        &partial,
        &[
            "--format",
            "json",
            "-o",
            path.to_str().unwrap(),
            "--jobs",
            "2",
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let result: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for name in [
        "Lfixture/ExprArith;->add(II)I",
        "Lfixture/BranchIfz;->bothPaths(I)I",
    ] {
        assert_eq!(matched(&result, name, name)["status"], "unchanged");
    }
    let issues = rows(&result, "issues");
    // The new side contains a truncated root and another root with an incorrect ZIP CRC.
    assert!(issues.len() >= 2, "{result}");
    assert!(
        issues.iter().all(|issue| issue["side"] == "new"
            && issue["reason"].as_str().is_some_and(|s| !s.is_empty())),
        "{issues:?}"
    );
}

#[test]
fn recycled_short_signatures_follow_bodies_instead_of_spelling() {
    let (before, after) = versions("handler-inline-challenge");
    let result = report(&before, &after);
    let a = "Ledge/Alias;->a(I)I";
    let b = "Ledge/Alias;->b(I)I";
    for (old, new) in [(a, b), (b, a)] {
        let pair = matched(&result, old, new);
        assert_eq!(pair["status"], "unchanged");
        assert!(rows(pair, "changes").is_empty());
        assert!(
            !rows(&result, "matches")
                .iter()
                .any(|pair| pair["old"]["signature"] == old && pair["new"]["signature"] == old)
        );
    }
}

#[test]
fn block_relocation_preserves_edges_and_reports_only_the_retargeted_branch() {
    let (before, after) = versions("reordered-branches");
    let result = report(&before, &after);
    let stable = "Llayout/Route;->stable(II)I";
    let unchanged = matched(&result, stable, stable);
    assert_eq!(unchanged["status"], "unchanged", "{unchanged}");
    assert!(rows(unchanged, "changes").is_empty());

    let redirect = "Llayout/Route;->redirect(II)I";
    let changed = matched(&result, redirect, redirect);
    assert_eq!(changed["status"], "changed");
    let changes = rows(changed, "changes");
    assert_eq!(changes.len(), 1, "{changed}");
    for (side, target) in [("old", "@0x6"), ("new", "@0x3")] {
        let site = &changes[0][side];
        assert_eq!(site["offset"], 0, "{changed}");
        let text = site["text"].as_str().unwrap();
        assert!(
            text.contains("if-lez") && text.contains(target),
            "{changed}"
        );
    }
}

#[path = "diff_test/selection.rs"]
mod selection;
