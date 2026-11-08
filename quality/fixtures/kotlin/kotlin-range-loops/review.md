# Fixture: kotlin-range-loops

Kotlin loop forms: `until`, `downTo`, `step`, `repeat`, `forEach` and labelled
`break@`/`continue@`. The Kotlin source is the semantic reference for the LLM review below.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Case;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `until` | ? Not reviewed line by line. |
| `downTo` | ✔ The decreasing integer while loop agrees with downTo. |
| `stepped` | ? Not reviewed line by line; step uses ProgressionUtilKt. |
| `repeated` | ? Not reviewed line by line. |
| `each` | ✔ List.forEach expands into an iterator with the intended values. |
| `labelled` | ? Not reviewed line by line for the labelled break/continue paths. |

### Findings

- Range loops expand into integer while loops; until/downTo/step/range behavior must be inferred from values. Recorded forms: `..`.
- repeat and forEach expand into inline iteration or counter loops. Recorded forms: `repeat(n) { }`, `forEach { }`.
- The complete SourceDebugExtension SMAP appears in the class text. Recorded forms: `@SourceDebugExtension(value = "SMAP\nCase.kt\nKotlin\n*S Kotlin…")`.
- The complete Metadata annotation is also printed; labelled break requires following goto targets. Recorded forms: `@Metadata(d1 = …, d2 = …) runtime`, `break@outer`.

### Ranked fixes

Improve range/repeat/forEach presentation and the placement of SMAP/Metadata facts. Recorded forms: `@Metadata`.

## Rounds

- R1 2026-11-08: ranges lower to plain integer loops and SMAP enters the class body.
