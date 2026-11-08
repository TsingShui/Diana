# Fixture: complex-lambda

lambdas, capturing lambda, anonymous supplier, method reference and comparator. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

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
