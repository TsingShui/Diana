use super::{matched, report, rows, versions};
use serde_json::Value;

fn compare(name: &str) -> Value {
    let (before, after) = versions(name);
    report(&before, &after)
}

fn unchanged(result: &Value, old: &str, new: &str) {
    let pair = matched(result, old, new);
    assert_eq!(pair["status"], "unchanged", "{pair}");
    assert!(rows(pair, "changes").is_empty(), "{pair}");
}

fn literal_change(pair: &Value, offset: u32, old: &str, new: &str) {
    assert_eq!(pair["status"], "changed", "{pair}");
    assert!(
        rows(pair, "changes").iter().any(|change| {
            [("old", old), ("new", new)]
                .into_iter()
                .all(|(side, literal)| {
                    change[side]["offset"] == offset
                        && change[side]["text"]
                            .as_str()
                            .is_some_and(|text| text.contains(literal))
                })
        }),
        "{pair}"
    );
}

#[test]
fn switched_constants_do_not_cross_two_independently_anchored_owners() {
    let result = compare("swapped-helper-constants");
    unchanged(&result, "Lx/a;->route(I)I", "Ly/s;->route(I)I");
    unchanged(&result, "Lx/b;->route(I)I", "Ly/t;->route(I)I");
    let first = matched(&result, "Lx/a;->a(I)I", "Ly/s;->b(I)I");
    literal_change(first, 0, "#0x7", "#0xf");
    let second = matched(&result, "Lx/b;->a(I)I", "Ly/t;->b(I)I");
    literal_change(second, 0, "#0xf", "#0x7");
}

#[test]
fn two_callers_retain_a_helper_moved_out_of_an_otherwise_paired_owner() {
    let result = compare("cross-owner-helper-move");
    unchanged(&result, "Lxx/e;->marker(I)I", "Lyy/g;->marker(I)I");
    unchanged(&result, "Lxx/f;->marker(I)I", "Lyy/h;->marker(I)I");
    unchanged(&result, "Lxx/e;->read(I)I", "Lyy/g;->read(I)I");
    unchanged(&result, "Lxx/e;->write(I)I", "Lyy/g;->write(I)I");
    let moved = matched(&result, "Lxx/e;->m(I)I", "Lyy/h;->n(I)I");
    literal_change(moved, 0, "#0x11", "#0x13");
}

#[test]
fn pruned_argument_and_changed_literal_remain_visible_after_matching() {
    let result = compare("pruned-argument");
    let helper = matched(&result, "Lx/u;->k(II)I", "Ly/v;->r(I)I");
    literal_change(helper, 4, "#0x1f", "#0x25");
    assert!(
        rows(helper, "changes")
            .iter()
            .any(|change| change["kind"] == "prototype"
                && change["old"]["text"] == "(II)I"
                && change["new"]["text"] == "(I)I"),
        "{helper}"
    );
    for method in ["first", "second"] {
        let name = format!("Lapp/Api;->{method}(I)I");
        let caller = matched(&result, &name, &name);
        assert_eq!(caller["status"], "changed");
        assert!(
            rows(caller, "changes")
                .iter()
                .any(|change| change["old"]["offset"] == 1
                    && change["new"]["offset"] == 1
                    && change["old"]["text"]
                        .as_str()
                        .is_some_and(|s| s.contains("v1 v0 Lx/u;->k(II)I"))
                    && change["new"]["text"]
                        .as_str()
                        .is_some_and(|s| s.contains("v1 Ly/v;->r(I)I"))),
            "{caller}"
        );
    }
}

#[test]
fn compiler_inline_preserves_outer_identity_and_locates_absorbed_helper() {
    let result = compare("optimized-single-inline");
    let helper = "La/a;->a(II)I";
    let old_entry = "La/a;->c(II)I";
    let new_entry = "La/a;->a(II)I";
    let marker = "La/a;->b()Ljava/lang/String;";
    unchanged(&result, marker, marker);
    let outer = matched(&result, old_entry, new_entry);
    assert_eq!(outer["status"], "changed");
    assert!(
        rows(outer, "changes")
            .iter()
            .any(|change| change["old"]["offset"] == 5
                && change["old"]["text"]
                    .as_str()
                    .is_some_and(|s| s.contains(helper))),
        "{outer}"
    );
    for change in rows(outer, "changes") {
        for side in ["old", "new"] {
            if let Some(text) = change[side]["text"].as_str() {
                assert!(
                    !text.contains("->rotateLeft(") && !text.contains("->rotateRight("),
                    "{change}"
                );
            }
        }
    }
    assert!(
        !rows(&result, "matches")
            .iter()
            .any(|p| p["old"]["signature"] == helper)
    );
    let fragments = rows(&result, "fragments");
    assert_eq!(fragments.len(), 1, "{result}");
    let fragment = &fragments[0];
    assert_eq!(fragment["old"]["signature"], helper);
    assert_eq!(fragment["new"]["signature"], new_entry);
    // The six original DEX operations occupy 3+1+1+3+1+2 code units:
    // [0, 11) of the old helper occurs at [5, 16) of the new entry.
    assert_eq!(fragment["old_offset"], 0);
    assert_eq!(fragment["new_offset"], 5);
    assert_eq!(fragment["instruction_count"], 6);
}
