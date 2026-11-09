# Fixture: iteration-tail-cleanup

## Current verdict — terminal-follow conjunction, 2026-11-09

`cleanup` now prints `p1 == 5 || p1 == 6` before its `IllegalArgumentException`;
`read` prints `p1 == 4 || p1 == 6` before its `AssertionError`. The successful
`p0 + 1` result and both throw/return paths are unchanged; the rest of the fixture
text is unchanged in the candidate comparison.

## Current verdict — terminal guard follow-up

Read the normal/typed negative-result return guard: cleanup still precedes it,
value2 skips only addition, and break/return/fault skips the original inner
update. Other paths update once; the outer update target remains visible. Helper
mode tests keep mark-before-fault and selected exception classes.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — while/do-while architecture correction

Read nested normal/typed/cleanup-fault paths. The value2 path skips addition but
still reaches the original inner update; return/break/fault skips it. The outer
bypass now goes to its visible update, skipping the inner body. locked releases
before its visible counter assignment. skipUpdate still retests without
increment on its flagged backedge; stride reads the original step at the body
tail.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Compared the same prepared DEX with completed-eleven-repair bda9a49. Only
derived label/control coordinates change. Development-only correspondence found
identical printed words, original L/H program points and identical printed
lexical-scope bindings across every method publication; declaration-only members
are exact. Original P2/P3/P4/P5 facts agree. The unchanged semantic path review
below is reused, not replaced by a naming score or a claim of behavioral
execution.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


## Prior verdict — nonconstant update review

Added stride(II)I has pre-inspection Java/Smali scenarios. The same prepared DEX
on pre-update and candidate binaries preserves its mark/add/update/test order.
The candidate now prints the one supplied row-plus-step operation in a for header;
it reads p1 after that iteration's body, not at initialization or before the test.
The recorded rows0,2,4,6 sum12; rows0,3 sum3; zero limit has no body/update, and
step MAX at limit1 marks0 then exits. Zero/negative-step progress is not inferred.
Other member texts are unchanged and reuse their previous reviews.

The new [mutable-step challenge](../mutable-iteration-step/review.md) changes step
inside a protected body and adds no-body nonlocal condition completions. Its
initial candidate exposed and blocked a missing-break regression before the
owning producer/consumer fact was repaired. Final reading and package checks
support bounded effectiveness, not behavioral execution PASS.

## Prior verdict — monitor continuation review

Only locked(Object,int) changes in this follow-up. Java/Smali and final text
were reread: acquire and mark still precede the explicit release, then the supplied
for update runs. Failure before/inside the held body releases only when acquired
and skips the update; null and zero-row behavior is unchanged. The old ordinary-
while barrier below is historical: P6 now supplies its missing selected-loop
Continue role. The two deliberately bypassing backedges in skipUpdate remain while;
nested and all helper text are unchanged.

This follow-up is bounded direct LLM path reading, not executed behavioral PASS.
Corpus inspection preserves exact P2/P3/P4/P5 and original P6 graph/lock facts,
including synchronized release targets, routing and before/after plans. Unchanged
text reuses prior reviews. See the new
[monitor-exit challenge](../../sync/monitor-iteration-exits/review.md) for scope
and the initial candidate regression; no device run or Java recompilation ran.

## Prior LLM semantic review

Compared Java, actual Smali, concrete helper bodies and final ordinary Engine
products/text. Independent scenarios were recorded before candidate inspection.
The same prepared DEX is used on the recorded pre-tail working tree based on
`ebbc003` and the candidate. No Java recompilation, device execution or independent
behavioral scenario executor ran.

| Original subject | Judgment |
|---|---|
| `Lfixture/IterationTail;->nested(III)I` | Equivalent on the recorded paths: both loop updates, cleanup-before-exit, inner break and outer continuation agree. |
| `Lfixture/IterationTail;->skipUpdate(II)I` | Equivalent: its direct header backedge deliberately skips row++, and remains while. |
| `Lfixture/IterationTail;->locked(Ljava/lang/Object;I)I` | Equivalent: specialized release routing and the visible ordinary update remain intact. |
| `read(II)I`, `cleanup(II)V`, `mark(I)V` | Same event-before-result/throw rules, original arguments and wrapping trace arithmetic. |

