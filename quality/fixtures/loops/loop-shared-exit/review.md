# Fixture: loop-shared-exit

nested loops leaving through a flag or a labelled break. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopSharedExit;`

## Current verdict — round 2, 2026-11-08

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
