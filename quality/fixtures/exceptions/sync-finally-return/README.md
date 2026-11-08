# Finally inside a synchronized block with an early return

## Input and expectations

The Java explanation and Smali input are independently authored for this
repository; they contain no third-party code or data. Original owner:
`Lfixture/SyncFinallyReturn;`. The subject is
`Lfixture/SyncFinallyReturn;->syncFinally(Ljava/lang/Object;I)I`.

`open(I)I`, `read(I)I`, `close(I)V` and `record(I)V` are native helpers. The
method opens a resource, enters `synchronized (lock)`, runs a try/finally that
closes the resource while the monitor is held, and executes `record(0)` only when
the early-return branch is not taken. The monitor is released after the finally
on both paths. The lock is a distinct non-null object except in the explicit
null-lock scenario.

Each scenario starts with an empty trace. Unless a scenario states a fault,
`open(0) = 7`, `read(7) = 70`, `close(7)` returns normally, and `record(0)`
returns normally. Mode 1 takes the early return. Fault throwables are distinct
and non-null: `E` for a read failure, `C` for a close failure, `A` for a record
failure, and `NullPointerException` for a null monitor. No asynchronous exception
or allocation failure is assumed.

Notation: `O`, `R`, `C`, `REC` denote open, read, close and record; `LOCK`,
`UNLOCK` denote monitor events.

| Subject and arguments | Fault | Expected exit | Ordered calls and effects | Lock events |
|---|---|---|---|---|
| `syncFinally(lock, 0)` | none | return 70 | O(0), R(7), C(7), REC(0) | LOCK, C(7), REC(0), UNLOCK |
| `syncFinally(lock, 1)` | early return | return 70 | O(0), R(7), C(7) | LOCK, C(7), UNLOCK |
| `syncFinally(lock, 0)` | R(7) throws E | throw E | O(0), R(7), C(7) | LOCK, C(7), UNLOCK |
| `syncFinally(lock, 0)` | R(7) throws E; C(7) throws C | throw C | O(0), R(7), C(7) | LOCK, C(7), UNLOCK |
| `syncFinally(lock, 0)` | C(7) throws C | throw C | O(0), R(7), C(7) | LOCK, C(7), UNLOCK |
| `syncFinally(lock, 0)` | REC(0) throws A | throw A | O(0), R(7), C(7), REC(0) | LOCK, C(7), REC(0), UNLOCK |
| `syncFinally(lock, 1)` | C(7) throws C | throw C | O(0), R(7), C(7) | LOCK, C(7), UNLOCK |
| `syncFinally(null, 0)` | null monitor | throw NullPointerException | O(0) | LOCK fails before the body |
| `syncFinally(lock, 0)` | O(0) throws E | throw E | O(0) | none |

The finally close runs while the monitor is held on the normal path, the early
return, the read-failure path and the close-failure path. The early return skips
`record(0)` but still closes and releases. A close failure replaces the pending
return or exception. A record failure occurs after the close and before the
release, so the monitor is released before the record failure propagates.

## Trigger structure

The Smali enters the monitor with `monitor-enter p0`. The read is protected by a
body handler that closes and rethrows. The body handler, the normal finally close
and the record call are inside a second catch-all range whose target releases the
monitor and rethrows. The early-return branch and the normal path share the
finally close; the branch selects whether `record(0)` runs. The monitor-exit and
return are outside every protected range.

The prepared DEX is assembled from `SyncFinallyReturn.smali`. The p6 product
reports one natural try item, one synchronized plan, three lock events and three
chains (two handler chains plus the uncaught chain) for `syncFinally`.

## Preparation and inspection

```sh
mkdir -p .cache/fixtures/sync-finally-return
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/sync-finally-return/classes.dex \
  quality/fixtures/exceptions/sync-finally-return
cp .cache/fixtures/sync-finally-return/classes.dex \
  quality/fixtures/exceptions/sync-finally-return/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/sync-finally-return/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. The current
confirmation used repository HEAD `6d663d9`; engine sources are unchanged from
`12de370`. `review.md` records the reading judgment.

## Review status

The review is a manual source/output reading judgment. No automated behavioral
executor, Java recompilation or fixture test was run. The helper and monitor
failures are scenario inputs; helper bodies are native declarations.
