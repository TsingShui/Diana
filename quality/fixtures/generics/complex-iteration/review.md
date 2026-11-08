# Fixture: complex-iteration

iterator loops, enhanced for, map entries and removal via iterator. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

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
