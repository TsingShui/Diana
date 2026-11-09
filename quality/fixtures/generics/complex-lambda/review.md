# Fixture: complex-lambda

## Current verdict — whole-operation follow-up

Read the changed loop/local placement alongside the retained lambda bodies and
bootstrap facts. Captures, original calls and evaluated arguments remain once
and in order; scalar repetition still follows the same body. No invokedynamic
meaning is reconstructed from external declarations.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


lambdas, capturing lambda, anonymous supplier, method reference and comparator. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `printer` | ✔ Capture uses ComplexLambda$0(this, p0); the original lambda body identity is retained. |
| `counter` | ✔ The anonymous ComplexLambda$1 class retains its state. |
| `applyTwice` | ✔ |
| `sorted` | ? Not reviewed. |
| `describe` | ✔ append("value:").append(p0) agrees. |
| `prefixer` | ✔ The method reference appears as ComplexLambda$2. |

### Findings

- The lambda body keeps its original synthetic name and exposes dex_flags<synthetic> in the body. Recorded forms: `lambda$printer$0$fixture-ComplexLambda`.
- The method-reference implementation appears under its original R8-generated synthetic name. Recorded forms: `$r8$lambda$ey1lUbfxWqoov2vHAeHNf7_uUW8`.
- Comparator.naturalOrder() in sorted was not confirmed.

### Ranked fixes

Improve lambda, synthetic-member and flag presentation while retaining their identities and readable bodies. Recorded forms: `dex_flags<>`, `list.sort(...)`.

## Rounds

- R1 2026-11-08: The lambda body keeps its original synthetic name and exposes dex_flags<synthetic> in the body. Recorded forms: `lambda$printer$0$fixture-ComplexLambda`.
