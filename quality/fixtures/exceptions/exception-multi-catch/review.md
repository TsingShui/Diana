# Fixture: exception-multi-catch

## Natural catch display

Both subjects now print only their `try/catch` layout. The two-clause `parse` and
the three-clause `multiCapture` keep their original clause order and original
`goto` targets, including the `capture_exception` statements its printed clauses
do not bind in a header. No catch body, order or result changed. Text and JSON publish together; owned sites, chains, completions,
snapshots and receives remain. Renewed LLM reading and product equality, not
executed behavior.

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `NumberFormatException v4`, `NullPointerException v3`. The original captured object is bound once at exceptional
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
