# Invoke register words and reference slots

This independently authored fixture combines an ordinary list invoke, a
range invoke with wide arguments, a signature-polymorphic range invoke
and a filled-array range at neighboring original PCs. The original
subject is `Lfixture/WordOrder;->run(DJILjava/lang/invoke/MethodHandle;)I`.

The list passes the long argument and the trailing int to the first
overload of `take`. The five encoded words of the range pass the double,
long and int to the other overload. The receiver of `invokeExact` is the
incoming method handle and its sole argument is the long. Its first
reference identifies the method and its second reference supplies the
`(J)I` call-site prototype. The filled array contains the three returned
values in call order. The final call also passes the first and third
results individually. A thrown call prevents all subsequent calls.

The directional mutations are derived from the published Smali. A
four-word count for the five-word range prevents parameter grouping.
All four surviving words remain individual original operands.
When multiple references are invalid, the first instruction takes
precedence. If both polymorphic slots are invalid, the method slot is
first. An unreadable second slot alone reports the prototype at PC 10.
All changes are method-local and leave the three native declarations
available. The test mutates private copies of the prepared DEX.

The input uses Smali 3.0.9 and OpenJDK 21.0.12.1 at API 28.

```sh
fixture=quality/fixtures/atoms/invoke-word-order
work=.cache/fixtures/invoke-word-order
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble \
  --api 28 -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
diana get-class "$fixture/classes.dex" 'Lfixture/WordOrder;'
cargo nextest run --locked -p diana-decomp --test invoke_word_order_test
```
