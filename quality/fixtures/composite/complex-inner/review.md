# Fixture: complex-inner

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

Read the changed inner-class accumulation loop. Outer/inner receiver field
accesses and body additions retain their order; the scalar update remains after
a completed iteration. Constructors, enclosing-instance identities and other
members are preserved.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


anonymous class, member inner, static nested and a generic bag. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `adding` | ✔ Captures appear as constructor arguments to ComplexInner$1(this, p0). |
| `runVisitor` | ✔ Both visit calls retain their order. |
| `runAccumulator` | ✔ Accumulation agrees. |
| `runBag` | ✔ Generic arguments are absent from the declaration, but the retrieved value agrees. |

### Findings

- Anonymous-class capture is represented by ComplexInner$1(this, p0); the class name remains synthetic.
- Bag<String> declarations/construction are rendered with raw ComplexInner$Bag types. Recorded forms: `Bag<String> bag = new Bag<>()`, `ComplexInner$Bag bag = new ComplexInner$Bag()`.
- Other methods differ mainly through intermediate locals and loop presentation. Recorded forms: `runVisitor`.

### Ranked fixes

Improve generic arguments and separately review capture fields and the anonymous run implementation.

## Rounds

- R1 2026-11-08: Anonymous-class capture is represented by ComplexInner$1(this, p0); the class name remains synthetic.
