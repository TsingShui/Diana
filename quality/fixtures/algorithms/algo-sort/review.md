# Fixture: algo-sort

insertion, quick and merge sort. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 2, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `insertion` | ✔ The inner while plus an inverted comparison/break preserves the original conjunction. |
| `quick` | ✔ The public entry invokes the private overload; partition and prefix-recursion order agree. |
| `merge` | ✔ Three-way merge, left selection on <=, tail copies and break agree. |

### Findings

- The partition loop no longer prints `[repeat: ...]`: `while (v16 <= v18)` is direct, with v17/v19 merged into v16/v18. Recorded forms: `[repeat: …]`.
- Remaining differences are intermediate copies, index temporaries and the missing source-style for loop, not the partition behavior. Recorded forms: `int v6 = p1;`, `v16 = v6`.
- The private quick overload and public entry have the same name; distinguish them by prototype.

### Independent execution

`quick` agreed in 896 executed comparisons over array contents and two indices: returns, in-place mutations and exception classes matched. A deliberately broken swap was detected. This is bounded evidence, not proof for every input.

### Ranked fixes

Improve i++/+= and for presentation, and distinguish the private overload by its declaration.

## Rounds

- R1 2026-11-08: The partition loop expressed its update with `[repeat: ...]`, requiring the reader to interpret phase notation. Recorded forms: `quick`, `[repeat: …]`.
- R2 2026-11-08: Phase annotations disappeared; remaining differences concentrate in locals and for-loop presentation.
