//! Multi-site inputs retain their original dispatch, captures and protection.
use diana_decomp::p2::NormalEdge;
use diana_decomp::p7::{TerminalKind, TransferSpelling};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

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
            for terminal in code.terminals() {
                if let TerminalKind::Switch { arms, .. } = &terminal.kind {
                    for &arm in arms {
                        let TransferSpelling::Goto(target) =
                            code.emission().transfers()[arm.0 as usize].spelling
                        else {
                            panic!("dispatch rows must state their target");
                        };
                        assert_eq!(target, code.transfers()[arm.0 as usize].destination);
                        if code.transfers()[arm.0 as usize].role == Some(NormalEdge::SwitchDefault)
                        {
                            let label = &code.labels()[target.0 as usize].name;
                            assert!(rendered.text().contains(&format!("default: goto {label};")));
                            assert!(rendered.text().contains(&format!("{label}:")));
                        }
                    }
                }
            }
        },
    );
}

#[test]
fn two_loop_sites_keep_the_between_site_record_and_early_exit() {
    inspect(
        "kotlin/suspend-loop-two-sites",
        "Lfixture/SuspendTwoSites;",
        "total",
        "(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;",
        |facts, code, rendered| {
            assert!(
                facts.coroutine_loop().is_none(),
                "multi-site source folding is not established"
            );
            assert!(code.emission().coroutine_loop().is_none());
            assert_eq!(rendered.text().matches("SuspendTwoSites.load(").count(), 2);
            assert_eq!(
                rendered.text().matches("SuspendTwoSites.record(").count(),
                1
            );
            assert!(rendered.text().contains("case 2:"));
            assert!(rendered.text().contains("I$3"));
            assert!(rendered.text().contains(" < 0)"));
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
            assert!(rendered.text().contains("default: goto "));
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
        |_, _, rendered| {
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
                .find("default: goto ")
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
