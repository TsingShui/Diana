# Fixture: loop-map-views

map views and a custom Iterable behind an enhanced for. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopMapViews;`

## Current verdict — round 1, 2026-11-08

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
