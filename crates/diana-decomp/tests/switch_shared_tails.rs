//! Independent paths: quality/fixtures/branches/switch-shared-tails/README.md.
//! Product/anchor inspection, not executed final-plan behavior.

use diana_decomp::p2::NormalEdge;
use diana_decomp::p6::RegionKind;
use diana_decomp::p7::{ControlId, EmissionLayout, MethodCode, TransferId, TransferSpelling};
use diana_decomp::p9::Origin;

#[path = "support/inspection.rs"]
mod inspection;
#[path = "support/loops.rs"]
mod loops;
#[path = "support/rethrows.rs"]
mod rethrows;
use inspection::inspect;

fn groups(code: &MethodCode) -> (ControlId, &[ControlId], &[Vec<TransferId>]) {
    code.emission()
        .controls()
        .iter()
        .enumerate()
        .find_map(|(index, c)| match &c.layout {
            EmissionLayout::Switch {
                bodies,
                cases: Some(groups),
                ..
            } => Some((
                ControlId(index.try_into().unwrap()),
                bodies.as_slice(),
                groups.as_slice(),
            )),
            _ => None,
        })
        .expect("owning P6 case interface")
}
fn key(code: &MethodCode, groups: &[Vec<TransferId>], wanted: i64) -> usize {
    groups.iter().position(|g| g.iter().any(|id| matches!(code.transfers()[id.0 as usize].role, Some(NormalEdge::SwitchCase { key, .. }) if key == wanted))).unwrap()
}

#[test]
fn the_original_nested_switch_owns_its_case_loop_but_not_the_two_shared_tails() {
    inspect(
        "composite/complex-switch-nested",
        "Lfixture/ComplexSwitchNested;",
        "run",
        "([I)I",
        |facts, code, rendered| {
            let (id, bodies, cases) = groups(code);
            let region = &facts.regions()[id.0 as usize];
            let follow = region.follow().unwrap();
            assert!(!region.nodes().contains(&follow));
            let body = &facts.regions()[bodies[key(code, cases, 1)].0 as usize];
            let inner = facts
                .loops()
                .iter()
                .min_by_key(|p| p.members().len())
                .unwrap();
            assert!(inner.members().iter().all(|n| body.nodes().contains(n)));
            assert_eq!(loops::updates_once(facts, code, rendered).len(), 2);
            assert_eq!(cases.iter().map(Vec::len).sum::<usize>(), 4);
            let bypass = &facts.regions()[bodies[key(code, cases, 2)].0 as usize];
            let target = facts
                .links()
                .iter()
                .find(|l| bypass.nodes().contains(&l.from()) && !bypass.nodes().contains(&l.to()))
                .unwrap()
                .to();
            assert_ne!(target, follow);
            assert!(!region.nodes().contains(&target));
            assert!(rendered.text().contains("break R"));
            assert!(!rendered.text().contains("default: goto"));
            assert_eq!(rendered.text().matches(".state = 2").count(), 1);
        },
    );
}

#[test]
fn a_case_local_recovering_try_and_loop_stay_whole_under_outer_finally() {
    inspect(
        "branches/switch-shared-tails",
        "Lfixture/SharedTails;",
        "route",
        "(IIIII)I",
        |facts, code, rendered| {
            let (id, bodies, cases) = groups(code);
            assert_eq!(cases.iter().map(Vec::len).sum::<usize>(), 5);
            let case = &facts.regions()[bodies[key(code, cases, 1)].0 as usize];
            let inner = facts
                .loops()
                .iter()
                .min_by_key(|p| p.members().len())
                .unwrap();
            assert!(
                inner
                    .lexical_members()
                    .iter()
                    .all(|n| case.nodes().contains(n))
            );
            let typed = facts
                .tries()
                .iter()
                .find(|p| p.finally().is_none())
                .unwrap();
            assert!(typed.members().iter().all(|n| case.nodes().contains(n)));
            let cleanup = facts
                .tries()
                .iter()
                .find(|p| p.finally().is_some())
                .unwrap();
            assert!(
                facts.regions()[id.0 as usize]
                    .nodes()
                    .iter()
                    .all(|n| cleanup.body().contains(n))
            );
            assert_eq!(loops::updates_once(facts, code, rendered).len(), 2);
            assert_eq!(rendered.text().matches(".read(").count(), 1);
            assert_eq!(rendered.text().matches(".cleanup(").count(), 1);
            assert!(
                rendered
                    .text()
                    .contains("catch (java.lang.IllegalArgumentException)")
            );
            assert!(rendered.text().contains("finally {"));
            assert!(rendered.text().contains("break R"));
            rethrows::assert_pending_rethrow(facts, rendered);
        },
    );
}

