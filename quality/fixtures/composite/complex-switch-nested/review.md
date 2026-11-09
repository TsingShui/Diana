# Fixture: complex-switch-nested

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `all v12`. The original captured object is bound once at exceptional
entry, before the unchanged receiving/body actions. Its exact-state uses remain
inside that handler and have no capture-phase, Phi or copy dependency requiring
an outer assignment. Normal execution does not bind or run the handler.

After accounting for that binding and its removed private declaration/assignment,
the complete remaining output is exact. Original P1/P3/P4/P5 products agree.
Thus the earlier independently reasoned normal, caught, cleanup, rethrow and
secondary-failure paths below are reused: their calls, arguments, values,
priority, transfers and effect order did not change. The renewed LLM judgment
finds no contradictory capture lifetime or exception identity. Structural
comparison supports this reading rather than supplying behavioral proof.

The [fresh monitor/binding challenge](../../sync/monitor-surface-boundaries/review.md)
covers capture identity, receiving/value flow, nested failures and escaping
storage. Current fixture/model regressions are separate executed evidence;
this record does not claim VM/device execution for this input.

## Prior verdict — private case loop with shared suffix bypasses

Compared the same prepared DEX on dd70d20 and this candidate. run now owns one
natural case interface. Default breaks the original outer while, skipping all
case effects, shared applied increment and index update. Case0 writes state1,
copies the outer accumulator and breaks to the one shared +1. Case2 writes
state2 and jumps to the downstream index update, skipping +1. Case1 contains
the complete original inner while: matching the remainder preserves the current
inner accumulator and reaches that same downstream update; exhaustion copies
the final inner accumulator and includes +1. Snapshot saves remain immediately
before the original effects and the catch receives after capture.

Read all complete changed paths against Java and original products. Empty input
and default-first input return0 with the original final addition; [0] returns1,
[2]/[1] return0, [5] returns1, and [0,2,1,5,3,0] returns2/final state4 without
reading its last0. Negative remainders retain their actual default. Array failure
still uses the saved current accumulator and cleanup/rethrow. This is source
reasoning, not scenario execution. The remaining original gotos skip real shared
work and are not replaced with a while continue that would skip index update.

The [fresh challenge](../../branches/switch-shared-tails/review.md) adds a typed
catch, outer finally and actual selector-only assignment at a downstream entry.
Original P2/P3/P4/P5, normal edges and copy phases agree; P7/P9 consume supplied
case ownership without discovery. Other members retain their prior review.

## Prior verdict — raw default and protected loop exits

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
