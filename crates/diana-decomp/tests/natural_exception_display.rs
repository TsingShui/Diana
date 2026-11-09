//! Display-only natural catch coverage through the ordinary Engine entry.
use diana_decomp::p6::{CatchTarget, RegionKind};
use diana_decomp::p7::{ExceptionSyntax, MethodCode};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

fn clean_natural(code: &MethodCode, rendered: &Rendered) {
    assert!(
        !rendered.text().contains("exception_chain"),
        "{}",
        rendered.text()
    );
    assert!(!rendered.text().contains("[exn "));
    let mut natural = 0;
    for site in code.sites() {
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(|span| matches!(span.origin, Origin::Site { site: original, .. } if original == site.id))
                .count(),
            1
        );
        if !matches!(site.syntax, ExceptionSyntax::Natural) {
            continue;
        }
        natural += 1;
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
            clean_natural(code, rendered);
            assert_eq!(facts.tries().len(), 1);
            let clauses = &code.chains()[code.sites()[0].chain.0 as usize].clauses;
            assert_eq!(clauses.len(), 2);
            assert!(clauses[0].target.is_some() && clauses[1].target.is_none());
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
            clean_natural(code, rendered);
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
            clean_natural(code, rendered);
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
fn a_two_handler_chain_is_covered_by_its_printed_catch_order() {
    inspect(
        "exceptions/natural-typed-catch-display",
        "Lfixture/ReadableCatch;",
        "multi",
        "(I)I",
        |_, code, rendered| {
            clean_natural(code, rendered);
            let chain = code
                .chains()
                .iter()
                .find(|chain| chain.clauses.len() == 3)
                .unwrap();
            assert_eq!(
                chain
                    .clauses
                    .iter()
                    .map(|clause| clause.order)
                    .collect::<Vec<_>>(),
                [0, 1, 2]
            );
            assert!(chain.clauses[0].catch.is_some() && chain.clauses[1].catch.is_some());
            let first = rendered
                .text()
                .find("catch (java.lang.IllegalStateException")
                .unwrap();
            let second = rendered
                .text()
                .find("catch (java.lang.IllegalArgumentException")
                .unwrap();
            assert!(first < second, "original typed priority order");
            assert!(rendered.text().contains("return -2"));
        },
    );
}

#[test]
fn nested_catches_share_no_type_priority_ambiguity_inside_their_finally() {
    inspect(
        "exceptions/exception-only-completion",
        "Lfixture/ExceptionCompletion;",
        "nested",
        "(ILjava/lang/Throwable;)I",
        |facts, code, rendered| {
            clean_natural(code, rendered);
            assert_eq!(facts.tries().len(), 2);
            assert_eq!(rendered.text().matches("finish(").count(), 1);
            let inner = rendered
                .text()
                .find("catch (java.lang.IllegalStateException")
                .unwrap();
            let cleanup = rendered.text().find("finally {").unwrap();
            assert!(inner < cleanup);
        },
    );
}

#[test]
fn two_enclosing_same_type_handlers_keep_their_lexical_priority() {
    inspect(
        "exceptions/nested-catch-display",
        "Lfixture/LayeredCatch;",
        "layered",
        "(I)I",
        |_, code, rendered| {
            clean_natural(code, rendered);
            let text = rendered.text();
            let inner = text
                .find("catch (java.lang.IllegalStateException v2)")
                .unwrap();
            let outer = text
                .find("catch (java.lang.IllegalStateException v4)")
                .unwrap();
            assert!(inner < outer, "the inner handler prints first");
            assert_eq!(text.matches("LayeredCatch.recover(").count(), 2);
            // Three original cleanup sites print once each: the outer all-catch,
            // the normal body completion and the inner caught-normal completion.
            assert_eq!(text.matches("LayeredCatch.finish(").count(), 3);
            // The DEX protects the inner recovery interval with the same clause
            // set, so its own nested scope remains visible rather than merged.
            assert_eq!(text.matches("try {").count(), 3);
        },
    );
}

#[test]
fn a_catch_body_call_keeps_its_own_inner_handler_only() {
    inspect(
        "exceptions/nested-catch-display",
        "Lfixture/LayeredCatch;",
        "catchBody",
        "(I)I",
        |_, code, rendered| {
            clean_natural(code, rendered);
            let text = rendered.text();
            let recovery = text.find("LayeredCatch.recover(").unwrap();
            let second = text.find("LayeredCatch.step(1,").unwrap();
            assert!(recovery < second);
            assert_eq!(text.matches("catch (").count(), 2);
            assert!(text.contains("return (v3 + v4)"));
        },
    );
}

#[test]
fn a_printed_union_clause_states_both_original_types_at_one_handler() {
    inspect(
        "exceptions/nested-catch-display",
        "Lfixture/LayeredCatch;",
        "aliased",
        "(I)I",
        |_, code, rendered| {
            clean_natural(code, rendered);
            let chain = code
                .chains()
                .iter()
                .find(|chain| chain.clauses.len() == 3)
                .unwrap();
            assert!(chain.clauses[0].catch.is_some() && chain.clauses[1].catch.is_some());
            assert_eq!(chain.clauses[0].target, chain.clauses[1].target);
            let text = rendered.text();
            let union = text
                .find("catch (java.lang.IllegalStateException | java.lang.IllegalArgumentException")
                .unwrap();
            assert!(text[union..].contains("LayeredCatch.recover("));
            assert_eq!(text.matches("LayeredCatch.recover(").count(), 1);
        },
    );
}
