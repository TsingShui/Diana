# Fixture: branch-if-inverted

Conditional branches with empty arms, side-effecting calls, copy-bearing arms,
floating-point comparisons, exception protection, nested conditions and a loop
exit. `Inverted.java` is the reference and build input; `classes.dex` is generated
with the shared `javac --release 8 -g` and `d8 --min-api 24` pipeline.

- Provenance: authored for this repository; no third-party source or input.
- Preparation observed: OpenJDK javac 21.0.12.1 and Android build-tools 37.0.0 D8.
- Regenerate with `quality/build.sh`; never edit the DEX by hand.
- Selector: `Lfixture/Inverted;`
- Current measurement from `python3 quality/score.py --binary target/debug/diana --slug branch-if-inverted`: structure 77.1%, naming 74.7% over 13 scored methods.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---:|---:|---|
| `invertedVoid` | 91.7% | 100% | added braces and qualified call |
| `invertedNull` | 92.6% | 80% | added braces and qualified call |
| `invertedEmptyBlock` | 91.7% | 100% | added braces and qualified call |
| `invertedProbe` | 90.0% | 75% | qualified call and temporary name |
| `invertedFloat` | 71.6% | 22% | explicit `compare<CmpgFloat>` |
| `plain` | 83.5% | 67% | initializer becomes branch copy |
| `effectPredicate` | 83.5% | 60% | initializer becomes branch copy |
| `nullCheck` | 50.0% | 75% | conditional expression becomes branches |
| `twoArms` | 68.5% | 67% | conditional expression becomes branches |
| `nested` | 89.2% | 100% | short circuit becomes nested If |
| `floatCmp` | 46.2% | 100% | explicit NaN tests, compare and gotos |
| `guardedCall` | 54.4% | 75% | explicit exception chain and handler |
| `loopBreak` | 90.0% | 50% | generated local declaration |

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

- R1 2026-11-08: structure 77.1%, naming 74.7%; eight inverted controls inspected, zero-length true-arm anchors observed.
