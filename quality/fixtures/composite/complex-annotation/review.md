# Fixture: complex-annotation

runtime annotations with array, enum, class and default values. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ComplexAnnotation;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `read` | ✔ Method annotations are retained. |
| `write` | ✔ Only explicitly encoded values are shown; defaults are not expanded, matching the DEX facts. |

### Findings

- The enum annotation value repeats its type name: `@Marker(color = GREEN ComplexAnnotation$Color ComplexAnnotation$Color, name = ...)`. Recorded forms: `@Marker(color = GREEN ComplexAnnotation$Color ComplexAnnotation$Color, name = …)`.
- Annotation visibility appears as a trailing runtime word in the main text. Recorded forms: `@ComplexAnnotation$Marker(…) runtime`.
- The class and array annotation values agree; the nested annotation class is rendered separately. Recorded forms: `type = class String`, `numbers = {1, 2, 3}`.

### Ranked fixes

Fix repeated enum annotation type names and improve visibility presentation.

## Rounds

- R1 2026-11-08: The enum annotation value repeats its type name: `@Marker(color = GREEN ComplexAnnotation$Color ComplexAnnotation$Color, name = ...)`. Recorded forms: `@Marker(color = GREEN ComplexAnnotation$Color ComplexAnnotation$Color, name = …)`.
