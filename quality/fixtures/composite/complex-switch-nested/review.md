# Fixture: complex-switch-nested

## Current verdict — raw default and protected loop exits

Compared the same prepared DEX on7354242 and this candidate. Default still goes
to its one original L8, now a private exit inside the while that breaks that
loop. It skips state writes, the applied increment and outer update before
normal cleanup. Case2 explicitly goes to L9, writes state2 and preserves applied
while advancing index. Case0 goes to L23, writes state1 and adds one. Case1
retains its original inner sum and outer-continue routes. Negative remainders
still select the original default. Before-effect snapshots retain the current
outer/inner accumulator and the handler's receive remains after capture.

An intermediate draft made default empty before case0 and was rejected: that
would select case0 work. P6 explicit raw-selector routes and P7's selector-phase
collapse guard now retain the exact gotos. A Sequence successor is not a raw
case-label successor. Read the complete changed method and original Java; all
original P2/P3/P4/P5 facts and copy obligations agree. The
[fresh cyclic-selector challenge](../../exceptions/protected-loop-exits/review.md)
retains this trigger under protection with a private default and real case cycle.
No behavioral executor, Java recompilation or device run is claimed.

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


switch inside a loop inside try, with labelled break and continue. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 2

LLM reread of Java, original delivered control/storage facts and current text for
`Lfixture/ComplexSwitchNested;->run([I)I`, candidate based on `ebbc003`.
The old unconfirmed labelled-exit judgment below is superseded for these paths;
no behavioral scenario executor or Java recompilation ran.

Case 0 writes state=1, then adds one to applied and advances the outer index.
Case 2 writes state=2 and goes directly to L19, preserving applied and skipping
that addition. Case 1 starts inner=0: a matching remainder goes through L13 to
L19 with the current inner sum and skips applied++; nonmatching iterations add
inner and increment it. Finishing inner=3 uses L16/L18 to add the outer count.
The default goes to L5 after the outer loop and before normal cleanup. Thus
labelled continue/break retain their actual destinations rather than becoming
a switch-local break. Negative remainders keep the same original comparisons.

The normal cleanup adds the final applied count to state. Every throwing array
or other protected access snapshots the current count, including the inner
v15 state rather than the older outer v19. The capture restores that count,
updates state once and rethrows the same object. Cleanup is outside its own
protection, so its failure does not cause another update. Null input saves zero
before its length read. The new try includes only safe control prefixes.

Snapshots and gotos still obstruct reading. The judgment is direct path comparison,
not machine behavioral PASS; original facts and availability agree in inspection.

## Prior verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `run` | ? Unconfirmed: jumps and throw-site snapshots obscure the labelled continue/break targets in the recorded text review. |

### Findings

- The outer label becomes goto L13/L16/L17/L19; the recorded text review did not establish the labelled exit targets. Recorded forms: `outer:`, `continue outer`, `break outer`.
- Repeated after_operands_before_effect snapshots make control flow difficult to read. Recorded forms: `[after_operands_before_effect {v22 = v19;}]`.
- The inner loop contains an empty If. Recorded forms: `if (v7 != v5) { }`.

### Ranked fixes

Improve labelled exits, necessary snapshot presentation and empty-branch handling.

## Rounds

- R1 2026-11-08: The outer label becomes goto L13/L16/L17/L19; the recorded text review did not establish the labelled exit targets. Recorded forms: `outer:`, `continue outer`, `break outer`.
