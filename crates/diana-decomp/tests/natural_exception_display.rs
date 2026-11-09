//! Display-only natural catch coverage through the ordinary Engine entry.
use diana_decomp::p6::{CatchTarget, RegionKind};
use diana_decomp::p7::{ExceptionSyntax, MethodCode};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn clean_typed(code: &MethodCode, rendered: &Rendered) {
    assert!(
        !rendered.text().contains("exception_chain"),
        "{}",
        rendered.text()
    );
    assert!(!rendered.text().contains("[exn "));
    let mut natural = 0;
    for site in code.sites() {
        assert_eq!(rendered.spans().iter().filter(|span| matches!(span.origin, Origin::Site { site: original, .. } if original == site.id)).count(), 1);
        if !matches!(site.syntax, ExceptionSyntax::Natural) {
            continue;
        }
        natural += 1;
        assert_eq!(site.routes.len(), 2);
        let clauses = &code.chains()[site.chain.0 as usize].clauses;
        assert_eq!(clauses.len(), 2);
        assert!(clauses[0].catch.is_some() && clauses[0].target.is_some());
        assert!(clauses[1].catch.is_none() && clauses[1].target.is_none());
        assert!(!code.emission().site_annotation(site.id));
        assert!(!code.emission().show_chains()[site.chain.0 as usize]);
    }
    assert!(
        natural > 0,
        "actual lexical coverage, not propagation-only input"
    );
}

#[test]
fn ordinary_typed_catch_does_not_repeat_its_uncaught_route_as_a_chain() {
    inspect(
        "exceptions/exception-only-completion",
        "Lfixture/ExceptionCompletion;",
        "typed",
        "(ILjava/lang/Throwable;)I",
        |facts, code, rendered| {
            clean_typed(code, rendered);
            assert_eq!(facts.tries().len(), 1);
            assert_eq!(
                facts
                    .throws()
                    .iter()
                    .filter(|site| facts.handler_chains()[site.chain().0 as usize]
                        .clauses()
                        .last()
                        .is_some_and(|clause| clause.target == CatchTarget::Uncaught))
                    .count(),
                facts.throws().len()
            );
            assert!(
                rendered
                    .text()
                    .contains("catch (java.lang.IllegalStateException")
            );
            assert!(rendered.text().contains("throw p1"));
        },
    );
}

#[test]
fn a_loop_retains_its_sum_and_recovery_while_omitting_only_redundant_routes() {
    inspect(
        "exceptions/natural-typed-catch-display",
        "Lfixture/ReadableCatch;",
        "run",
        "(II)I",
        |facts, code, rendered| {
            clean_typed(code, rendered);
            assert_eq!(
                facts
                    .regions()
                    .iter()
                    .filter(|region| matches!(region.kind(), RegionKind::Loop(_)))
                    .count(),
                1
            );
            assert_eq!(rendered.text().matches("ReadableCatch.step(").count(), 1);
            assert_eq!(rendered.text().matches("ReadableCatch.recover(").count(), 1);
            assert!(rendered.text().contains("+ 1)"));
        },
    );
}

#[test]
fn a_throw_keeps_its_operand_and_the_prior_unused_call_effect() {
    inspect(
        "exceptions/natural-typed-catch-display",
        "Lfixture/ReadableCatch;",
        "rethrow",
        "(ILjava/lang/Throwable;)I",
        |_, code, rendered| {
            clean_typed(code, rendered);
            let step = rendered.text().find("ReadableCatch.step(").unwrap();
            let throw = rendered.text().find("throw p1").unwrap();
            let recover = rendered.text().find("ReadableCatch.recover(").unwrap();
            assert!(step < throw && throw < recover);
        },
    );
}

#[test]
fn explicit_shared_handler_context_keeps_its_chain_snapshots_and_receives() {
    inspect(
        "exceptions/natural-typed-catch-display",
        "Lfixture/ReadableCatch;",
        "foreign",
        "(I)I",
        |facts, code, rendered| {
            assert!(facts.tries().is_empty());
            assert!(rendered.text().contains("exception_chain"));
            assert!(rendered.text().contains("[exn c0]"));
            assert!(rendered.text().contains("on_exception_entry"));
            assert!(rendered.text().contains("after_operands_before_effect"));
            for site in code
                .sites()
                .iter()
                .filter(|site| matches!(site.syntax, ExceptionSyntax::Explicit))
            {
                assert!(code.emission().site_annotation(site.id));
                assert!(code.emission().show_chains()[site.chain.0 as usize]);
            }
            assert_eq!(rendered.text().matches("ReadableCatch.step(").count(), 2);
            assert_eq!(rendered.text().matches("ReadableCatch.outside(").count(), 1);
        },
    );
}

#[test]
fn the_broader_multi_handler_shape_retains_all_original_priority_information() {
    inspect(
        "exceptions/natural-typed-catch-display",
        "Lfixture/ReadableCatch;",
        "multi",
        "(I)I",
        |_, code, rendered| {
            let chain = code
                .chains()
                .iter()
                .find(|chain| chain.clauses.len() == 3)
                .unwrap();
            assert!(chain.clauses[0].catch.is_some());
            assert!(chain.clauses[1].catch.is_some());
            assert!(chain.clauses[2].target.is_none());
            assert_eq!(
                chain
                    .clauses
                    .iter()
                    .map(|clause| clause.order)
                    .collect::<Vec<_>>(),
                [0, 1, 2]
            );
            for site in code.sites().iter().filter(|site| site.chain == chain.id) {
                if code.emission().site_annotation(site.id) {
                    assert!(rendered.text().contains("exception_chain"));
                }
            }
            assert_eq!(rendered.text().matches("ReadableCatch.step(").count(), 1);
            assert!(rendered.text().contains("return -2"));
        },
    );
}
