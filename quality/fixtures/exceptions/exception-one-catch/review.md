# Fixture: exception-one-catch

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


one try, one catch. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 1, 2026-11-08

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
