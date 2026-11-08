# Fixture: complex-stream

stream pipelines, method references and Optional. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `evensDoubled` | ✔ Filter/map/collect order agrees; original lambda body identities remain visible. |
| `upper` | ✔ The two method references appear as an R8 lambda implementation and ComplexStream$0. |
| `sumOver` | ✔ |
| `firstPositive` | ✔ Optional.orElse(-1) agrees. |
| `fromArrays` | ✔ IntStream.concat/forEach agrees; Objects.requireNonNull is the D8 method-reference check. |

### Findings

- Original lambda body names and synthetic flags appear directly in the class body. Recorded forms: `lambda$…$N`, `$r8$lambda$…`, `dex_flags<synthetic>`.
- Each lambda object is constructed before filter/map/forEach; intermediate streams occupy named locals. Recorded forms: `new ComplexStream$N()`, `Stream v4/v6`.
- Collector creation, collect and the result cast are separate statements rather than one pipeline. Recorded forms: `Collector v7 = Collectors.toList(); Object v8 = v6.collect(v7); return ((List) v8);`.

### Ranked fixes

Improve lambda/pipeline and synthetic-member presentation without hiding original members or checks. Recorded forms: `values.stream().filter(…).map(…).collect(…)`, `dex_flags<>`.

## Rounds

- R1 2026-11-08: Original lambda body names and synthetic flags appear directly in the class body. Recorded forms: `lambda$…$N`, `$r8$lambda$…`, `dex_flags<synthetic>`.
