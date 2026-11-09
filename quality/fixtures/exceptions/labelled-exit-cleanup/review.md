# Fixture: labelled-exit-cleanup

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `all v8`, `all v9`. The original captured object is bound once at exceptional
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

## Prior verdict — terminal guard follow-up

Read the over100 return guard. Close precedes return and skips the inner update;
otherwise the same update follows. Outer continuation, negative break, pending
capture/suppression and remaining closes are unchanged.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — while/do-while architecture correction

Read the newly structured private tail. Matching read closes once, increments
row once and continues the outer while, skipping inner/default close. Other
nonnegative reads add then either update the inner counter or close/return
over100. Negative reads close/break outer. Failed reads retain
suppression/rethrow; failed close runs no later update.

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


Try-with-resources in an outer loop with labelled continue, labelled break and a
return inside a nested loop. `LabelledExitCleanup.java` explains the behavior;
`LabelledExitCleanup.smali` is the assembly input. The prepared `classes.dex` was
assembled from the Smali and copied beside it.

- Provenance: authored for this repository; no third-party code or data.
- Original owner: `Lfixture/LabelledExitCleanup;`.
- Subject: `Lfixture/LabelledExitCleanup;->scan(II)I`.
- R1 review base: repository HEAD `6d663d9`; engine sources unchanged from
  `12de370`.
- Expected behavior: the scenario table in `README.md`.

## Prior verdict — iteration-tail review

The Java explanation, actual Smali, README helper model and final scan(II)I were
reread. Only the inner column becomes for; the outer while keeps its separate
row updates. Limit match closes, updates row and continues OUTER without a
column update. Negative closes then breaks outer with neither update; threshold
closes then returns 111 without either update. Read failure still closes,
suppresses on the original throwable and rethrows. A failing normal close cannot
retry or increment. The normal 48, limit-match 4, threshold 111, acquisition-failure
and zero-row paths retain their recorded calls.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 2

Manually reread the Java explanation, Smali and fresh P6/P9 delivery at
`09e322b`, using the README's native-helper models. Continue closes before the
row increment and outer continuation; break closes without incrementing the row;
return closes after saving the updated total. Read failures suppress a failing
close on the same primary. Acquisition and normal-exit close failures remain
outside that resource handler. No new semantic mismatch was found.

The two zero initializations now appear separately in their original definition
order. This changes no call, result or loop path. The remaining `goto L9` is a
readability limitation, not a demonstrated wrong transfer. Fresh delivery has no
stopped member or rendering issue. Behavioral execution remains unrun.

## Prior review — round 1

### Semantics

This is a manual source/output reading judgment, not an automated execution result.

| Method | Semantics |
| --- | --- |
| `scan` | ✔ Equivalent by path comparison: every inner-loop exit closes the current handle once, in the same order as the source, and exceptional close failures suppress on the primary. |

### Path comparison

The current output keeps the outer loop with an `R39:` label, the inner loop, and
one `try`/`catch (all)` around the read plus its control comparisons. The normal
close and each exit-specific close are outside that catch.

| Path | Reference and recovered behavior |
|---|---|
| Normal inner completion | Four reads for row 0, close handle 0, increment row, repeat for row 1, close handle 1, leave the outer loop and return the accumulated total. The output's `while (v6 < 4)` has the same four iterations and the same post-loop close and row increment. |
| Labelled continue | When the read equals `limit`, the output calls `close(v2)`, increments the row, and executes `continue R39`. `R39` is the outer loop, so the pending inner work and the remaining columns are skipped exactly as `continue outer` requires. |
| Labelled break | When the read is negative, the output calls `close(v2)` and executes `break R39`, leaving the outer loop without the row increment and returning the accumulated total. |
| Return | When the total exceeds 100, the output calls `close(v2)` and returns the accumulated total. The read value has already been added. |
| Read failure | The read is inside the catch. The handler captures the throwable, closes the handle, suppresses a close failure on the captured throwable, and rethrows. The exit closes are not in this protected range. |
| Normal close failure | A failing close on the normal inner completion, continue, break or return path is printed after the close that can throw and is not wrapped by the same catch. The failure therefore propagates and the pending transfer is discarded. |
| Acquisition failure in row 1 | The outer row-0 close has already run; `open(1)` is outside any handler and propagates without closing an unacquired handle. |

The output's `if (v3 != p1) { goto L9; }` is an inverted branch with a goto, and
`continue R39` is printed explicitly. These are shape differences; the branch
targets and the close placements agree with the source. The output also prints the
increment before `continue R39` because the source's outer `for` update runs on
that path; the outer `while` has no separate update section.

### Readability

The method is readable as nested loops with a natural inner `try`/`catch`; the
remaining friction is the explicit close before each exit, the `goto L9` used for
the continue-versus-body split, the `R39:` label, and the generated capture names.
No `exception_chain` line is needed. The absence of a compact `try (resource...)`
form is a shape limitation, not a semantic mismatch.

### Commands

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/labelled-exit-cleanup/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/labelled-exit-cleanup/classes.dex \
  --exec '{"cmd":"p6-all","details":true}'
```

No automated behavioral execution and no Rust fixture test were run.

## Rounds

- R1: Prepared `classes.dex` from the authored Smali; direct semantic reading
  finds the normal and exceptional loop exits consistent with the source. The
  remaining limits are explicit notation and the unrun behavioral executor.
