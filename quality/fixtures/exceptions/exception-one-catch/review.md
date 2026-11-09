# Fixture: exception-one-catch

## Natural catch display

The three single typed-catch subjects now print only their `try/catch`; duplicate
chain lines, inline markers and `H` labels are omitted. Division, field reads and
writes, the rethrow of the original `v2`, and `-1`/state results are unchanged.
Explicit protection and any nonmatching propagation keep their previous spelling.
P2–P6 source products are exact. Text/JSON publication changes together; the original sites, chains,
completions, snapshots and receives remain in the owned product. This is a
renewed LLM reading of the exact before/after text plus product equality, not
executed behavior.

## Prior verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `ArithmeticException v2`, `ArithmeticException v2`, `RuntimeException v4`. The original captured object is bound once at exceptional
entry, before the unchanged receiving/body actions. Its exact-state uses remain
inside that handler and have no capture-phase, Phi or copy dependency requiring
an outer assignment. Normal execution does not bind or run the handler.

After accounting for that binding and its removed private declaration/assignment,
the complete remaining output is exact. Original P1/P3/P4/P5 products agree.
Thus the earlier independently reasoned normal, caught, cleanup, rethrow and
secondary-failure paths below are reused: their calls, arguments, values,
priority, transfers and effect order did not change. The renewed LLM judgment
finds no contradictory capture lifetime or exception identity. Structural
comparison supports this reading rather than supplying behavioral proof.

The [fresh monitor/binding challenge](../../sync/monitor-surface-boundaries/review.md)
covers capture identity, receiving/value flow, nested failures and escaping
storage. Current fixture/model regressions are separate executed evidence;
this record does not claim VM/device execution for this input.

## Prior verdict — whole-operation follow-up

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
