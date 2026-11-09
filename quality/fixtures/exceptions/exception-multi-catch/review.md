# Fixture: exception-multi-catch

## Current verdict — whole-operation follow-up

Compared the same prepared DEX with completed-eleven-repair bda9a49. Only
derived label/control coordinates change. Development-only correspondence found
identical printed words, original L/H program points and identical printed
lexical-scope bindings across every method publication; declaration-only members
are exact. Original P2/P3/P4/P5 facts agree. The unchanged semantic path review
below is reused, not replaced by a naming score or a claim of behavioral
execution.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


one try, one multi-catch clause, one caught rethrow. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 1, 2026-11-08

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
