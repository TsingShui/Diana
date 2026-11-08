# Fixture: exception-one-catch

one try, one catch. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `catchReturns` | ✔ |
| `catchRethrows` | ✔ State becomes 2 before the original exception is rethrown. |
| `sideEffectsInTry` | ✔ Ordering agrees. |

### Findings

- Catch return, catch rethrow and protected effect order agree.
- Per-operation exn markers, an exception-chain line and a pre-try nop remain visible. Recorded forms: `[exn c0]`, `exception_chain`, `nop;`.

### Ranked fixes

Improve catch-variable and protection presentation while keeping original routes and facts available. Recorded forms: `[exn cN]`, `exception_chain`, `nop`, `catch (T e)`.

## Rounds

- R1 2026-11-08: Catch return, catch rethrow and protected effect order agree.
