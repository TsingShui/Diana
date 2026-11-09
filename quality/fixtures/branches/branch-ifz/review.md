# Fixture: branch-ifz

## Current verdict — terminal guard follow-up

Read the zero-condition return and state3 path. The write remains false-only
after the early return, with the original result and no extra evaluation.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


single-register branches. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 1, 2026-11-08

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
