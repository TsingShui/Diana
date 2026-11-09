# Fixture: nested-loop-exits

## Current iteration-tail review

Direct LLM comparison of the Java explanation, actual Smali, mark helper model
and final ordinary Engine output. The subject owner is `Lfixture/LoopExits;`.
This is path-reading evidence, not executed behavioral PASS.

| Original method | Reviewed behavior |
|---|---|
| `nestedBreak(III)I` | A hit calls mark(row), adds 10 and leaves only the inner loop; the outer update still runs. Nonhits add column before its original increment. |
| `outerBreak(III)I` | The marked/add-10 path leaves the labelled outer for before its update; no inner update is executed either. |
| `outerContinue(III)I` | Unchanged: two distinct outer update sites remain in an ordinary while, including mark/add-10/update before the labelled continuation. |
| `sharedExit(III)I` | Both original hit tests still enter the one marked block. Normal inner completion continues the outer for without marking; hit performs mark/add-10 before its update. |
| `siblingLoop(II)I` | Hit jumps out of the first for before its update into the separate second loop. Normal first-loop exhaustion skips that second loop. Each second-loop mark precedes its own update. |
| `protectedExit(III)I` | Hit retains its original protected mark and catch returning -1. Success adds 10 before the outer update; normal inner exhaustion skips that private marked path. A throwing mark never executes add/update. |

The changed schedules extract only the existing pure outer updates. They do not
clone marks, move the shared exit into either test arm or reinterpret a sibling
loop as an enclosing one. Remaining gotos into shared/protected exit content
retain their original labels and copies. The protected and shared methods now
use an outer continue for their ordinary path, so it still reaches the update
without executing the separately entered marked block.

Original P2/P3/P4/P5 products and relevant original P6 graph/protection facts
agree before/after. Six existing `nested_loop_exits.rs` checks remain regression
support; they are structural/source-product checks, not a mark-scenario executor.
The new [iteration-tail challenge](../iteration-tail-cleanup/review.md) covers
nested updates, cleanup, return/break and update-skipping backedges.

No device execution or Java recompilation ran. Arbitrary native mark side effects
and VM failures outside the README model remain outside this path judgment.
