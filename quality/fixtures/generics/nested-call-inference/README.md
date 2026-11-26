# Nested generic call inference

The hand-written Smali and Java explanation use Diana's project license.
`Lfixture/NestedCalls;` defines two identity functions. The generic bound on
`bounded` is `List<String>`. The unbounded `id` accepts any reference.

`chain14(Ljava/lang/String;)Ljava/lang/String;` calls `id` fourteen times. The
first call receives the input and every subsequent call receives its predecessor's
result. The final DEX cast accepts the original string or null. The reviewed Java
source nests all fourteen calls in the same order. Neither helper modifies its input.

`choose(ZLjava/util/List;Ljava/util/List;)Ljava/util/List;` calls `bounded`
once on the list selected by the boolean. The true branch has a parameterized
input. The false branch supplies a raw list, which requires an unchecked source
conversion. The reviewed output keeps the cast on the false branch and applies
`id` once after the selection. Both helper bodies return their input. The DEX
cast and the emitted casts operate on list references and accept null. The
helpers never read an element, so arbitrary elements in the raw list reach the
caller unchanged. This path review compares the Smali and decompiled source.

The prepared DEX was assembled with Smali 3.0.9 and OpenJDK 21.0.12.1 at API 28.

```sh
fixture=quality/fixtures/generics/nested-call-inference
work=.cache/fixtures/nested-generic-inference
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble \
  --api 28 -o "$work/classes.dex" "$fixture/NestedCalls.smali"
cp "$work/classes.dex" "$fixture/classes.dex"
cargo nextest run --locked -p diana-decomp --test nested_generic_calls_test
```
