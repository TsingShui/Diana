# Conditional writes to bound source variables

The Smali and Java explain a conditional write to a parameter that shares a source variable with the incoming argument. The opaque native helpers record their arguments and can throw. The boolean returned by `take` controls which later effects occur.

In `guarded`, resetting the argument or supplying zero returns minus one before either call. For any other input, both arguments of `take` are the incoming integer converted to float. When `take` returns false, `done` receives the same two values. A normal exit returns the integer input.

The challenge `pair` applies two independently guarded defaults. When the selected integers are both zero the method returns minus one. Otherwise the ordered pair of converted values goes to `take`. A false result calls `done` on that pair and then `keep` on the selected integers. Both normal call paths return their integer sum with DEX wrapping.

In `caughtDefault`, a normal return from `mayThrow` returns minus one. A runtime exception supplies a fresh binding. The `clear` flag can overwrite that binding with null. The selected object determines the `instance-of` result. The `repeat` flag adds one preceding `replacement` and one preceding `keep` call. The final `keep` and `replacement` always occur in this order on a normally returning handler. Other exception types and exceptions from handler helpers escape.

The local in `localConstant` always starts at 37. Both float operands of every call have the exact value 37.0. The flag determines whether there are one or two calls.

The fixture was assembled with Smali 3.0.9 on OpenJDK 21.0.12.1 at API 24.

```sh
fixture=quality/fixtures/branches/bound-value-folding
work=.cache/fixtures/bound-value-folding
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble \
  --api 24 -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
diana get-class "$fixture/classes.dex" 'Lfixture/BoundValues;'
cargo nextest run --locked -p diana-decomp --test bound_value_folding_test
```
