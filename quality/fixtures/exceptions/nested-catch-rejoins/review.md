# Fixture: nested-catch-rejoins

## Natural catch display

Both the loop subject's typed/union/RuntimeException/finally shape and the
synchronized subject's union catch plus all-catch now print without chain text or
handler labels. Monitor enter/release ordering, `mark(20)`/`mark(21)`, `v2=100`,
`-2`, the cleanup call and its own explicit `c3` rethrow chain are unchanged. Text and JSON publish together; owned sites, chains, completions, snapshots
and receives remain. Renewed LLM reading and product equality, not executed
behavior.

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `NumberFormatException | NullPointerException v3`, `RuntimeException v4`, `NumberFormatException | NullPointerException v6`, `all v10`. The original captured object is bound once at exceptional
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

## Prior verdict — terminal-follow conjunction, 2026-11-09

`cleanup` now prints `p1 == 5 || p1 == 6` before its `IllegalArgumentException`;
`read` prints `p1 == 4 || p1 == 6` before its `AssertionError`. The successful
`p0 + 1` result and both throw/return paths are unchanged; the rest of the fixture
text is unchanged in the candidate comparison.

## Current verdict — terminal guard follow-up

Read scan's negative return guard and helper branches.
Cleanup/addition/update/threshold order remains, including the shared typed
capture. Synchronized run/release targets and other loop ownership are unchanged
by this P7-only display choice.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — while/do-while architecture correction

Read run's released normal paths: goto reaches each visible inner update after
release, not a dangling hidden header label; outer update follows complete inner
traversal. Shared catches, cleanup arguments/failures, captures and both
original update points remain. No update executes after release failure/rethrow.

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


## Prior verdict — monitor continuation review

The final run(Object,int,int) now has both for updates. Java/Smali, helper models
and final changed path were reread: code is computed before acquisition; read,
typed recovery, held cleanup and addition precede release; only then does column
advance. Thus (lock,2,0) remains 21 and typed modes remain 600. Mode3/Error,
normal cleanup failure and replacement release then throw without either pending
update; null and zero-row behavior is unchanged. Foreign and helper text are
unchanged. An initial candidate printed goto L13 before continue while hiding L13
in the header: a definite regression. The shared P7 ReleasedSync consumer was
repaired and a direct-outlet regression check added before this judgment.

This follow-up is bounded direct LLM path reading, not executed behavioral PASS.
Corpus inspection preserves exact P2/P3/P4/P5 and original P6 graph/lock facts,
including synchronized release targets, routing and before/after plans. Unchanged
text reuses prior reviews. See the new
[monitor-exit challenge](../../sync/monitor-iteration-exits/review.md) for scope
and the initial candidate regression; no device run or Java recompilation ran.

## Prior verdict — iteration-tail review

The original Java/Smali/helper models and current changed blocks were reread.
Only NestedCatchLoops.run's outer row update moves to a for, after all inner
monitor acquisitions, held cleanup and releases. Its inner loop remains while
because specialized release-to-step routing has no compatible source update
continuation yet. Null/zero and exceptional replacement paths skip the outer
update as before. foreign text is unchanged. The copied helper scan has the
same reviewed per-iteration for placement as its parent fixture.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict

Finally follow-up: the included helper scan now prints its per-iteration cleanup
as finally, with the same reviewed paths and original transfers. The
NestedCatchLoops run/foreign subject text is byte-for-byte unchanged in that
comparison; their lock and foreign-entry judgments below are reused. See
`../branching-finally-suffix/` for the new cleanup challenge and pending-rethrow
safety boundary. No additional behavioral execution is claimed.

Direct LLM reading of both Java/Smali pairs and final ordinary Engine output.
The prepared DEX is identical in each comparison. The baseline catch-group
working tree on `ebbc003` has no natural loops in the subjects. The candidate
repairs private loop-entry construction; an intermediate maximal-SCC approach
was insufficient for the inner loop and was replaced, not retained as a retry.
These are semantic reading judgments, not executed behavioral scenarios.

