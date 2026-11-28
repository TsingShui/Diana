# Source recovery from a javac-compiled class

`SourceRecovery.java` exercises constructor initialization, numeric operations,
local variables, field accesses, a helper call, forward branches, a loop and a
typed exception handler. `update` also contains javac's standard string
concatenation site.

```sh
fixture=quality/fixtures/jvm/source-recovery
work=.cache/fixtures/jvm-source-recovery
python3 "$fixture/prepare.py" "$work"
java --class-path "$work/classes" "$fixture/Check.java"
java --class-path "$work/opaque-classes" "$fixture/Check.java"
java --class-path "$work/classes" "$fixture/CheckHandlers.java"
cargo nextest run --locked -p diana --test jvm_source_test
```

Preparation needs Python 3 and a JDK 17 or newer. The recipe compiles Java 17
class files with debug metadata. It writes the same class as a standalone
file, a class at `j1` in `source.jar` whose ZIP path differs from its actual
name, and a class at `j1/j0` in the nested JAR listed by `bundle.jar`.
`multi-release.jar` repeats the same class at `j1` and `j2`. A named class
lookup is ambiguous, while either physical location can be selected. The
damaged bundle adds a broken catalog row. A named lookup discloses the
missing catalog facts and keeps the recovered method. A physical lookup
resolves the known archive chain independently.

`opaque-bundle.jar` changes the concatenation bootstrap owner to the
independently authored `CustomBootstrap`. Its bytecode delegates to the
standard bootstrap when executed in the JVM. Diana can preserve the original
dynamic site and surrounding code through an explicit local coverage issue.

## Debug metadata variants

For the DecBench corpus, prepare both Java 17 variants in separate directories.

```sh
fixture=quality/fixtures/jvm/source-recovery
corpus=.cache/decbench/corpus/jvm-source-recovery
python3 "$fixture/prepare.py" "$corpus/debug" --variant debug
python3 "$fixture/prepare.py" "$corpus/stripped" --variant stripped
```

The default `debug` variant retains local-variable, line-number and source
metadata. The `stripped` variant uses `javac -g:none`. Each directory has
the same archive and class layout. The same independent expectations apply to
both variants, including the effect order and escaping exceptions. With the
same JDK and Python compression library, repeated preparation of each variant
produces identical bytes. Keep the prepared files under `.cache/`.

## Independent expectations

| Method | Paths and effects |
| --- | --- |
| `<init>(I)V` | Invoke the superclass constructor, then assign the argument to `value`. |
| `calculate(I)I` | Read `value`, add the argument, pass the sum to `twice` and subtract one from the result. |
| `twice(I)I` | Multiply its argument by two. |
| `choose(I)I` | Add positive inputs to `value`. Subtract zero and negative inputs from `value`. |
| `descendingSum(I)I` | Start from zero. While the input is positive, add it to the sum and decrement it. Return zero if the loop is skipped. |
| `divideOrDefault(II)I` | Return the quotient on success. The handler for `ArithmeticException` returns `-7`. |
| `update(I)I` | Increase `value`, construct `last` from that new value, decrease `value` by two and return the second numeric result. |
| `last()Ljava/lang/String;` | Read and return the original `last` field. |
| `classify(I)I` | `-7` and `40` use the same return `61`. `6` returns `-4`, `100` returns `99`, and other inputs return the input plus `200`. |

`javap` confirms the `invokedynamic` instruction begins at offset 15 of
`update`, followed by `putfield last` at offset 20. The first store to `value`
appears at offset 7 and the second at offset 30. The exception table in
`divideOrDefault` covers `[0, 3)` and starts the typed handler at offset 4.
The custom bootstrap variant preserves those instruction offsets.
The `lookupswitch` in `classify` starts at byte 1. Its first following case
starts at byte 44 and accepts both `-7` and `40`. The default target is byte
50. The `100` case starts later at byte 56.

