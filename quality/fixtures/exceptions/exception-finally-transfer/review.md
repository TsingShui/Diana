# Fixture: exception-finally-transfer

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `all v6`, `all v7`, `NumberFormatException v2`, `NumberFormatException v5`. The original captured object is bound once at exceptional
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

## Prior verdict — return replacement in finally

Compared the same prepared DEX on `b214ec6` and this repair. Only
`Lfixture/ExceptionFinallyTransfer;->returnInFinally(I)I` changes. Read its Java
and original P1 facts against both complete method texts.

### Semantics

| Path | Source and recovered behavior |
|---|---|
| Ordinary input | State1 remains in try, then state2 executes in finally. A nonnegative parameter returns the original parameter after cleanup. |
| Negative input | State2 precedes `p0 < 0`; that branch returns -1 and replaces the pending parameter return. |
| Original exceptional body completion | The original caught error enters state2 cleanup. Negative input replaces it with -1; otherwise normal cleanup implicitly rethrows that same capture. A failure of the cleanup store remains outside its own protection and replaces the pending outcome. |

The original field target, operand states, effect order and unhandled cleanup
routes are unchanged. The inverse predicate reflects removing the implicit
pending rethrow, not dropping an executable branch. Other methods are byte-exact
in P9 and reuse the prior loop/nested-catch reviews below. P1/P3/P4/P5 agree.
The [new challenge](../abrupt-finally-completion/review.md) adds saved-state,
throwing-close and all-abrupt interactions. No contradiction was found under
sequential DEX completion semantics. No behavioral executor/device/Java run is
claimed; historical execution statements below were not rerun.

## Prior verdict — terminal guard follow-up

Read cleanup overrides and nested catches. Negative finally reads retain their
explicit jump past the update; rethrow/return guards preserve the chosen pending
outcome. State writes and original later work remain on exactly the previous
paths.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — while/do-while architecture correction

Read explicit cleanup overrides. breakInFinally's negative read reaches state3
outside the loop without the index update; nonnegative cleanup writes state4
then updates. A failing finally read replaces the pending fault.
continueInFinally writes state5 before its body update. Return and throw
overrides remain explicit.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Read Java and complete final methods. breakInFinally's original goto still
bypasses the for update and reaches state3; normal nonnegative cleanup selects
state4. A failure during the finally read replaces a pending exception.
continueInFinally's state5 precedes its original update on either normal branch.
Return overrides and nested typed-catch parse/trim order remain explicit; these
overriding cleanups are not unsafely folded.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


finally with return/break/continue, and try inside catch. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ExceptionFinallyTransfer;`

## Prior verdict — iteration-tail review

Java and final text were reread for
Lfixture/ExceptionFinallyTransfer;->continueInFinally([I)I. Zero still skips
total++ but both normal branches write state=5 before the original index update.
Either protected read failure writes 5 and rethrows without updating the index;
failure in the finally write also skips the update. Return/break and nested-catch
subjects are unchanged.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `returnInFinally` | ✔ Finally return -1 overrides the pending try return. |
| `breakInFinally` | ✔ Finally break discards the pending exception and exits the loop through L6: state=3. |
| `continueInFinally` | ✔ Finally runs on both continue and normal paths. |
| `tryInsideCatch` | ✔ The nested try inside catch retains both protection chains and catches. |

### Findings

- Finally return/break/continue expand into gotos and duplicated cleanup rather than a finally keyword.
- H10 falls into L6: state=3 and returns, matching break-in-finally discarding the exception, though the text can look misleading. Recorded forms: `breakInFinally`.
- Per-operation exn markers, region labels and exception-chain declarations are interleaved. Recorded forms: `[exn c1]`, `R9:`, `R7:`, `exception_chain`.

### Ranked fixes

Improve finally and labelled-exit presentation while retaining the original protection and transfer facts.

## Rounds

- R1 2026-11-08: Finally return/break/continue expand into gotos and duplicated cleanup rather than a finally keyword.