| Original subject | Judgment |
|---|---|
| `Lfixture/NestedCatchLoops;->run(Ljava/lang/Object;II)I` | Equivalent on the independently recorded scenarios: loop bounds, index reset, values, held cleanup and exceptional release agree. |
| `Lfixture/NestedCatchLoops;->foreign(II)I` | Equivalent: the real pre-loop exceptional arrival and its saved row/total are retained, even when rows is zero. |
| `Lfixture/LoopCatchFinally;->scan(II)I` | Same reviewed behavior as the parent fixture, now with a natural while. |
| Helper `priority`, `read`, `cleanup`, `mark` | Identical source bodies and unchanged readable output; parent path/model reviews reused. |

### Nested normal and typed paths

The output initializes row and total once, column inside each outer iteration,
and code=row*3+column before acquiring the original lock. Both original loop
conditions and updates are retained. A successful read stores code+1; cleanup
runs while held, then total changes, then release occurs before column advances.
Two rows therefore sum 1..6 to 21 with the recorded alternating read/cleanup marks.

Modes 1/2 enter one shared typed capture, mark 20 and select 100. Cleanup still
runs held, and total changes once. Six iterations yield 600, with one catch mark
per iteration. No handler body or call was duplicated by grouping or nesting.

### Abrupt paths and identity

Mode 3 bypasses the typed catches, runs cleanup and rethrows the captured ISE;
mode 4 does the same for Error. That rethrow routes to the monitor handler, which
releases and throws the same pending object. Mode 5 throws from normal cleanup;
it routes directly to the monitor handler, never retries cleanup and never adds
the value to total. In mode 6 cleanup replaces the pending Error with IAE, then
the monitor releases before propagating that IAE. There is no suppression call.

The source output explicitly says synchronized releases are explicit. Its normal
release jumps past the exceptional release label; the two are not sequential or
additional implicit Java releases. A null lock fails at acquisition before either
helper; zero rows skip acquisition, even for null. VM/linkage/concurrent faults
outside the README model are not covered.

### Genuine foreign arrival

`foreign` deliberately remains a graph. The first read snapshots row=total=0
before its effect. On NFE/NPE, the shared capture restores those states, marks 20,
selects 100 and enters the original common addition/update join before the loop
test. Thus (0,1) returns 100 rather than zero. Later body failures snapshot the
current row/total and use the same handler; (2,1/2) returns 200. Successful mode 0
ignores the first read's result and sums the later 1 and 2. An initial ISE is
uncaught. No catch was moved into a while that would hide the first arrival.

## Mechanism and effectiveness

Normal LT remains unchanged. Original normal-DFS retreating links nominate
headers; owning-stage predecessor closure admits only internally entered,
reachable catch corridors with no foreign normal or exceptional interior arrival.
An SCC alone is not evidence of a natural loop. Applying this to each header,
not only a maximal SCC, recovers both nested loops and preserves the monitor.

This new challenge demonstrates generalization beyond the seed: two nested
loops survive held cleanup and typed rejoin, while a same-handler foreign entry
correctly prevents the private-loop claim. The iteration is effective within
these reviewed scenarios, not a claim of universal semantic verification or
commit authorization.

## Supporting checks

`crates/diana-decomp/tests/loop_handler_entries.rs` checks the seed's single loop,
two selected nested loops, their containment, one synchronized region, unique
original monitor/call/capture source anchors, and refusal of the foreign guard.
All 25 existing/new package structural checks passed with all features. They do
not execute these scenarios or prove lock behavior independently.

Before/after corpus inspection covered 77 fixtures, 128 classes and 594 methods:
no stopped member, render failure or render issue. Original P2, P3/P4 and relevant
storage/control/exception/lock facts agree. Only this fixture and its parent text
changed relative to the catch-group baseline; other readable results are identical.
No Java recompilation, device execution or independent behavioral executor ran.

```sh
cargo nextest run --locked -p diana-decomp --test loop_handler_entries
```
