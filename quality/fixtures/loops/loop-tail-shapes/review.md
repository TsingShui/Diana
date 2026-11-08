# Fixture: loop-tail-shapes

Every loop shape whose test can sit after the body, next to the head-test shapes
that must stay `while`. A tail test compiles to one block whose branch returns to
its own start; the empty-body variants are the ones that most easily collapse into
each other, so they are the regression for the P6 tail-test classification.

The Java source is the reference and the expectation; the DEX is generated from it
with `javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and
committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 2, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `emptyWhileSideEffect` | ✔ The condition is evaluated before each entry to the empty while body. |
| `emptyDoSideEffect` | ✔ Each do iteration calls next() once. |
| `emptyWhile` | ✔ The folded condition preserves the original divergent loop behavior. |
| `plainWhile` | ✔ `while (v1 < p0) { v1++; }` |
| `plainDo` | ✔ `do { v1++; } while (v1 < p0);` |
| `emptyForBody` | ✔ The while/update form retains the values, though the for header is absent. |
| `initBeforeDo` | ✔ Initialization remains outside the correctly recovered do-while. |
| `compoundDo` | ✔ The nested If/break preserves first() && second() short-circuiting. |
| `breakInsideDo` | ✔ The explicit while/early-break form preserves the values. |
| `breakInsideWhile` | ✔ `if (v1 != 3) v1++; else break;` |
| `continueInsideDo` | ✔ Continue reaches the condition, preserving the original selection. |
| `infiniteWithBreak` | ✔ The loop agrees, with an additional nop. |
| `scanLength` | ✔ Length reads and the index update preserve the source behavior. |
| `scanDo` | ✔ The split array-test/break retains short-circuit order. |

### Findings

- Tail-test classification works both ways: plainDo/initBeforeDo/continueInsideDo recover do-while;
  emptyWhile/emptyWhileSideEffect and emptyForBody retain while.
- emptyDoSideEffect and compoundDo move condition evaluation into the body; behavior agrees but empty-do syntax is absent. Recorded forms: `do { }`.
- Predicate preludes become while(true) plus a conditional break, not a compact while condition. Recorded forms: `while (true) { …; if (exit) break; }`, `while (cond)`.
- infiniteWithBreak adds a nop; breakInsideDo adds an L5 label. Recorded forms: `L5:`.

### Ranked fixes

Improve predicate-prelude composition into assignment expressions, replacing while(true)/break Recorded forms: `while ((v1 = p0.length) > v2)`, `while (true)`.
with source-like while conditions where supported; then improve empty-do, nop and parameter presentation. Recorded forms: `while (cond)`, `do { }`, `nop;`.

## Rounds
- R3 2026-11-08: Empty branches are removed; an empty true arm becomes an inverted single-arm If without moving predicate evaluation. Recorded forms: `else { }`, `if`.

- R1 2026-11-08: Both tail-test directions agree; explicit test blocks and nop remain. Recorded forms: `test { }`, `nop;`.
- R2 2026-11-08: Test/yield while notation became an explicit while(true) body and conditional break. Recorded forms: `while (test { … yield … })`, `while (true) { …; if (exit) break; }`.