#[test]
fn direct_tail_entry_copies_execute_once_and_other_cases_bypass_them() {
    inspect(
        "branches/switch-shared-tails",
        "Lfixture/SharedTails;",
        "route",
        "(IIIII)I",
        |facts, code, rendered| {
            let (id, bodies, cases) = groups(code);
            let direct = key(code, cases, 3);
            let body = &facts.regions()[bodies[direct].0 as usize];
            assert!(body.nodes().is_empty());
            let target = body.follow().unwrap();
            assert_ne!(Some(target), facts.regions()[id.0 as usize].follow());
            let [arm] = cases[direct].as_slice() else {
                panic!("one direct selector")
            };
            let emission = &code.emission().transfers()[arm.0 as usize];
            assert!(matches!(emission.spelling, TransferSpelling::Fallthrough));
            let mut executed = 0;
            for copy in &emission.copies {
                if code.copies()[copy.0 as usize].steps.is_empty() {
                    continue;
                }
                let spans: Vec<_> = rendered
                    .spans()
                    .iter()
                    .filter(|s| matches!(s.origin, Origin::Copy { copy: id, .. } if id == *copy))
                    .collect();
                assert_eq!(spans.len(), 1);
                assert!(spans[0].start.0 < spans[0].end.0);
                assert!(spans[0].start.0 > rendered.text().find("case 3:").unwrap());
                executed += 1;
            }
            assert!(
                executed > 0,
                "actual assignment, not merely origin requirements"
            );
            let other = &facts.regions()[bodies[key(code, cases, 2)].0 as usize];
            let link = facts
                .links()
                .iter()
                .find(|l| other.nodes().contains(&l.from()) && l.to() == target)
                .unwrap();
            assert!(
                matches!(code.emission().transfers()[link.id().0 as usize].spelling, TransferSpelling::Goto(label) if code.labels()[label.0 as usize].node == target)
            );
        },
    );
}

#[test]
fn a_foreign_case_entrance_stays_explicit_without_copying_calls_or_tails() {
    inspect(
        "branches/switch-shared-tails",
        "Lfixture/SharedTails;",
        "foreign",
        "(II)I",
        |facts, code, rendered| {
            assert!(
                !code
                    .emission()
                    .controls()
                    .iter()
                    .any(|c| matches!(c.layout, EmissionLayout::Switch { cases: Some(_), .. }))
            );
            assert!(
                !facts
                    .regions()
                    .iter()
                    .any(|r| matches!(r.kind(), RegionKind::Switch { cases: Some(_), .. }))
            );
            for event in [10, 20, 30] {
                assert_eq!(
                    rendered.text().matches(&format!(".mark({event})")).count(),
                    1
                );
            }
            assert!(rendered.text().contains("case 0: goto"));
            assert!(rendered.text().contains("return -1"));
        },
    );
}

#[test]
fn abi_selector_method_exits_keep_original_marker_and_invalid_state_paths() {
    inspect(
        "kotlin/kotlin-suspend-state",
        "Lfixture/Case;",
        "fetch",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            let (id, _, cases) = groups(code);
            assert_eq!(cases.iter().map(Vec::len).sum::<usize>(), 4);
            assert!(matches!(
                facts.regions()[id.0 as usize].kind(),
                RegionKind::Switch { cases: Some(_), .. }
            ));
            assert_eq!(
                rendered
                    .text()
                    .matches("call to 'resume' before 'invoke' with coroutine")
                    .count(),
                1
            );
            assert_eq!(rendered.text().matches(".load(").count(), 2);
            assert!(rendered.text().contains("return "));
            assert!(
                rendered.text().contains("goto L"),
                "later resume bypasses the first result conversion"
            );
        },
    );
}
