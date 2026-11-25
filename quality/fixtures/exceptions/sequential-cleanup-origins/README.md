# Sequential cleanup origins

This Apache-2.0 fixture is independently authored for Diana. The original selector is
`Lfixture/SequentialCleanupOrigins;->readBoth(II)I`. Its parent fixture
[reassigned-resource-finally](../reassigned-resource-finally/README.md) uses a
single cleanup region whose handle changes within its protected body. This
challenge reuses one DEX register for two different parameters across two
consecutive catch-all regions. It exercises the lifetime of the method-wide
SSA origin facts used when comparing normal and exceptional cleanup copies.

In the first region the normal close uses a move of the first parameter and
the handler closes that parameter directly. In the second region the normal
close uses a move of the second parameter while the handler closes the first.
All four close instructions are outside both protected ranges. The same helper
and register on both normal paths make the distinct reaching definitions
material to recovery.

## Independent expectations

For inputs 10 and 20, successful reads return 3 and 7 respectively. Each helper
records its argument before it returns or throws. The test model assigns any
selected read failure the identity E and any selected close failure the distinct
identity C. The native close helper accepts repeated calls. Every row starts
with a fresh trace.

| Fault | Ordered calls | Exit |
|---|---|---|
| None | read(10), close(10), read(20), close(20) | Return 10 |
| First read throws | read(10), close(10) | Throw E |
| First read and its cleanup throw | read(10), close(10) | Throw C |
| First normal close throws | read(10), close(10) | Throw C |
| Second read throws | read(10), close(10), read(20), close(10) | Throw E |
| Second read and its cleanup throw | read(10), close(10), read(20), close(10) | Throw C |
| Second normal close throws | read(10), close(10), read(20), close(20) | Throw C |


The companion method `Lfixture/SequentialCleanupOrigins;->closeBoth(II)I`
uses the same reads and register reassignment. Its second handler closes the
second parameter directly. Each region therefore has equivalent normal and
exceptional close operands, with distinct origins across the two regions. For
this method the two rows with a failing second read end in close(20). All
other calls, exits and throwable identities are as listed above.

## Preparation and comparison

The fixture was assembled with Smali 3.0.9 for API 24 on OpenJDK 21.0.12.1.
Baksmali 3.0.9 disassembly of the published DEX confirmed both catch-all ranges
in each method, both assignments to v0, and all four close operands.

```sh
fixture=quality/fixtures/exceptions/sequential-cleanup-origins
work=.cache/fixtures/sequential-cleanup-origins
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble --api 24 \
  -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.baksmali.Main disassemble --api 24 \
  -o "$work/disassembly" "$fixture/classes.dex"
diana get-class "$fixture/classes.dex" 'Lfixture/SequentialCleanupOrigins;'
cargo nextest run --locked -p diana-decomp --test sequential_cleanup_origins_test
```

The pre-cache export candidate based on `eac4bb4` and the Diana 0.3.0
candidate with method-wide origin reuse read the same prepared DEX. The
[semantic review](review.md) records the pre-existing wrong operand and its repair. The
[ordered cleanup arguments](../ordered-cleanup-arguments/README.md) challenge
exercises the same merge with a shared preceding effect and swapped arguments.
