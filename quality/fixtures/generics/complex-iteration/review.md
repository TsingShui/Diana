# Fixture: complex-iteration

## Current verdict — whole-operation follow-up

Read changed array/collection iteration paths. Loads, iterator calls, casts and
accumulation preserve their order; supplied scalar tails remain at their
original repetition points. Empty-input exits and failures skip the same
updates; no iterator protocol or external hierarchy is inferred.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


iterator loops, enhanced for, map entries and removal via iterator. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — iteration-tail review

Java and final text were reread for
Lfixture/ComplexIteration;->sumNested([[I)I. Its outer captured-length traversal
uses a for update after the full inner row accumulation. A null row or failed
load does not advance the outer counter. Sum additions and inner updates retain
their original order. Iterator/removal/map methods are unchanged.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `add` | ✔ |
| `totalWeight` | ✔ entrySet iteration and summation agree. |
| `dropShort` | ✔ iterator.remove() is retained. |
| `heavy` | ? Not reviewed. |
| `sumNested` | ✔ Indexed loops preserve the values of nested enhanced iteration. |

### Findings

- Map.Entry enhanced iteration expands into Iterator, Object and an explicit entry cast. Recorded forms: `for (Map.Entry<…> entry : counts.entrySet())`, `((Map$Entry) v5).getValue()`.
- String enhanced iteration expands into Iterator and an explicit String cast. Recorded forms: `for (String name : names)`, `((String) v5).length()`.
- The counts.get(name) and null-selection paths in heavy were not confirmed.

### Ranked fixes

Improve enhanced iteration, Map.Entry and generic-argument presentation without losing element checks. Recorded forms: `Object`.

## Rounds

- R1 2026-11-08: Map.Entry enhanced iteration expands into Iterator, Object and an explicit entry cast. Recorded forms: `for (Map.Entry<…> entry : counts.entrySet())`, `((Map$Entry) v5).getValue()`.
