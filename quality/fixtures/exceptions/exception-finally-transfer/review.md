# Fixture: exception-finally-transfer

## Current verdict — while/do-while architecture correction

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
