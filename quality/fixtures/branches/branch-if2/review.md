# Fixture: branch-if2

## Current verdict — terminal guard follow-up

Read each comparison guard and state write. The return path skips the original
calls field assignment; the other path writes its original constant before
returning the same selected value. Condition polarity and long-comparison
semantics remain.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


two-register branches. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `lessThan` | ✔ Inverted but equivalent. |
| `greaterOrEqual` | ✔ |
| `equal` | ✔ |

### Findings

- All three comparisons agree, but are inverted/reordered; else becomes early return. Recorded forms: `<`, `>=`.
- Original left/right parameter names are not recovered.

### Ranked fixes

Improve source-like conditional presentation and parameter naming. Recorded forms: `if/else`.

## Rounds

- R1 2026-11-08: All three comparisons agree, but are inverted/reordered; else becomes early return. Recorded forms: `<`, `>=`.
