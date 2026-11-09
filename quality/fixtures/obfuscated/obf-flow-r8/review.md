# Fixture: obf-flow-r8

## Current verdict — whole-operation follow-up

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


R8-obfuscated enum state machine (switch, loop, try/finally, ternary). Built from the same Java reference as its source fixture with an
obfuscator, so the decompiled bodies can be compared against a known-good
reference.

- Provenance: authored for this repository. No third-party code or data; the
  obfuscated DEX is generated from our own source.
- Toolchain: R8, or ProGuard when the rules file starts with `# obfuscator:
  proguard`; `-dontshrink -dontoptimize` unless the rules say otherwise.
  `quality/build.sh` runs whichever tool the fixture's own rules file names.
- `mapping.txt` records original and obfuscated identities. Pair methods by
  original owner, name and full prototype; reused short names alone are ambiguous.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — iteration-tail review

Mapping and changed Lb;->b([I)I were reread against the identical Java explanation.
MIN_VALUE skips advance/count but still updates the index. DONE, a.c, exits via
L8 before update. The original bound, this.a(v3), count-after-success and normal/
exceptional this.b write-back ordering remain intact. Other methods and short
member identities are unchanged.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 2

LLM reread of the Java explanation, delivered original facts, current text and
mapping for `Lb;->b([I)I` (original ComplexFlow.run). The candidate is based on
`ebbc003`. Pairing uses owner and full prototype, not the overloaded short name.

The changed try includes only nonthrowing counter prefixes. A MIN_VALUE element
goes to the index update without calling a(I)La; or increasing the applied count.
Enum DONE is a.c; reaching it exits to the normal cleanup. Other inputs call
a(I)La; with the original loaded element and count only after success. Both
cleanup routes write this.b + applied once, with the handler rethrowing its
captured object. A cleanup failure is not caught by the same cleanup handler.
Null input can fail only after applied is initialized to zero. No original
short member identity or available body was dropped.

Other method text is unchanged. Existing enum/ternary/naming observations remain,
including a.c denoting DONE and this.b denoting steps; generated local shadowing
does not authorize treating a short name as a different member. This is manual
path evidence under the source's ordinary helper/VM assumptions, not executed
behavioral acceptance.

## Prior verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `advance` | ✔ The START ternary, RUNNING nested If and empty else preserve the source selection. |
| `run` | ✔ The loop, continue/break and finally write-back agree with complex-flow.run. |
| `summarize` | ✔ The ternary expands into locals and If with equivalent selection. |

### Findings

- After R8 renaming, P7 selects when rather than switch, with the original numeric decision arms. Recorded forms: `when (v3) { 0 -> {…} 1 -> {…} else -> { } }`.
- MemberClasses remains; the field and its type share the short spelling a. Recorded forms: `@MemberClasses(value = {class a}) system`, `private a a;`.
- The when presentation avoids the early default label and gotos seen in the un-obfuscated view. Recorded forms: `default:`.

### Ranked fixes

Review when presentation for Java input; use the appropriate P7 decision form without changing original selections.

## Rounds

- R1 2026-11-08: After R8 renaming, P7 selects when rather than switch, with the original numeric decision arms. Recorded forms: `when (v3) { 0 -> {…} 1 -> {…} else -> { } }`.
