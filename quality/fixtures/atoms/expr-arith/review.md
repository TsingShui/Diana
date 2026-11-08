# Fixture: expr-arith

arithmetic, bits, shifts. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `add` | ✔ Equivalent to the reference. |
| `minusOne` | ✔ Equivalent to the reference. |
| `multiply` | ✔ Equivalent to the reference. |
| `divide` | ✔ Equivalent to the reference. |
| `modulo` | ✔ Equivalent to the reference. |
| `bits` | ✔ Equivalent to the reference. |
| `shifts` | ✔ Equivalent to the reference. |
| `negate` | ✔ Equivalent to the reference. |

### Findings

- Arithmetic results agree; expressions are assigned to locals before return. Recorded forms: `int ID = (…)`.
- The wide multiplication `x * 17L` is split across two statements.

### Ranked fixes

Prefer a direct return expression such as `return x + 1;` when the delivered plan permits it.

## Rounds

- R1 2026-11-08: Arithmetic results agree; expressions are assigned to locals before return. Recorded forms: `int ID = (…)`.
