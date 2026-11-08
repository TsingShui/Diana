# Synchronized cleanup and rethrow

## Input and expectations

The Java explanation and Smali input are independently authored for this
repository; they contain no third-party code or data. Original owner:
`Lfixture/SyncCleanupRethrow;`. The subject is
`Lfixture/SyncCleanupRethrow;->guarded(Ljava/lang/Object;I)I`.

`open(I)I`, `read(I)I` and `close(I)V` are native helpers. The method acquires a
resource, enters `synchronized (lock)`, reads the resource inside a
`Throwable` catch that closes and rethrows, and closes the resource again after the
monitor is released on the normal path. The lock is a distinct non-null object in
every scenario except the explicit null-lock scenario. Lock acquisition and
release are ordered events, not helper calls.

Each scenario starts with an empty trace. Unless a scenario states a fault,
`open(0) = 5`, `read(5) = 50`, and `close(5)` returns normally. Fault throwables
are distinct and non-null: `E` for a read failure, `C` for a close failure, `A`
for an `addSuppressed` failure, and `ISE("body")` for mode 1. A null lock makes
the monitor acquisition throw `NullPointerException` before the protected body.
No asynchronous exception or allocation failure is assumed.

Notation: `O`, `R`, `C` denote open, read and close; `S(E,C)` denotes
`E.addSuppressed(C)`; `LOCK`, `UNLOCK` denote monitor events. The calls column
lists helper calls; lock events are shown separately.

| Subject and arguments | Fault | Expected exit | Ordered calls | Lock events |
|---|---|---|---|---|
| `guarded(lock, 0)` | none | return 50 | O(0), R(5), C(5) | LOCK, UNLOCK before C(5) |
| `guarded(lock, 1)` | body ISE | throw ISE("body") | O(0), R(5), C(5) | LOCK, C(5), UNLOCK |
| `guarded(lock, 0)` | R(5) throws E | throw E | O(0), R(5), C(5) | LOCK, C(5), UNLOCK |
| `guarded(lock, 0)` | R(5) throws E; C(5) throws C | throw E, suppressed [C] | O(0), R(5), C(5), S(E,C) | LOCK, C(5), S(E,C), UNLOCK |
| `guarded(lock, 0)` | normal C(5) throws C | throw C | O(0), R(5), C(5) | LOCK, UNLOCK, C(5) |
| `guarded(lock, 0)` | R(5) throws E; C(5) throws C; S throws A | throw A | O(0), R(5), C(5), S (throws A) | LOCK, C(5), S, UNLOCK |
| `guarded(null, 0)` | null monitor | throw NullPointerException | O(0) | LOCK fails before the body |
| `guarded(lock, 0)` | O(0) throws E | throw E | O(0) | none |

The normal path releases the monitor before the second close. The exceptional
path closes and suppresses while the monitor is still held, then releases the
monitor, then rethrows. A normal close failure cannot be suppressed by the
catch whose close already ran; it propagates after the release. A null lock never
enters the protected body, so the acquired resource is not closed.

## Trigger structure

The Smali enters the monitor with `monitor-enter p0`. The read body is protected
by the resource catch handler. The resource catch handler is itself protected by
the monitor catch-all, so an exception from the close, the suppression call or the
rethrow releases the monitor before leaving. The normal close is after
`monitor-exit p0` and outside both handlers. The suppression handler protects only
the close call inside the catch.

The prepared DEX is assembled from `SyncCleanupRethrow.smali`. The p6 product
reports one synchronized plan, three lock events, no natural try item and four
chains (three handler chains plus the uncaught chain) for `guarded`.

## Preparation and inspection

```sh
mkdir -p .cache/fixtures/sync-cleanup-rethrow
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/sync-cleanup-rethrow/classes.dex \
  quality/fixtures/exceptions/sync-cleanup-rethrow
cp .cache/fixtures/sync-cleanup-rethrow/classes.dex \
  quality/fixtures/exceptions/sync-cleanup-rethrow/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/sync-cleanup-rethrow/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. The current
confirmation used repository HEAD `6d663d9`; engine sources are unchanged from
`12de370`. `review.md` records the reading judgment.

## Review status

The review is a manual source/output reading judgment. No automated behavioral
executor, Java recompilation or fixture test was run. The helper and lock failures
are scenario inputs; helper bodies are native declarations.
