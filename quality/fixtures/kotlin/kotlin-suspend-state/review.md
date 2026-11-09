# Fixture: kotlin-suspend-state

## Current total loop review

This LLM review compares `Case.kt::total`, its prepared DEX, and the suspend-loop
candidate against baseline `73542421d671be2fc169b72c7233aa0b5b955fd5`.
The reviewed engine candidate is `4c6a3d1`; the integration recheck uses `8785d2f`.
The signature retains Object, the original int parameter, and
Continuation<? super Integer>, and adds `suspend`. The body has initial sum/index,
a head-tested while, one load call, the original Number conversion and additions,
and the boxed return. The old whole loop/reentry graph becomes ABI attachments
plus the source order, rather than disappearing with its original facts.

For a nonpositive limit, the initial predicate returns boxed zero without load.
For each nonnegative index below limit, load's original helper rejects negative
arguments only, then returns boxed index * 2. The returned int contributes to sum
before index increases. For limit 4 the loads receive 0, 1, 2, 3 and the result is
12; limit 1 returns 0. Integer overflow keeps the original DEX integer operations.

The resume corridor restores I$0 (limit), I$2 (index), and I$3 (captured sum), then
checks failure before the Number cast or arithmetic. I$1 is also read, as in the
input, although that read does not supply the eventual sum update. Both result
routes reach the same original sum/index update. The three forwarded copies
read the call result, old sum and old index before their sources change; no
update or call is duplicated. The original state stores precede load. A marker
result returns before the tail, and a resumed failure skips all tail work.
Frame selection, high-bit label adjustment, invalid-state throw/message and ABI
targets remain visible, with original source mappings. The view infers suspension
from the input protocol; the supplied load helper itself completes immediately.

No contradiction was found in these reviewed paths. The
[captured-effect challenge](../suspend-loop-captured-effects/review.md) adds another
mutable value, a captured expression, helper effects, and mismatching boundaries.
Other original fixture members have unchanged output in the fixture comparison.
Not run: a final-plan behavioral executor or actual coroutine scheduling.

## Current verdict — terminal guard follow-up

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
