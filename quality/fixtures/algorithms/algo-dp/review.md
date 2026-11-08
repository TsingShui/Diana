# Fixture: algo-dp

edit distance, longest increasing subsequence and knapsack. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 2, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `editDistance` | ✔ Boundary initialization, nested Math.min and diagonal recurrence agree; int[][] declarations are restored; 64 executed comparisons agreed. |
| `longestIncreasing` | ✔ The nested loops and best[i] = best[j] + 1 agree. |
| `knapsack` | ✔ Reverse capacity traversal and candidate comparison agree. |

### Findings

- `new int[m][n]` becomes `Array.newInstance(Integer.TYPE, new int[]{m,n})` plus an Object local; most reads have `/* Object load */` and lack casts.
- Intermediate locals and loop spelling account for the remaining readability differences.
- The nesting order of `Math.min(v32, v35)` and `Math.min(v29, v37)` agrees.

### Ranked fixes

Recover array construction and loop presentation; reduce intermediate array-read locals. Recorded forms: `new int[m][n]`.

## Rounds

- R2 2026-11-08: Two-dimensional array declarations were restored to `int[][]`; the three methods agreed in 1646 executed comparisons.
- R1 2026-11-08: `new int[m][n]` becomes `Array.newInstance(Integer.TYPE, new int[]{m,n})` plus an Object local; most reads have `/* Object load */` and lack casts.
