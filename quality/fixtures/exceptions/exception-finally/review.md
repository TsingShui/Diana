# Fixture: exception-finally

finally on the normal path and on the throwing path. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `normalPath` | ✔ Finally is folded into the normal path with equivalent behavior. |
| `throwingPath` | ✔ The recorded output recovers a finally block. |

### Findings

- Finally expands into a normal-path copy plus catch-all cleanup/rethrow. Recorded forms: `finally`, `catch (all)`.
- throwingPath moves return acc into the normal branch; the finally field write appears once on each path.
- The state write remains on both normal and exceptional paths.

### Ranked fixes

Improve natural finally presentation, local initialization and parameter names. Recorded forms: `catch (all)`, `int acc = 0;`.

## Rounds

- R1 2026-11-08: Finally expands into a normal-path copy plus catch-all cleanup/rethrow. Recorded forms: `finally`, `catch (all)`.
