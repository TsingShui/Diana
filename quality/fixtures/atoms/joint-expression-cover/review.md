# Semantic review of selected expression ownership

This is an LLM source/output review, not an executed behavioral equivalence test.
The same prepared input was inspected on `1f49236` and the containing ownership
migration. P9 text, issues, member availability and source-span counts agree.
The engine's four targeted nextest cases check selected ownership and origins;
they do not execute the native helper model or inject VM faults.

## Reviewed paths

`ordered`: the first flag test skips the whole right operand of `||`. With zero
flag, take(1) completes before take(2), which completes before pair receives
11 and 12. Each call appears once. Every helper failure escapes before the next
call or result arm. The shared arm returns 7; the zero pair-result arm returns 9.
The explicit native model makes that zero result unreachable in normal execution,
but its structural preservation still follows from the independent branch input.

`blocked`: the flag's successful path jumps directly to return 7. The other path
keeps both take statements, pair, mark and the final numeric test in that order.
mark observes the stored pair result. Its fault escapes. Refusing compact
Condition syntax does not leave a proposed argument evaluation in the product.

`ordinary`: both takes stay in source order and pair receives their two stored
results. The second take's argument contains the original wrapping addition.
The return contains pair once. This migration preserves the existing result;
full joint ordinary-call recovery is not demonstrated by this baseline comparison.

`reversed`: the first take remains a statement. The second take occurs in pair's
first operand; the second operand reads the already captured first result.
Consequently the effect order remains first take, second take, pair, while the
pair's arguments remain reversed. The output does not reorder calls to resemble
source argument preparation.

`constructed`: allocation precedes the two take evaluations in the selected new
expression, and initialization follows them. The original prepared operations now
retain their lowered statement association directly. The concrete constructor
calls Object initialization, writes left, writes right and returns. Argument or
initialization failure prevents publication. The original allocation/site origins
remain present; no fault was actually executed.

The helper model and DEX sources provide the assumptions for these judgments.
There is no contradiction on the reviewed paths. Runtime equivalence, malformed
input behavior beyond existing parsing contracts and historical fixture regressions
remain unrun in this structural slice.

## Shared suffix-cover review

The extended input is inspected on `c2e1ac5` and the containing suffix-cover
refactor. The two classes retain eleven methods, eight executable results and
533 source spans on both revisions, without member or render failure. These are
availability/origin observations, not an equivalence score. The candidate's
ordinary and tree methods now contain complete nested return forests. The
barrier and blocked methods gain safe suffix composition; constructed, ordered
and reversed retain their previous text.

`ordinary` now evaluates the two takes left to right before pair. The wrapping
addition belongs to the second take's argument. Removing the temporary stores
introduces no other observer or extra helper execution. `tree` evaluates its
whole left pair subtree before its right pair subtree, then calls the outer
pair. Repeated helpers remain distinct operations, and reuse of the original
registers does not substitute later values for earlier operands.

`barrier` retains the first take and mark as statements. Only the second take
moves into pair's second operand, after pair's first operand reads the already
captured value. mark still observes the first take's result and prevents earlier
preparation from entering the later suffix. `blocked` retains both explicit flag
branches and the pair store required by mark and the numeric test. It now nests
both takes in pair's argument list. Arguments execute before the pair call even
though the printed function name precedes their text.

The selected-plan scalar executor covers ordinary, reversed, tree and barrier
on seeds 0, 3, -10 and INT_MAX, including every reachable native-helper fault.
All sixteen normal and sixty-eight fault scenarios pass their independently
authored wrapping results and ordered event prefixes. It follows selected
Sequence/Block, body actions, nested Evaluation edges and materializations;
it does not replay an upstream CFG. The blocked pair subtree additionally
computes 1112 with T(1), T(2), P(11,12). Condition branching, construction and VM
faults remain LLM-reviewed and are not executed by this scalar checker.

No contradiction was found in the changed paths or the two added challenge
subjects. Historical regression and old test-product adaptation remain deferred;
this record does not claim full migration or full-corpus acceptance.
