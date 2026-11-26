# Loop with an exception-only latch

This independently authored fixture extends the protected loops in
[protected loop exits](../../exceptions/protected-loop-exits/README.md).
The only edge back to the loop header comes from a typed catch. All ordinary
switch cases terminate, two distinct case labels share a return, and an
exception from either call in that return reaches the same handler.

The original subject is `Lfixture/LoopOwners;->dispatch(II)I`. When
`remaining <= 0`, it returns `empty(remaining)`. A positive value calls
`choose(remaining, mode)`. Keys 0 and 1 return `second(first(remaining))`,
key 2 returns `alternate(remaining)`, and all other keys return
`stop(remaining)`. Each helper can have effects or throw.

An `IllegalStateException` from any protected helper calls `retry` with
the current value. After a successful retry the loop decrements the value
and tests it again. Other exceptions escape. Failures in `retry` or
`empty` escape even when they have the caught type.

The Java explanation includes an implicit constructor. The Smali input
contains only the subject and its seven native helpers. The prepared DEX
uses Smali 3.0.9 and OpenJDK 21.0.12.1 at API 24.

```sh
fixture=quality/fixtures/loops/exception-only-latch
work=.cache/fixtures/exception-only-latch
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble \
  --api 24 -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
diana get-class "$fixture/classes.dex" 'Lfixture/LoopOwners;'
cargo nextest run --locked -p diana-decomp --test exception_only_latch_test
```
