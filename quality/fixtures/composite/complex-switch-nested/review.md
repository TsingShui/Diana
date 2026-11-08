# Fixture: complex-switch-nested

switch inside a loop inside try, with labelled break and continue. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

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
