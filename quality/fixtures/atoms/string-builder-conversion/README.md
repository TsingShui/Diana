# Object conversion after precomputed builder arguments

The independently authored [Smali](BuilderConversion.smali) defines the
DEX input. The [Java reference](BuilderConversion.java) exercises actual
JDK builder conversion with recorded producer calls, `toString` calls
and distinct throwable objects. The original class is
`Lfixture/BuilderConversion;`.

| Method | Original locator | Chain |
| --- | --- | --- |
| `precomputed(I)Ljava/lang/String;` | `e0:l0:c0:m1` | `StringBuilder` with both Object producers evaluated before construction |
| `protectedBuffer(I)Ljava/lang/String;` | `e0:l0:c0:m3` | The same ordering through `StringBuffer`, with typed producer and conversion handlers |

The two producers execute at code units 1 and 6. The builder or buffer is
allocated at 10 and initialized at 12. The two `append(Object)` calls at
15 and 19 may invoke user-defined `toString()`. The final `toString()` at
23 reads the builder result. The protected interval [0,27) maps
`IllegalArgumentException` to the producer handler at 28 and
`IllegalStateException` to the conversion handler at 36.

The [scenarios](scenarios.json) compare full ordered event traces and
throwable identity. The second producer must finish before either object
is converted. If it throws, there is no conversion. A first conversion
fault follows both producers and prevents the second conversion. If
both the second producer and the first conversion are configured to
throw, the producer fault wins and selects its original typed handler.
A null first object produces `null` without a `toString()` event.
An unmatched exception from the second producer escapes both handlers.

The input was assembled with Apktool 2.11.1 and OpenJDK 21.0.12.1 using
`SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`
(API 24). Its SHA-256 is
`34a66d1fd95acee598dd5ca8cd185c88ed7e5f47c20209483baebb408b1090c9`.

The original DEX witness and independent Java reference agree on all
15 scenarios. Exact Diana 0.4.11 recovery fails 11. It folds both
chains to concatenations that convert the first object before the
second producer. The overlapping-fault scenarios propagate a different
throwable and can select a different typed catch.
The 0.4.12 candidate recovers both producers before the builder chain
and passes all 15 scenarios.

```sh
diana get-method quality/fixtures/atoms/string-builder-conversion/classes.dex \
  'Lfixture/BuilderConversion;->protectedBuffer(I)Ljava/lang/String;' --format json
python3 -B quality/decbench run --suite dex-builder-object-conversion --binary /path/to/diana
```
