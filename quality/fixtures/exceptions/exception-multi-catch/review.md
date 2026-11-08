# Fixture: exception-multi-catch

one try, one multi-catch clause, one caught rethrow. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `parse` | ✔ |
| `rethrow` | ✔ Both catches converge before throwing the new IllegalStateException(error). |

### Findings

- Multi-catch becomes two catch clauses joining through goto rather than a compound catch label. Recorded forms: `|`.
- Extra local declarations and an exception-chain line precede the body. Recorded forms: `int v; Type v;`, `exception_chain c0`.
- Both catches update state before the original return or rethrow.

### Ranked fixes

Improve compound-catch, declaration and exception-route presentation; preserve clause order and parameter identity. Recorded forms: `catch (A | B e)`, `exception_chain`.

## Rounds

- R1 2026-11-08: Multi-catch becomes two catch clauses joining through goto rather than a compound catch label. Recorded forms: `|`.
