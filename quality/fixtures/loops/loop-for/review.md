# Fixture: loop-for

## Current verdict — while/do-while architecture correction

Read threshold/filtered-sum paths. Threshold success stores the original index
and breaks before its update; other paths advance once. Filtering skips addition
only, not the body counter tail. Initial tests and failed accesses retain their
order.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Read changed headers, initializers and body tails. Every for update is the
supplied original scalar self-store at its original repetition point; pre-test
assignments stay before the first test. Bounds, body effects, early exits and
returned totals retain the prior paths.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


classic for, with break and continue in the body. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — iteration-tail review

Java and final text were reread. skipNegative([I)I retains both original array
reads on its nonnegative path and only the first on a negative skip, followed by
the same index update. stopAt([II)I writes state before break and skips the
update on that break or a failed access/write. sumTo(I)I is unchanged. A for
placement is an equivalent recovered schedule, not an original-keyword claim.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `sumTo` | ✔ |
| `skipNegative` | ✔ Continue becomes an empty else with equivalent behavior. |
| `stopAt` | ✔ Break is retained. |

### Findings

- All three for loops expand into initialization followed by while rather than a source-style for header. Recorded forms: `for (`.
- Continue/break remain labels and gotos. Recorded forms: `continue`, `skipNegative`.
- Values and accumulation agree.

### Ranked fixes

Improve for headers, explicit break/continue and parameter naming. Recorded forms: `for (init; cond; update)`.

## Rounds
- R3 2026-11-08: Empty branches are removed; an empty true arm becomes an inverted single-arm If without moving predicate evaluation. Recorded forms: `else { }`, `if`.

- R1 2026-11-08: All three for loops expand into initialization followed by while rather than a source-style for header. Recorded forms: `for (`.
