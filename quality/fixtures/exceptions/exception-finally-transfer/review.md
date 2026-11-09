# Fixture: exception-finally-transfer

finally with return/break/continue, and try inside catch. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ExceptionFinallyTransfer;`

## Current verdict — iteration-tail review

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
