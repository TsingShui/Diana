# Fixture: complex-inner

anonymous class, member inner, static nested and a generic bag. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

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
