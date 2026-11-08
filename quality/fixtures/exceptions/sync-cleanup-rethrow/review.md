# Fixture: sync-cleanup-rethrow

A synchronized scope holds the monitor across a resource catch that closes and
rethrows. `SyncCleanupRethrow.java` explains the behavior;
`SyncCleanupRethrow.smali` is the assembly input. The prepared `classes.dex` was
assembled from the Smali and copied beside it.

- Provenance: authored for this repository; no third-party code or data.
- Original owner: `Lfixture/SyncCleanupRethrow;`.
- Subject: `Lfixture/SyncCleanupRethrow;->guarded(Ljava/lang/Object;I)I`.
- R1 review base: repository HEAD `6d663d9`; engine sources unchanged from
  `12de370`.
- Expected behavior: the scenario table in `README.md`.

## Current verdict — round 2

Manually reread the Java explanation, Smali and fresh P6/P9 delivery at
`09e322b`. Normal execution releases before its close. Read/body failure closes
while locked, suppresses a failing close on the primary, then releases and
rethrows that primary. A modeled suppression-call failure releases and propagates
that failure instead. Null monitor acquisition reaches neither body nor cleanup.
No semantic mismatch was found under the README's models.

The current text is unchanged from the inspected `6d663d9` engine. H9/H11/H13
still separate resource catch, suppression and monitor cleanup. The extra
planned-continuation goto is unreachable because the preceding arms already
jump or throw. Both are readability findings. Fresh delivery has no stopped
member or rendering issue. Behavioral execution remains unrun; R1 scores below
are historical.

## Prior review — round 1

### Semantics

This is a manual source/output reading judgment, not an automated execution result.

| Method | Semantics | Structure | Naming |
|---|---|---|---|
| `guarded` | ~ Equivalent behavior with a different shape: lock acquisition and release order, close/suppression order, primary identity and the normal close after release agree; the catch and monitor cleanup are explicit graphs. | 52.7% | 57.1% |

### Path comparison

The output is:

```text
synchronized (p0) { // Diana: releases are explicit
    v3 = read(v2) [exn c1];
    if (p1 != 1) { release synchronized (p0); goto L7; }
    else { ... throw IllegalStateException [exn c1]; }
    goto L7; // planned continuation
    H9:  capture; close(v2) [exn c2]; goto L12;
    H11: capture; addSuppressed(v7) [exn c3];
    L12: throw v6 [exn c3];
    H13: capture; release synchronized (p0); throw v8;
}
L7: close(v2); return v3;
```

| Path | Reference and recovered behavior |
|---|---|
| Normal `mode == 0` | O(0), LOCK, R(5), UNLOCK, C(5), return 50. The output releases inside the `if` branch, jumps to `L7`, closes and returns. |
| `mode == 1` | O(0), LOCK, R(5), constructed `ISE`, C(5), UNLOCK, throw `ISE`. The throw routes through `c1` to `H9`; `H9` closes, then `H13` releases and throws the captured value. |
| Read failure | Same route as mode 1 with the captured read failure; close runs before release. |
| Read failure and close failure | `H9` calls C(5); its failure routes through `c2` to `H11`, which calls `addSuppressed` on the captured primary. The rethrow routes through `c3` to `H13`, which releases and rethrows the primary. |
| `addSuppressed` failure | `H11` has `[exn c3]`; the failure releases through `H13` and propagates, replacing the primary as an abrupt catch completion. |
| Normal close failure | The close at `L7` has no chain marker and runs after the release; its failure propagates without re-entering the resource catch. |
| Null lock | The monitor acquisition fails before the protected body; no close is printed on that path. |

The `goto L7; // planned continuation` after the `if`/`else` is unreachable on the
reviewed paths: the true arm already jumps to `L7`, and the false arm throws. It
does not create an extra runtime path, but it is visible as a planned-continuation
artifact inside the synchronized body. The condition is printed inverted
(`p1 != 1`) with the body arms exchanged; the selected effects match the source.

### Concrete mismatch

None found in the reviewed paths. The lock/close ordering and primary/suppressed
identities agree with the source.

### Reading obstruction

The catch that closes and rethrows is not printed as a `catch` clause. It appears
as `H9`/`H11`/`L12`/`H13` blocks inside the synchronized body, and the reader must
follow three `exception_chain` lines to know which block owns the read, the close
failure and the monitor release. `H13` is the monitor cleanup, `H9` is the resource
catch and `H11` is its suppression handler. The explicit `release synchronized`
statements are required by the delivered notation, but they make the normal path
and the monitor failure path look like separate program logic. The unreachable
`goto L7; // planned continuation` adds a further ambiguity. This is a concrete
readability obstacle; the execution facts are preserved.

### Uncertain and unreviewed paths

- The helper bodies are native; the scenario table supplies their returns and
  faults.
- The null-lock scenario assumes the monitor acquisition throws before the
  protected body; it is not exercised by an executor.
- Asynchronous exceptions and monitor implementation failures are not modeled.
- No automated behavioral execution or Rust fixture test was run.

### Score (supplementary)

`python3 quality/score.py --slug sync-cleanup-rethrow` reports structure 52.7%
and naming 57.1%. The score still reflects explicit chains and handler labels; it
is not a verdict.

### Commands

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/sync-cleanup-rethrow/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/sync-cleanup-rethrow/classes.dex \
  --exec '{"cmd":"p6-all","details":true}'
python3 quality/score.py --slug sync-cleanup-rethrow
```

## Rounds

- R1: Prepared `classes.dex` from the authored Smali; manual path comparison finds
  the monitor and cleanup ordering consistent with the source. The explicit
  handler graph and unreachable continuation artifact are recorded as reading
  obstructions.
