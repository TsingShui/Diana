# Fixture: loop-shared-exit

## Current verdict — whole-operation follow-up

Compared the same prepared DEX with completed-eleven-repair bda9a49. Only
derived label/control coordinates change. Development-only correspondence found
identical printed words, original L/H program points and identical printed
lexical-scope bindings across every method publication; declaration-only members
are exact. Original P2/P3/P4/P5 facts agree. The unchanged semantic path review
below is reused, not replaced by a naming score or a claim of behavioral
execution.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


nested loops leaving through a flag or a labelled break. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopSharedExit;`

## Prior verdict — iteration-tail review

Java and final text were reread for flagExit([[II)I, labelledExit([[II)I and
sharedTail([I)I. flagExit sets found/result then leaves the inner loop without a
column update; its outer row update still precedes the next bound/found tests.
labelledExit's return or outer break skips both pending updates. sharedTail
retains distinct index initializations, repeated array reads and total across
both loops; either skip path still advances its own index once.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 2, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `flagExit` | ✔ The hit action runs once in the inner branch before break; normal exhaustion skips it and retains the outer update. |
| `labelledExit` | ✔ A hit returns directly; the negative branch breaks R29, skips both updates and returns -1. |
| `sharedTail` | ✔ Both continue routes keep their index updates while selecting opposite signs on the two passes. |

### Findings

- The found condition moves into the loop body and its flag uses integer storage.
- The private hit action now stays in its branch and exits via break; the normal-exit planned-continuation goto disappears.
- The negative path prints a labelled break; the four prior gotos in these two methods disappear.
- This round changes flagExit and labelledExit only; sharedTail text is unchanged.

### Ranked fixes

Improve Boolean flag, array-read composition and redundant-local presentation.

## Rounds
- R3 2026-11-08: Empty branches are removed; an empty true arm becomes an inverted single-arm If without moving predicate evaluation. Recorded forms: `else { }`, `if`.

- R1 2026-11-08: The found condition moves into the loop body and its flag uses integer storage.
- R2 2026-11-08: Private nested exits remain inside the loop; four gotos become ordinary/labelled breaks; methods were reread semantically.
