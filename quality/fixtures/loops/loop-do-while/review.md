# Fixture: loop-do-while

do-while, including a do-while with break. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 2, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `countUp` | ✔ `do { v1 = v1 + 1; } while (v1 < p0);` |
| `stopEarly` | ✔ The do-while condition preserves the original early-break selection. |
| `nested` | ✔ Both loops are do-while with the intended values. |

### Findings

- All three methods recover do-while; graph<Unstructured> and empty repeat notation disappear. Recorded forms: `do { … } while (cond)`, `[repeat: ]`.
- The remaining differences are explicit increment assignments, qualified state access and nop. Recorded forms: `v1 = (v1 + 1)`, `index++`, `LoopDoWhile.state`, `nop;`.
- nested still prints a nop after its inner loop for an empty block. Recorded forms: `nop;`.

### Ranked fixes

Improve increment/compound-assignment presentation, empty-block nop handling and parameter names. Recorded forms: `++`, `+=`, `nop;`.

## Rounds

- R1 2026-11-08: All three methods used while/test/graph notation rather than a do-while form. Recorded forms: `while (test { graph<Unstructured> entries [L1] … yield … }) { }`.
- R2 2026-11-08: All three methods recovered do-while; internal notation disappeared, leaving increment assignments and nop. Recorded forms: `v1 = (v1 + 1)`, `nop;`.