## Overload binding and effect order

`OverloadWitness.java` distinguishes `char` and `int` formals in static and
instance calls, instance creation, and constructor delegation. The prepared
class is `debug/classes/fixture/overloads/OverloadWitness.class` in DecBench.
At offsets 10, 19, and 25 of `wide(C)I`, the original owner is
`Lfixture/overloads/OverloadWitness;` and the selected descriptors are
`choose(I)I`, `<init>(I)V`, and `select(I)I`. The delegated constructor
`<init>(CZ)V` also calls `<init>(I)V`, at byte offset 2.

`CheckOverload.java` supplies independent expectations for the results and
the ordered trace. The wide route evaluates its character input once and
produces trace `137578`. Its results for `A` and U+FFFE are 2060 and 263936.
The narrow route produces trace `1264` with results 1095 and 197502. On the
0.4.6 baseline, the wide `A` route becomes 1660 with trace `126468` when the
recovered Java is compiled. The narrow control keeps its expected behavior.
Diana 0.4.7 preserves both routes. The CLI integration test recompiles the
exact recovered class and executes the independent expectations.

The generic control passes `String` and `List<String>` to `selectGeneric`.
Its original descriptor is `(Ljava/lang/Object;Ljava/util/List;)I` and the
call returns 1. Widening the first argument to `Object` would conflict with
the type inferred from `List<String>` and select the fallback returning 3.

```sh
python3 quality/decbench run --suite jvm-overload-bindings --binary /path/to/diana
cargo nextest run --locked -p diana --test jvm_source_test recovered_source_preserves_overload_results_and_effect_order
```

## Overlapping JVM exception tables

`ExceptionChains.java` supplies neighboring statements protected by different
ordered handler chains. In `guarded`, the class-file table has `[0, 6)` for
`ArithmeticException` and `[0, 25)` for `ArrayIndexOutOfBoundsException`.
`guardedFinally` adds a catch-all over `[0, 6)`, `[13, 23)`, and `[31, 33)`.
The outer typed exception row covers `[0, 11)`, `[13, 28)`, and `[31, 41)`.

The `trace` field starts at one. Each `mark` appends a digit to expose order.

| Path | `guarded` result and trace | `guardedFinally` result and trace |
| --- | --- | --- |
| Divide `21` by `3` | `7`, `14` | `7`, `13` |
| Divide `21` by zero | `-7`, `124` | `-7`, `123` |
| Read an empty array | `-8`, `15` | `-8`, `135` |
| Read a null array | Escaping `NullPointerException`, `1` | Escaping `NullPointerException`, `13` |

## Local variable type witnesses

`TypedLocals.java` supplies original debug slots for a `long` accumulator,
an `int[]` value, a `String` value and a `double` accumulator. All four
selected locals are declared in slot 1 and flow directly to the method's
return. The independent selector uses that returned-local role in the
recovered publication, so a renamed variable does not change the expectation.

`decbench-local-types-debug.json` also checks the integer accumulator and
typed catch in `SourceRecovery.java`. The stripped counterpart shows how
the benchmark reports absent original debug-type evidence.

## Field order and values between effects

`EffectSequences.java` supplies straight-line field-effect witnesses.
`exchange` must read the old value of `left` before writing the new value,
then store the old value in `right`. `accumulate` updates and reads `left`
before incrementing `count`. That saved value is then written to `right`.
`updatePair` writes the two input parameters to separate fields in order.

The first call to `exchange(7)` on a fresh authored instance returns zero,
and `exchange(19)` on the same instance then returns seven. The
`CheckEffectSequences.java` witness checks these independent expectations.
It also exercises the counter update and the return of the sum in
`updatePair`.

The ordinary Diana 0.4.2 publication writes `left` first and reads it only
for the following write to `right`. The witness passes on the authored
Java and reports expected 0, observed 7 when compiled with the recovered
publication. The effect-order check compares the source evidence separately.
