# Fixture: exception-finally-transfer

finally with return/break/continue, and try inside catch. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ExceptionFinallyTransfer;`

## Current verdict — round 1, 2026-11-08

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
