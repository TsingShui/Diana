# Fixture: kotlin-suspend-state

suspend functions and the continuation state machine. The Kotlin source is the semantic reference for the LLM review below.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`
  (see `quality/build.sh` output for the versions in use).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Case;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `Case` | ? Not judged. |
| `load` | ✔ Negative values throw IllegalStateException; other inputs return twice the value. |
| `fetch` | ? Not reviewed line by line for values; label selection, COROUTINE_SUSPENDED and I$0 state storage remain visible. |
| `guarded` | ✔ Try/catch and throwOnFailure ordering agree. |
| `total` | ? Not reviewed line by line for suspension inside the loop. |

### Findings

- The suspend modifier is absent; fetch has the lowered Object/Continuation signature. Recorded forms: `Object fetch(int, Continuation<? super Integer>)`.
- The state machine exposes label switching, goto L6/L7, getCOROUTINE_SUSPENDED and throwOnFailure. Recorded forms: `switch (label) { case 0: … case 1: … }`, `IntrinsicsKt.getCOROUTINE_SUSPENDED()`, `ResultKt.throwOnFailure`.
- The internal reference v29 declaration is visible instead of a Java/Kotlin source type. Recorded forms: `reference v29;`.
- Continuation fields I$0/I$1/label are read and written explicitly.

### Ranked fixes

Improve suspend/state-machine, reference-type and continuation-field presentation without removing original state facts.

## Rounds

- R1 2026-11-08: The suspend modifier is absent; fetch has the lowered Object/Continuation signature. Recorded forms: `Object fetch(int, Continuation<? super Integer>)`.
