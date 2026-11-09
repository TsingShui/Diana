# Fixture: sync-finally-return

A finally inside a synchronized block with an early return and a following record
effect. `SyncFinallyReturn.java` explains the behavior;
`SyncFinallyReturn.smali` is the assembly input. The prepared `classes.dex` was
assembled from the Smali and copied beside it.

- Provenance: authored for this repository; no third-party code or data.
- Original owner: `Lfixture/SyncFinallyReturn;`.
- Subject: `Lfixture/SyncFinallyReturn;->syncFinally(Ljava/lang/Object;I)I`.
- R1 review base: repository HEAD `6d663d9`; engine sources unchanged from
  `12de370`.
- Expected behavior: the scenario table in `README.md`.

## Current verdict — monitor continuation review

Java, actual Smali, native-helper scenarios and final syncFinally(Object,int)
were reread. The same goto L7 after release is now the supplied control continuation
rather than a duplicate ordinary transfer. Both branches still close while held;
mode1 skips record and returns captured 70 after release. Normal mode records 0
before release. Read, close and record failures retain replacement/identity and
release order, with no cleanup retry. Null acquisition and open failure remain
outside held cleanup. The output change is annotation-only, but its source routing
was checked rather than treated as automatic evidence of equivalence.

This follow-up is bounded direct LLM path reading, not executed behavioral PASS.
Corpus inspection preserves exact P2/P3/P4/P5 and original P6 graph/lock facts,
including synchronized release targets, routing and before/after plans. Unchanged
text reuses prior reviews. See the new
[monitor-exit challenge](../../sync/monitor-iteration-exits/review.md) for scope
and the initial candidate regression; no device run or Java recompilation ran.

## Prior verdict — round 2

Manually reread the Java explanation, Smali and fresh P6/P9 delivery at
`09e322b`. Close runs while locked on normal, early-return and read-failure
paths. Mode 1 skips record but still closes and releases. A close or record
failure reaches H10 and releases once before propagation; there is no close retry.
Null monitor acquisition reaches neither body nor cleanup. No semantic mismatch was
found under the README's helper and monitor models.

The current text is unchanged from the inspected `6d663d9` engine. The finally
and monitor cleanup remain split across the natural inner try and an explicit
chain. Fresh delivery has no stopped member or rendering issue. Behavioral
execution remains unrun.

## Prior review — round 1

### Semantics

This is a manual source/output reading judgment, not an automated execution result.

| Method | Semantics |
| --- | --- |
| `syncFinally` | ~ Equivalent behavior with a different shape: close-before-release, record placement, early-return skipping and exception replacement agree; the monitor release is an explicit `H10` path. |

### Path comparison

The output is:

```text
int v2 = open(0);
synchronized (p0) { // Diana: releases are explicit
    try { v3 = read(v2); }
    catch (all) { v4 = capture; close(v2) [exn c2]; throw v4 [exn c2]; }
    close(v2) [exn c2];
    if (p1 != 1) { record(0) [exn c2]; }
    release synchronized (p0);
    goto L7;
    H10: v5 = capture; release synchronized (p0); throw v5;
}
L7: return v3;
```

| Path | Reference and recovered behavior |
|---|---|
| `mode == 0`, normal | O(0), LOCK, R(7), C(7), REC(0), UNLOCK, return 70. The output prints the close, the conditional record, the release and the jump to the return. |
| `mode == 1`, early return | O(0), LOCK, R(7), C(7), UNLOCK, return 70. The `if (p1 != 1)` condition is false, so the record is skipped; the release and return still run. |
| Read failure | The body catch captures the failure, closes v2, and rethrows through `c2` to `H10`; `H10` releases the monitor and rethrows. Close runs before release. |
| Close failure on the normal, early-return or catch path | The close call carries `c2`; `H10` releases the monitor and propagates the close failure. The pending return is discarded. |
| Record failure | The record call carries `c2`; `H10` releases the monitor and propagates the record failure after the close. |
| Null lock | The monitor acquisition fails before the protected body; no close or record runs on that path. |

The source's `return total` inside the synchronized try and the normal fallthrough
both reach the finally close and then the release. The output expresses the early
return by skipping the conditional record; the close and release are shared. The
return operand `v3` is read before the finally, so a close or record failure
replaces it rather than changing its value.

### Concrete mismatch

None found in the reviewed paths. The close-before-release order, early-return
skip and exception replacement agree with the source.

### Reading obstruction

The body has a natural `try`/`catch` for the read, but the finally close is an
inline call after the try and the monitor release is an `H10` block reached only
through `exception_chain c2`. A reader must know that the inline close and the
`H10` release together form the finally, and that a close failure on any path
reaches `H10` without a second close. The `goto L7` after the release is a normal
scope exit, but it looks like a detached jump. These are shape and notation
obstacles; the execution facts are preserved.

### Uncertain and unreviewed paths

- The helper bodies are native; the scenario table supplies their results and
  faults.
- The null-lock monitor failure and asynchronous exceptions are not executed.
- No automated behavioral execution or Rust fixture test was run.

### Commands

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/sync-finally-return/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/sync-finally-return/classes.dex \
  --exec '{"cmd":"p6-all","details":true}'
```

## Rounds

- R1: Prepared `classes.dex` from the authored Smali; manual path comparison finds
  the close/release/record ordering and early-return selection consistent with
  the source. The explicit `H10` release and inline finally close are recorded as
  a reading obstruction.
