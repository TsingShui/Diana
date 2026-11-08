# Fixture: loop-for

classic for, with break and continue in the body. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `sumTo` | ✔ |
| `skipNegative` | ✔ Continue becomes an empty else with equivalent behavior. |
| `stopAt` | ✔ Break is retained. |

### Findings

- All three for loops expand into initialization followed by while rather than a source-style for header. Recorded forms: `for (`.
- Continue/break remain labels and gotos. Recorded forms: `continue`, `skipNegative`.
- Values and accumulation agree.

### Ranked fixes

Improve for headers, explicit break/continue and parameter naming. Recorded forms: `for (init; cond; update)`.

## Rounds
- R3 2026-11-08: Empty branches are removed; an empty true arm becomes an inverted single-arm If without moving predicate evaluation. Recorded forms: `else { }`, `if`.

- R1 2026-11-08: All three for loops expand into initialization followed by while rather than a source-style for header. Recorded forms: `for (`.
