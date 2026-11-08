# Fixture: complex-indy-lambda

lambdas, captured lambdas and every method-reference kind. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ComplexIndyLambda;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `field` | ✔ |
| `staticValue` | ✔ |
| `constructorRef` | ✔ The method-handle kind and StringBuilder constructor prototype are retained. |
| `listFactory` | ? Not reviewed line by line. |
| `arrayFactory` | ? Not reviewed line by line. |
| `captured` | ✔ Captured this/p0 arguments and the lambda$captured$1 body identity are retained. |
| `boundField` | ? Not reviewed line by line. |
| `unboundStatic` | ? Not reviewed line by line. |
| `stringOps` | ? Not judged. |

### Findings

- Method and constructor references expose LambdaMetafactory call-site metadata plus invoke-custom.
- The remaining presentation difference is the explicit indy protocol.
- Captured this/p0 values appear in the original call arguments; the behavior is readable but not source-style lambda syntax.

### Ranked fixes

Improve constructor/method-reference and lambda presentation while retaining original members and bootstrap facts.

## Rounds

- R1 2026-11-08: Method and constructor references expose LambdaMetafactory call-site metadata plus invoke-custom.
