# Fixture: loop-do-while-sideeffect

## Current verdict — while/do-while architecture correction

Read every changed loop's entry test, ordinary body and original backedge. The
same body work precedes the now-visible update; an exiting test or abrupt effect
skips exactly the original remaining work. Initial values, loop nesting,
call/access/argument order and selected results remain. Some body-only
temporaries can now be declared at their existing assignments because no header
reads them. This changes no original value or effect.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Read the changed lowered loop schedule. Side-effecting predicate/body
evaluations stay at their original point and count; only the established pure
tail appears in the supplied update. The first iteration and its eventual test
do not gain a speculative evaluation.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


loop conditions that call something on every test. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopDoWhileSideEffect;`

## Prior verdict — round 3, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `consume` | ✔ The poll count and null test agree with the source. |
| `drain` | ✔ The empty do body disappears and part of the condition becomes an in-body break, with equivalent behavior. |
| `untilZero` | ✔ Each iteration reads values[index] once and increments once; bounds failures agree. |
| `retry` | ✔ Short-circuit evaluation is retained: attempts[index++] runs only after the first condition succeeds. |

### Findings

- Effectful loop tests appear at the start of a while(true) body followed by an inverted exit/break. Recorded forms: `while (true) { v1 = poll(); if (v1 == null) break; … }`.
- drain still expresses part of its do-while condition with an in-body If/else break.
- retry computes index-1 through an intermediate local before the array read.
- consume now places the Integer cast/unboxing at the read; explicit owner notation and the standalone cast disappear. Recorded forms: `((Integer) v1).intValue()`, `via`, `((Integer) v1);`.

### Ranked fixes

Improve effectful condition and do-while presentation without moving evaluation.

## Rounds
- R3 2026-11-08: Empty branches are removed; an empty true arm becomes an inverted single-arm If without moving predicate evaluation. Recorded forms: `else { }`, `if`.

- R1 2026-11-08: Effectful conditions appeared inside explicit test/yield blocks.
- R2 2026-11-08: Test/yield predicate notation became an inverted in-body condition and break. Recorded forms: `test { … yield … }`.
- R3 2026-11-08: Checks moved to the first post-copy read; standalone checks disappeared. The recorded class text compiled, and four methods agreed in 52 executed comparisons. Recorded forms: `via`.
