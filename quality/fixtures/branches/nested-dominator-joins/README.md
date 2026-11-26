# Nested dominator joins

This independently authored challenge extends the [protected branch
joins](../protected-branch-joins/README.md) fixture. The five stages use
separate nested decisions, alternate arm bypasses and exits to the same
physical return. The initial comparison uses Diana commit `7be3f1e`.

`Lfixture/NestedJoins;->run(II)I` reads its key in five successive
three-bit groups, starting at the least significant bit. In each group
a set bit 0 selects `left` or `right` according to bit 1 (clear or set).
Their argument is the value entering the stage. A clear bit 0 bypasses
both calls. On the bypass path a set bit 2 jumps to `finish` with the
unchanged value, skipping the stage's `join` and all later stages. Every
other path calls `join` with the selected or carried value and passes its
result into the next stage. The fifth successful join feeds `finish`.

The native helpers model independent, observable calls that may return
any integer or throw. Any exception escapes immediately. The Java
explanation uses a loop; Smali unrolls it and shares one `finish` block
between all early and ordinary returns.

The DEX was assembled with Smali 3.0.9 and OpenJDK 21.0.12.1 at API 24.

```sh
fixture=quality/fixtures/branches/nested-dominator-joins
work=.cache/fixtures/nested-dominator-joins
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble \
  --api 24 -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
diana get-class "$fixture/classes.dex" 'Lfixture/NestedJoins;'
cargo nextest run --locked -p diana-decomp --test nested_dominator_joins_test
```
