# Fixture: branch-if-inverted

Conditional branches with empty arms, side-effecting calls, copy-bearing arms,
floating-point comparisons, exception protection, nested conditions and a loop
exit. `Inverted.java` is the reference and build input; `classes.dex` is generated
with the shared `javac --release 8 -g` and `d8 --min-api 24` pipeline.

- Provenance: authored for this repository; no third-party source or input.
- Preparation observed: OpenJDK javac 21.0.12.1 and Android build-tools 37.0.0 D8.
- Regenerate with `quality/build.sh`; never edit the DEX by hand.
- Selector: `Lfixture/Inverted;`

## Current verdict — round 1, 2026-11-08

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
