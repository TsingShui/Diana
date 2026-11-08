# Fixture: branch-switch-enum

switch over an enum, including a default arm. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `apply` | ✔ Cases 0/1/2 correspond to FAST/SAFE/LEGACY. |

### Findings

- apply switches directly on the ordinal; the intermediate switch-map array is eliminated. Recorded forms: `int v = p0.ordinal()`, `$SwitchMap`.
- Temporaries, default placement and case order differ from source presentation. Recorded forms: `default:`.
- The recovered values agree.

### Ranked fixes

Improve enum-switch presentation and parameter names without changing case/default behavior. Recorded forms: `switch (mode)`.

## Rounds

- R1 2026-11-08: apply switches directly on the ordinal; the intermediate switch-map array is eliminated. Recorded forms: `int v = p0.ordinal()`, `$SwitchMap`.
