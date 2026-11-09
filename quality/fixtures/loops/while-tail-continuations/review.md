# Fixture: while-tail-continuations

## Current verdict — terminal-follow conjunction, 2026-11-09

`cleanup` now prints `if (p1 == 2 || p1 == 4) throw new
IllegalArgumentException(); else return;`. The mark, membership tests and
throw/return paths are unchanged; the rest of the fixture text is unchanged in the
candidate comparison.

## Current verdict — terminal guard follow-up

Read the changed terminal alternatives and remaining body. The selected
return/throw keeps its original evaluation, while other copies, calls, accesses
and results remain on the opposite path in their original order. Both-edge guard
orientation preserves the predicate's exact meaning; no data, check or effect is
dropped.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior LLM semantic judgement

Compared Java/Smali, concrete helpers and complete ordinary Engine source on the
same prepared DEX, baseline8ac6d95 versus the commit containing this review.
Independent expectations preceded this fixture's first inspection. Recorded
paths agree. No scenario executor, device run or Java recompilation ran; this
is bounded manual judgement, not behavioral PASS.

### Nested private outer continuation

Three loops retain separate row/middle/column state. The private branch calls
touch(100+row*10+middle,fault), adds10, increments row by2, then continues the
outer while. It skips both inner updates and normal outer touch200+row. The
ordinary branch adds column+1 and increments only column; completed inner and
middle loops retain their own updates.

For hit0/1/2 and rows3 the private visits are row0/row2, yielding20/22/26 and
marks100,120. Hit-1 instead visits all six columns per row, marks200/201 and
returns24 for rows2. Zero rows has no body/effect. A fault100 occurs before
private +10/row+2; fault120 occurs after the first completed private visit but
before the second visit's updates. Both propagate ISE without normal outer
marks or another update. The named continue targets the outer loop, not middle
or inner. No tail effect is cloned and no for header conceals the row update.

The seed LoopExits.outerContinue now has the same readable If/outer-continue
shape. P6 already proved the private corridor but formerly discarded its exact
membership/end target. It now retains that private evidence. A contained branch
can own a particular enclosing latch only with the complete corridor, original
entrance, proper nested loop and exact enclosing continue target. Every included
latch needs that proof; ordinary laminarity, entry, owner, construction, lock and
protection guards remain. There is no boundary retry or second loop analysis.

### Shared entrance refusal

shared adds a second original conditional entrance to the tail, at column1.
Those two edges prevent either branch from claiming it private. The source
retains both gotos to one tail and a separate goto around it on normal completion.
For rows3/hit-1 it adds1 then10 at rows0/2, returning22 with100,120. Fault100
propagates before the private additions/update. There is one tail touch call,
not a copy per predecessor; the distinct normal outer call remains. This is an
actual shared-entrance refusal rather than an unrelated type/copy guard.

### Cleanup followed by body update

walk reads row+1, except row1 skips the addition. That bypass still executes
finally and the original parameter-based update. Step1/limit4 returns8 with
10,30,11,31,12,32,13,33; step2/limit3 returns4 with10,30,12,32. Mode3 stops after
row2 cleanup and returns4 without another update/read. Zero limit has no effects.

Mode1 preserves original NFE through cleanup; mode2's normal cleanup IAE skips
update; mode4 cleanup replaces pending Error with IAE. All fault cases record
10,30 only. The update reads the original step at its body point after successful
cleanup. No positivity, invariance or progress assumption is introduced.

### Tail-tested first iteration

tail executes touch60 and its first sum/update before testing, including limits0
and-2, returning0. Limit3 visits60/61/62 and returns3. Fault61 propagates before
the second sum/update. The source remains do-while, not a pre-tested while that
would wrongly skip the first body for nonpositive limits.

## Architecture and supporting checks

The approved architecture already required only while/do-while/while(true).
The prior for recovery contradicted that rule; this change removes it rather
than adding a switch to disable it. P6 has no LoopUpdate producer/product or
special update continuation, P7 has no For placement/header owner, and P9 has no
for renderer/origin-only update traversal. Root/package instructions now repeat
the principle and link the current implementation to the approved contract.

A unique local branch join is nominated before an enclosing follow when no
local postdominator exists. This is general topology using the established
shared-target producer, not update discovery. Only one admitted primary is
queued; owner rejection does not trigger another boundary. This keeps ordinary
shared body work visible without a for-specific split.

Four ordinary-Engine checks cover complete private outer-tail ownership, real
shared-entrance refusal, cleanup/break/update positions and do-while first-body
placement. Existing checks now inspect actual body-update anchors, original
release Follow targets and final scoped/Goto completions instead of requiring
for or one particular compact-condition shape. Original scenario values/effects
were not changed. The initial four old-shape failures were not called passing;
manual reading and current-role checks retain both real breaks and the outer
update target. Product checks do not execute the independent scenarios.

Within these recorded cases while/do-while suffice without lost effects, extra
updates or hidden destinations. Arbitrary helper faults, concurrency and
unmodeled runs remain outside this judgement.

## Executed product evidence

Run `cargo nextest run --locked -p diana-decomp --test while_continuations --no-fail-fast`
from the repository root. All four checks pass on this commit. On isolated
8ac6d95 source with this same prepared DEX/helper/test, the do-while check passes
and the three head-loop checks reject the forbidden for output. This is
architecture-product sensitivity, not a before/after behavioral mismatch claim.

Default and all-feature package suites each pass 74 tests across 18 binaries.
Package Clippy with warnings denied, release/dbg_shell builds, format/diff and
desensitization checks pass. Same-DEX inspection observes 89 fixtures, 140 classes
and 673 methods without stopped members or render failures/issues. Original
P2/P3/P4/P5 and original-coordinate P6 operation/edge/exception/lock/copy-phase
facts agree with 8ac6d95 and the fixed pre-evolution baseline. All 35 current
changed fixture texts have direct current path reviews. These observations do
not execute the independent scenario matrix.