### Nested normal paths

The outer row starts at zero and resets the inner column only on entry to that
row's body. For (2,0,0), codes 0..4 each read then clean. Value 2 skips addition
but still advances the column; value 5 adds before breaking only the inner loop,
without its pending update. The outer update still runs. The result is 13.

With flag=1, row 0 contributes 4. Row 1's early continue reaches the outer update,
not the inner initialization or a repeated row-1 test, so the result is 4. Neither
counter was hoisted, duplicated or retargeted to the other loop. Code computation
remains before read, and cleanup uses that same iteration's code.

### Typed, abrupt and replacement paths

NFE/NPE enter one shared body, mark 20, select 100, clean and continue. All six
iterations return 600; the row-1 flag skip leaves only three and returns 300.
The broad RuntimeException path marks 21, selects -2, cleans then returns before
either update. Error bypasses typed handlers; successful cleanup preserves the
original object. Mode-5 normal cleanup failure occurs before addition or update.
Mode 6 replaces the pending Error with IAE. No failed cleanup is retried. Zero
rows skip both loops and every helper, even in an error mode.

### Admission barriers

skipUpdate's flag path increments total, not row. Its first two marks are 50,50
and it returns 2; a falsely unconditional for update would instead produce 50,51.
The final while retains the direct header continuation and its two original
backedges. Normal flag=0 still marks 50,51,52 and returns zero.

locked's non-null case acquires, marks and explicitly releases before its row
update. Exceptional release retains the captured object; the closing brace is
not an extra release. Null acquisition fails before mark/release, and zero rows
do not acquire. The release producer's Follow(update) still names a visible
ordinary update; the candidate does not hide that target in a header without a
compatible source-continuation product.

## Owning mechanism and challenge result

P6 now publishes one ordinary pure update plus its original unique backedge as a
source-compatible iteration continuation. Core/evaluation/exit facts remain;
body completion enters that update, then repeats at the original header. The
ordinary branch no-boundary-retry policy is unchanged. P7 consumes the supplied
body/predicate/update roles, and P9 only spells that schedule. The original update
has one nonempty operation anchor in its header; retained update control/routing
and zero-step copies have origin-only traversal, not another execution.

The initial seed became a for with no value-2 goto. This challenge initially
recovered only its outer for: the update guard mistakenly treated a containing
loop as foreign partial ownership. The final rule permits a true whole-loop
ancestor but still rejects an inner/sibling corridor owning another loop's update.
Both nested updates now recover; the two deliberate barriers remain ordinary.
A pre-test assignment case from loop-for-multi additionally confirms that original
predicate evaluation is not moved into the update or skipped on the first test.

The iteration is effective within this reviewed scope: it generalizes through
nested cleanup and preserves the negative guards and prior effects. This is not
universal semantic verification or commit authorization.

## Supporting evidence

Five `crates/diana-decomp/tests/iteration_tails.rs` checks consume ordinary Engine
delivery. They cover unique update anchors and original targets, nested header
ownership, pre-test evaluation, a bypassing backedge and specialized monitor routing.
They are structural/product checks, not execution of the scenarios above.

Corpus inspection covers 79 fixtures, 130 classes and 606 methods with no stopped
member, render failure or render issue. Exact P2/P3/P4/P5 and relevant original P6
control/protection/lock facts agree. All 19 changed fixture texts were read against
their source, Smali or available prepared-input facts and explicit helper models;
no definite mismatch was found in those reviewed paths. Unchanged member text
reuses prior judgments. Remaining complex gotos and unsupported update schedules
stay readable rather than becoming an invented dispatcher.

Final default and all-feature nextest each passed 35 package checks. Package
Clippy with warnings denied, release and dbg_shell builds passed. Behavioral
execution remains unrun; constructor/VM/concurrent effects outside the README
assumptions are not covered.
