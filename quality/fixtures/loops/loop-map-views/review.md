# Fixture: loop-map-views

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

Read changed traversal/accumulation paths. Original receiver accesses, iterator
calls, selected key/value operations and body order remain. Any supplied scalar
tail follows the same successful body; no call is moved into a header update or
removed by an empty-body assumption.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


map views and a custom Iterable behind an enhanced for. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopMapViews;`

## Prior verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `put` | ✔ |
| `sumValues` | ✔ Iteration and summation over counts.values() agree. |
| `keys` | ✔ keySet iteration agrees. |
| `walk` | ✔ Enhanced iteration over the custom Iterable agrees. |
| `drainValues` | ? Not reviewed line by line for iterator.remove(). |

### Findings

- Iteration over counts.values() expands into Iterator, Object and Integer cast/unboxing.
- Custom Iterable iteration also expands into an iterator; the Iterable type appears in a cast.
- The explicit Iterator for loop loses its source for form.

### Ranked fixes

Improve enhanced/Map-view iteration and reduce Object intermediates without losing checks.

## Rounds

- R1 2026-11-08: Iteration over counts.values() expands into Iterator, Object and Integer cast/unboxing.
