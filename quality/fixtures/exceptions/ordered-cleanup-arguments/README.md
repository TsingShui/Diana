# Ordered cleanup arguments

This Apache-2.0 challenge extends [sequential cleanup origins](../sequential-cleanup-origins/README.md).
The subject is `Lfixture/OrderedCleanupArguments;->run(II)I`. The separate
catch-all regions reuse v0 across parameter assignments. The second pair
of cleanup bodies both call mark with the completed first read, then invoke
the same close helper. The argument order of that final call distinguishes
the normal path from the handler. The Java example spells out the two paths.
The Smali defines both protected ranges and their handler operands.

## Scenarios

For run(10,20), read(10) returns 3 and read(20) returns 7 unless that
invocation throws E. Each helper records its arguments before returning or
throwing. Mark may throw M. Either occurrence of close may throw C.
Each fault is selected independently by occurrence and arguments.

| Fault | Observable calls in order | Exit |
|---|---|---|
| None | read(10), close(10,20), read(20), mark(3), close(20,10) | Return 10 |
| First read | read(10), close(10,20) | Throw E |
| First read and close | read(10), close(10,20) | Throw C |
| First close | read(10), close(10,20) | Throw C |
| Second read | read(10), close(10,20), read(20), mark(3), close(10,20) | Throw E |
| Second read and handler mark | read(10), close(10,20), read(20), mark(3) | Throw M |
| Second read and handler close | read(10), close(10,20), read(20), mark(3), close(10,20) | Throw C |
| Normal mark | read(10), close(10,20), read(20), mark(3) | Throw M |
| Normal close(20,10) | read(10), close(10,20), read(20), mark(3), close(20,10) | Throw C |

## Preparation

Smali 3.0.9 assembled the original fixture for API 24. The pre-fix candidate
is the export implementation based on `eac4bb4` before the cleanup operand
repair. The fixed candidate is the Diana 0.3.0 release.

```sh
fixture=quality/fixtures/exceptions/ordered-cleanup-arguments
work=.cache/fixtures/ordered-cleanup-arguments
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble --api 24 \
  -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.baksmali.Main disassemble --api 24 \
  -o "$work/disassembly" "$fixture/classes.dex"
diana get-method "$fixture/classes.dex" 'Lfixture/OrderedCleanupArguments;->run(II)I' \
  -o "$work/OrderedCleanupArguments.java"
python3 "$fixture/check.py" "$work/OrderedCleanupArguments.java"
cargo nextest run --locked -p diana-decomp --test sequential_cleanup_origins_test
```
