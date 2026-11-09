# Fixture: kotlin-range-loops

## Current verdict — whole-operation follow-up

Read the changed lowered range loop. Actual comparisons, body effects and scalar
tail retain their original order, including entry/exit values. No Kotlin
source-level stride, range type or progress fact is inferred from for
presentation.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


Kotlin loop forms: `until`, `downTo`, `step`, `repeat`, `forEach` and labelled
`break@`/`continue@`. The Kotlin source is the semantic reference for the LLM review below.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Case;`

## Prior verdict — iteration-tail review

Case.kt and final text were reread. Lfixture/Case;->labelled([[I)I now places the
outer array index in a for: negative exits both loops without updates, while
zero exits only the inner loop and performs the outer update, matching the
original continue@outer. stepped(I)I retains its one original progression-helper
call, nonnegative-last guard, add-before-endpoint-test and no +3 after reaching
the endpoint. No absent library declaration or helper behavior was inferred.
The other four methods are unchanged.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 1, 2026-11-08

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
