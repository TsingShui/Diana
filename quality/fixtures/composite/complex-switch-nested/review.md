# Fixture: complex-switch-nested

switch inside a loop inside try, with labelled break and continue. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 2

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
