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
