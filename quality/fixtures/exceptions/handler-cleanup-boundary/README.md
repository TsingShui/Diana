# Typed handler cleanup boundary

This original Apache-2.0 challenge extends [cleanup outlet frontiers](../cleanup-outlet-frontiers/README.md).
The original selector is `Lfixture/HandlerCleanupBoundary;->run(I)I`.
The read has a typed handler followed by a catch-all. Acquisition and probe
precede the protected read. Within the typed handler only log has catch-all coverage.
The added audit can throw after log returns and before the conditional close.
Both audit and close lie outside that catch-all range.

For key 7, acquire returns handle 21 and read returns 8. A selected helper
records its invocation before throwing. I is an IOException. P, R, L, A and
C are distinct unchecked exceptions from probe, read, log, audit and close.
The log receives the exact I. An acquisition failure prevents the probe and read and has no cleanup.

| Fault | Trace | Exit |
|---|---|---|
| None | acquire(7), probe, read(21), close(21) | Return 8 |
| Acquire throws I | acquire(7) | Throw I |
| Probe throws P | acquire(7), probe | Throw P |
| Read throws I | acquire(7), probe, read(21), log(I), audit(21), close(21) | Return -2 |
| Read throws I and log throws L | acquire(7), probe, read(21), log(I), close(21) | Throw L |
| Read throws I, log throws L and close throws C | acquire(7), probe, read(21), log(I), close(21) | Throw C |
| Read throws I and audit throws A | acquire(7), probe, read(21), log(I), audit(21) | Throw A |
| Read throws I and its post-audit close throws C | acquire(7), probe, read(21), log(I), audit(21), close(21) | Throw C |
| Read throws R | acquire(7), probe, read(21), close(21) | Throw R |
| Read throws R and close throws C | acquire(7), probe, read(21), close(21) | Throw C |
| Normal close throws C | acquire(7), probe, read(21), close(21) | Throw C |

Smali 3.0.9 and OpenJDK 21.0.12.1 assembled this case for API 24. The
pre-fix build is the 0.3.0 export candidate immediately before correcting
typed handler coverage. The comparison candidate includes that correction.

```sh
fixture=quality/fixtures/exceptions/handler-cleanup-boundary
work=.cache/fixtures/handler-cleanup-boundary
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble --api 24 \
  -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.baksmali.Main disassemble --api 24 \
  -o "$work/disassembly" "$fixture/classes.dex"
diana get-method "$fixture/classes.dex" 'Lfixture/HandlerCleanupBoundary;->run(I)I' \
  -o "$work/HandlerCleanupBoundary.java"
python3 "$fixture/check.py" "$work/HandlerCleanupBoundary.java"
cargo nextest run --locked -p diana-decomp --test handler_cleanup_boundary_test
```

The harness models the native declarations, supplies checked exceptions in the
Java signatures and executes the recovered run body unchanged. Each scenario
checks helper order and arguments, return value, and escaping throwable identity.
