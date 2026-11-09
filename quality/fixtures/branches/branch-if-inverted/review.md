# Fixture: branch-if-inverted

## Current verdict — terminal-follow conjunction, 2026-11-09

The `nested` subject now uses one ordered condition: `p0 == 0 || p1 == 0` selects
the shared `return 0` arm, while the other arm keeps `observe(p1); return 1;`.
The previous nested-If shape is retained below as historical evidence. This
focused reread covers the changed subject; the other member judgments are not
replaced or promoted to new execution results.

## Current verdict — terminal guard follow-up

Read guardedCall: zero returns before observe, nonzero keeps the same
try/RuntimeException recovery and1/-1 results. The try stays in the remaining
body, not moved outside its original enclosing protection.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — whole-operation follow-up

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


Conditional branches with empty arms, side-effecting calls, copy-bearing arms,
floating-point comparisons, exception protection, nested conditions and a loop
exit. `Inverted.java` is the reference and build input; `classes.dex` is generated
with the shared `javac --release 8 -g` and `d8 --min-api 24` pipeline.

- Provenance: authored for this repository; no third-party source or input.
- Preparation observed: OpenJDK javac 21.0.12.1 and Android build-tools 37.0.0 D8.
- Regenerate with `quality/build.sh`; never edit the DEX by hand.
- Selector: `Lfixture/Inverted;`

## Prior verdict — round 1, 2026-11-08

### Per method

| method | Observed shape |
| --- | --- |
| `invertedVoid` | added braces and qualified call |
| `invertedNull` | added braces and qualified call |
| `invertedEmptyBlock` | added braces and qualified call |
| `invertedProbe` | qualified call and temporary name |
| `invertedFloat` | explicit `compare<CmpgFloat>` |
| `plain` | initializer becomes branch copy |
| `effectPredicate` | initializer becomes branch copy |
| `nullCheck` | conditional expression becomes branches |
| `twoArms` | conditional expression becomes branches |
| `nested` | short circuit becomes nested If |
| `floatCmp` | explicit NaN tests, compare and gotos |
| `guardedCall` | explicit exception chain and handler |
| `loopBreak` | generated local declaration |

### Semantics

Verdicts compare the reference and the current readable output. The native helper
declarations have no method body to assess; their original identities remain present.

| method | semantics |
|---|---|
| `invertedVoid` | ✔ nonzero invokes `observe(x)` once; zero does not |
| `invertedNull` | ✔ non-null invokes `observe(x)` once |
| `invertedEmptyBlock` | ✔ nonzero invokes `observe(x)` once |
| `invertedProbe` | ✔ calls `probe()` once before the test, then `observe` only for nonzero |
| `invertedFloat` | ~ retains `cmpg-float` and tests its result; NaN does not enter the branch |
| `plain` | ✔ branch assigns 0 or invokes `observe` then assigns 1 |
| `effectPredicate` | ✔ `probe` precedes branch and any `observe` call |
| `nullCheck` | ✔ null returns 0 and non-null returns 1 |
| `twoArms` | ✔ chooses 1 or 2 before `observe` and return |
| `nested` | ✔ tests `y` only when `x` is nonzero; calls once only on the double match |
| `floatCmp` | ~ original NaN tests and floating comparison remain explicit with gotos |
| `guardedCall` | ~ `observe` remains in the RuntimeException-protected branch; handler returns -1 |
| `loopBreak` | ✔ increments before the comparison; on match calls `observe` then exits |
| `observe` / `probe` | ? native declarations; no input body |

### Findings

- Current output contains eight syntax-owned `IfShape::Inverted` instances. Their
  omitted true-arm controls each have a zero-length P9 origin anchor.
- The readable text keeps the original calls and conditional selections; floating
  compare and exception handling still use explicit Diana notation.
- This review is a reading assessment, not a semantic regression executor.

### Ranked fixes

Retain source origins for omitted controls and their nonprinting transfers. Keep
branch copies, exception routes and float-comparison behavior at original positions.

## Rounds

- R1 2026-11-08: eight inverted controls inspected, zero-length true-arm anchors observed.
