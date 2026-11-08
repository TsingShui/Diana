# Fixture: reassigned-resource-finally

An acquisition argument, a resource slot reassigned inside the protected region,
and a conditional finally. `ReassignedResourceFinally.java` explains the
behavior; `ReassignedResourceFinally.smali` is the assembly input. The prepared
`classes.dex` was assembled from the Smali and copied beside it.

- Provenance: authored for this repository; no third-party code or data.
- Original owner: `Lfixture/ReassignedResourceFinally;`.
- Subject: `Lfixture/ReassignedResourceFinally;->rotate(I)I`.
- R1 review base: repository HEAD `6d663d9`; engine sources unchanged from
  `12de370`.
- Expected behavior: the scenario table in `README.md`.

## Current verdict — round 2

Manually reread the Java explanation, Smali and fresh P6/P9 delivery at
`09e322b`. Throw-site snapshots retain ownership: first read saves {owned, old};
old close and second open save {not-owned, old}; second read saves {owned, new}.
Consequently an old-close or second-open failure does not close the old handle
again, while a second-read failure closes the new handle. Failing finally close
replaces the pending exit. No semantic mismatch was found under the README's
helper models.

The current text is unchanged from the inspected `6d663d9` engine. Snapshot
annotations and the constant-true normal ownership test obscure the source intent,
but carry the required distinctions. Fresh delivery has no stopped member or
rendering issue. Behavioral execution remains unrun; R1 scores are historical.

## Prior review — round 1

### Semantics

This is a manual source/output reading judgment, not an automated execution result.

| Method | Semantics | Structure | Naming |
|---|---|---|---|
| `rotate` | ~ Equivalent behavior with a different shape: acquisition order, reassignment, the conditional close and close-failure replacement agree; the acquired flag and current handle appear as snapshot registers and bracket annotations. | 18.2% | 50.0% |

### Path comparison

The output starts with `int v1 = read(0); int v2 = open(v1);` and a `try` around
the first read, the optional close/reassignment and the second read. The handler
uses `on_exception_entry {v13 = v16; v11 = v15;}` to restore the acquired flag and
current handle saved before the throwing operation.

| Path | Reference and recovered behavior |
|---|---|
| `mode == 0`, normal | The first read's snapshot is `{v16 = 1; v15 = v2}`. The normal branch records the result and current handle, then the normal conditional close runs and returns the saved result. |
| `mode == 1`, normal | The old close's snapshot is `{v16 = 0; v15 = v2}`. The second acquisition's snapshot is `{v16 = 0; v15 = v2}`. The new read's snapshot is `{v16 = 1; v15 = v7}`. The new handle is closed on the normal tail. |
| Seed read or first open fails | The failure is outside the protected region; the output has no handler for that region and no close. |
| First read fails | The handler restores `{1, v2}` from the throwing read's snapshot and closes v2. |
| Old close fails | The flag was cleared before the close; the snapshot is `{0, v2}`; the handler does not close again and propagates the close failure. |
| Second acquisition fails | The flag is still clear; the snapshot is `{0, old}`; the handler skips the close. |
| Second read fails | The snapshot is `{1, v7}`; the handler closes the new handle and propagates the failure. |
| Normal or handler close fails | The conditional close is outside the protected range; its failure replaces the pending return or exception and is not retried. |
| Return value | The output returns the value saved in `v14`; the first read and the second read are added in the same order as the source. |

The normal-path `if (1 != 0)` is a constant-true condition. Both source paths end
with the acquired flag true, so the condition is equivalent; it is a shape
artifact, not a missing branch. The normal path always has a live handle at the
finally, so closing `v12` corresponds to `if (acquired) close(current)`.

### Concrete mismatch

None found in the reviewed paths. The flag snapshots select the same handle as
the source for every throwing operation, including the two reassignment
boundaries.

### Reading obstruction

The snapshot protocol is the main obstacle. A reader must interpret
`[after_operands_before_effect {v16 = ...; v15 = ...;}]` on each throwing
operation and `on_exception_entry {v13 = v16; v11 = v15;}` in the handler to know
which flag and handle the finally sees. The names `acquired` and `current` do not
survive, and the constant-true normal check `if (1 != 0)` looks like a missing
condition even though it follows from the source. The execution facts are
preserved.

### Uncertain and unreviewed paths

- The helper bodies are native; the values and faults come from the scenario
  table.
- The fixture does not use suppression; a failing finally close replaces the
  pending exit.
- Asynchronous exceptions, constructor failures and concurrent helper calls are
  not modeled.
- No automated behavioral execution or Rust fixture test was run.

### Score (supplementary)

`python3 quality/score.py --slug reassigned-resource-finally` reports structure
18.2% and naming 50.0%. The snapshot annotations and generated names are the main
structural losses; the score is not a verdict.

### Commands

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/reassigned-resource-finally/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/reassigned-resource-finally/classes.dex \
  --exec '{"cmd":"p6-all","details":true}'
python3 quality/score.py --slug reassigned-resource-finally
```

## Rounds

- R1: Prepared `classes.dex` from the authored Smali; manual path comparison finds
  the flag/handle selection and conditional close consistent with the source. The
  snapshot notation and constant-true normal check are recorded as reading
  obstructions.
