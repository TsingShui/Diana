# Fixture: branch-if2

two-register branches. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

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
