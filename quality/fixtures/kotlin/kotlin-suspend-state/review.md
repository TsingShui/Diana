# Fixture: kotlin-suspend-state

## Current verdict — explicit raw selector destination

Compared the same prepared DEX on7354242 and this candidate. Only fetch's raw
default gains an explicit goto to its original invalid-state throw label.
The selector's other labels, resume fields/copies, suspended marker, load calls,
state writes and returns are unchanged. An unmatched original label now visibly
reaches that throw rather than an empty default preceding case0's goto.
This is original-dispatch correspondence, not a new coroutine analysis or a
claim that all suspension/resume paths have been independently validated.

The [fresh raw-selector challenge](../../exceptions/protected-loop-exits/review.md)
covers the statement-successor versus label-table distinction. P6 keeps raw
selector targets; P7 preserves supplied gotos instead of treating the owning
Block's next statement as the next case. Original P2/P3/P4/P5 facts agree.
The bounded review covers this changed route and reuses prior reading elsewhere;
no behavioral executor, Java recompilation or device run occurred.

## Prior verdict — terminal guard follow-up

Read suspended-marker returns and resume copies. load and label writes still
precede testing the marker; a suspended result returns before the original Phi
assignments. Non-suspended paths execute those same copies before
conversion/resume work. No coroutine protocol or new state is inferred.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — whole-operation follow-up

Compared the same prepared DEX with completed-eleven-repair bda9a49. Only
derived label/control coordinates change. Development-only correspondence found
identical printed words, original L/H program points and identical printed
lexical-scope bindings across every method publication; declaration-only members
are exact. Original P2/P3/P4/P5 facts agree. The unchanged semantic path review
below is reused, not replaced by a naming score or a claim of behavioral
execution.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


suspend functions and the continuation state machine. The Kotlin source is the semantic reference for the LLM review below.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`
  (see `quality/build.sh` output for the versions in use).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Case;`

## Prior verdict — round 1, 2026-11-08

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
