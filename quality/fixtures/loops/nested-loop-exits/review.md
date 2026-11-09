# Fixture: nested-loop-exits

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `RuntimeException v7`. The original captured object is bound once at exceptional
entry, before the unchanged receiving/body actions. Its exact-state uses remain
inside that handler and have no capture-phase, Phi or copy dependency requiring
an outer assignment. Normal execution does not bind or run the handler.

After accounting for that binding and its removed private declaration/assignment,
the complete remaining output is exact. Original P1/P3/P4/P5 products agree.
Thus the earlier independently reasoned normal, caught, cleanup, rethrow and
secondary-failure paths below are reused: their calls, arguments, values,
priority, transfers and effect order did not change. The renewed LLM judgment
finds no contradictory capture lifetime or exception identity. Structural
comparison supports this reading rather than supplying behavioral proof.

The [fresh monitor/binding challenge](../../sync/monitor-surface-boundaries/review.md)
covers capture identity, receiving/value flow, nested failures and escaping
storage. Current fixture/model regressions are separate executed evidence;
this record does not claim VM/device execution for this input.

## Prior verdict — while/do-while architecture correction

Read Java/Smali and final
nestedBreak/outerBreak/outerContinue/protectedExit/sharedExit/siblingLoop.
Private outerContinue now owns its mark/add/row-update tail inside the If with
exact outer Continue. The protected and two-entrance tails remain explicit and
their normal paths jump around them to the visible outer update.
RuntimeException returns -1 before update. The sibling-loop entry reaches its
test before its first update, while no-hit completion skips it.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Reread Java, Smali and all final methods. Inner/outer Break roles remain
distinct; outerContinue keeps its two explicit row updates rather than inventing
one shared tail. protectedExit/sharedExit gotos still reach their outside-inner
mark/add tail, then exactly one outer update; normal no-hit completion takes the
update without that tail. Protected RuntimeException returns -1 before update.
siblingLoop's entry goto reaches the second loop's test before its first update,
while no-hit completion skips that loop.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


## Prior iteration-tail review

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
