//! Multi-site inputs retain their original dispatch, captures and protection.
use diana_decomp::p6::RegionFacts;
use diana_decomp::p7::{EmissionLayout, MethodCode, TerminalKind, TransferSpelling};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

// Raw dispatch keeps gotos; natural case display needs the owning P6 entry,
// not an assumption that the next printed label is the original destination.
fn dispatch_delivery(facts: &RegionFacts, code: &MethodCode, rendered: &Rendered) {
    for terminal in code.terminals() {
        let TerminalKind::Switch { arms, .. } = &terminal.kind else {
            continue;
        };
        for &arm in arms {
            let transfer = &code.transfers()[arm.0 as usize];
            match code.emission().transfers()[arm.0 as usize].spelling {
                TransferSpelling::Goto(target) => assert_eq!(target, transfer.destination),
                TransferSpelling::Fallthrough => {
                    let body = code
                        .emission()
                        .controls()
                        .iter()
                        .find_map(|c| match &c.layout {
                            EmissionLayout::Switch {
                                bodies,
                                cases: Some(groups),
                                ..
                            } => groups
                                .iter()
                                .position(|g| g.contains(&arm))
                                .map(|i| bodies[i]),
                            _ => None,
                        })
                        .expect("selected P6 case body, not raw fallthrough");
                    let body = &facts.regions()[body.0 as usize];
                    let target = facts.links()[arm.0 as usize].to();
                    assert_eq!(body.entry().or(body.follow()), Some(target));
                }
                _ => panic!("selector lost its delivered route"),
            }
            assert_eq!(
                rendered
                    .spans()
                    .iter()
                    .filter(
                        |s| matches!(s.origin, Origin::Transfer { transfer, .. } if transfer == arm)
                    )
                    .count(),
                1
            );
        }
    }
}

#[test]
fn branching_resume_sites_keep_the_invalid_default_out_of_case_zero() {
    inspect(
        "kotlin/suspend-two-step-branch",
        "Lfixture/SuspendBranch;",
        "choose",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert!(facts.coroutine_loop().is_none());
            assert_eq!(rendered.text().matches("SuspendBranch.load(").count(), 2);
            assert_eq!(rendered.text().matches("SuspendBranch.record(").count(), 2);
            assert!(rendered.text().contains("boxInt(-1)"));
            dispatch_delivery(facts, code, rendered);
            let default = rendered.text().find("default:").unwrap();
            if code.emission().coroutine_flow().is_some() {
                // The source view retains an explicit ABI dispatch attachment,
                // so its default still reaches the original throw, not case 0.
                let target = code
                    .transfers()
                    .iter()
                    .find(|t| t.role == Some(diana_decomp::p2::NormalEdge::SwitchDefault))
                    .unwrap()
                    .destination;
                let label = &code.labels()[target.0 as usize].name;
                assert!(rendered.text()[default..].starts_with(&format!("default: goto {label};")));
                let start = rendered.text().find(&format!("{label}:")).unwrap();
                let throw = rendered.text()[start..].find("throw new ").unwrap() + start;
                assert!(start > default && throw > start);
            } else {
                let case0 = rendered.text()[default..].find("case 0:").unwrap() + default;
                assert!(rendered.text()[default..case0].contains("throw new "));
            }
        },
    );
}

#[test]
fn resumed_failures_and_marker_returns_keep_distinct_cleanup_paths() {
    inspect(
        "kotlin/suspend-finally-two-sites",
        "Lfixture/SuspendFinally;",
        "guarded",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert!(facts.coroutine_loop().is_none());
            assert!(code.emission().coroutine_loop().is_none());
            assert_eq!(rendered.text().matches("SuspendFinally.load(").count(), 2);
            assert_eq!(
                rendered.text().matches("SuspendFinally.cleanup(").count(),
                2
            );
            assert!(rendered.text().contains("exception_chain"));
            assert!(rendered.text().contains("capture_exception"));
            assert!(rendered.text().contains("throwOnFailure("));
            dispatch_delivery(facts, code, rendered);
            assert!(rendered.text().contains("default:"));
        },
    );
}

#[test]
fn effectful_default_and_shared_seed_survive_dispatch_spelling() {
    inspect(
        "kotlin/suspend-default-shared-capture",
        "Lfixture/SuspendCapturedDefault;",
        "choose",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            dispatch_delivery(facts, code, rendered);
            assert_eq!(
                rendered
                    .text()
                    .matches("SuspendCapturedDefault.load(")
                    .count(),
                2
            );
            assert_eq!(
                rendered
                    .text()
                    .matches("SuspendCapturedDefault.record(")
                    .count(),
                3
            );
            let default = rendered
                .text()
                .find("default:")
                .expect("default reaches its own effectful tail");
            let record = rendered
                .text()
                .find("SuspendCapturedDefault.record(-99)")
                .unwrap();
            let throw = rendered.text().find("throw ").unwrap();
            assert!(default < record && record < throw);
            assert!(rendered.text().contains("I$1"));
            assert!(!rendered.text().contains("suspend java.lang.Object"));
        },
    );
}
