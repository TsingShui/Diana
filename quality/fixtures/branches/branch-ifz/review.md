# Fixture: branch-ifz

single-register branches. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `whenZero` | ✔ Branch polarity is preserved. |
| `whenNonZero` | ✔ Branch polarity is preserved. |
| `bothPaths` | ✔ Branch polarity is preserved. |
| `nullCheck` | ✔ Branch polarity is preserved. |

### Findings

- Zero, nonzero and null condition forms retain their original polarity. Recorded forms: `if (p0 == 0)`, `if (p0 != 0)`, `if (input == null)`.
- Generated parameter names and qualified state access remain, rather than a conditional-structure difference. Recorded forms: `p0`, `BranchIfz.state`.

### Ranked fixes

Recover available debug parameter names and reduce qualification where unambiguous.

## Rounds

- R1 2026-11-08: Zero, nonzero and null condition forms retain their original polarity. Recorded forms: `if (p0 == 0)`, `if (p0 != 0)`, `if (input == null)`.
