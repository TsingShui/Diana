# Fixture: algo-matrix

matrix multiply, transpose and spiral traversal. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — iteration-tail review

Java and final text were reread. Only multiply(int[][],int[][]) and
transpose(int[][]) change their outer row placement. Dimension reads and
allocation still happen first, all inner products/accumulations or stores finish
before the row update, and a null/jagged/bounds failure skips that update.
The inner loops and spiral traversal are unchanged.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 2, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `multiply` | ✔ Declared int[][] with a cast at initialization; 64 executed comparisons agreed. |
| `transpose` | ✔ The same type handling; 8 executed comparisons agreed. |
| `spiral` | ✔ The four boundary loops and updates agree; 8 executed comparisons agreed. |

### Findings

- Creation still uses `Array.newInstance(Integer.TYPE, ...)`, but `int[][] v = (int[][]) ...`, `return v;` and `v[i]` have the intended types. The recorded class text compiled. Recorded forms: `Array.newInstance(Integer.TYPE, …)`, `int[][] v = (int[][]) …`.
- Executed comparisons covered multiply 64, transpose 8 and spiral 8: 80 comparisons, no mismatches.
- The remaining difference is presentation: nested loops expand into while loops and intermediate locals; `p1[0] /* Object load */` remains visible.

### Ranked fixes

Improve two-dimensional construction, direct array reads and nested-loop presentation. Recorded forms: `new int[m][n]`, `int[][]`.

## Rounds

- R2 2026-11-08: Two-dimensional declarations were restored to int[][]; checks moved to initialization; executed comparisons agreed for all three methods.
- R1 2026-11-08: Creation used `Array.newInstance(Integer.TYPE, new int[]{...})`; later reads needed casts or lost their static type, including `return v8;`. Recorded forms: `Array.newInstance(Integer.TYPE, new int[]{…})`.
