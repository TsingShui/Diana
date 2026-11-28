# JVM floating-point compare recovery

The generator and Java witness are independently authored under the
[repository license](../../../../../LICENSE). `make_jvm.py` emits
`Lfixture/FloatCompare;` using Python's standard library. Its first eight
methods preserve the original reproduction. `--base-only` recreates that
original class byte for byte. The extended class adds `fcmpg`, `fcmpl`,
`dcmpg` and `dcmpl` controls and predicates. The Java runner resets the
effect trace to zero and checks exactly one left call followed by exactly
one right call in each scenario.

| Exact original methods | Operation under test |
| --- | --- |
| `j0:m4` `one()Z` | `fcmpg` tested for equality with +1 |
| `j0:m5` `zero()Z` | `fcmpg` tested for greater than zero |
| `j0:m6` `value()I` | `fcmpg` returned as an integer |
| `j0:m7` `nanSkips(Ljava/lang/RuntimeException;)Z` | Left NaN, followed by a right operand that can throw the exact supplied exception |
| `j0:m12` `floatHigh(FF)I`, `j0:m13` `floatLow(FF)I` | Float ordinal with positive and negative NaN bias |
| `j0:m14` `doubleHigh(DD)I`, `j0:m15` `doubleLow(DD)I` | Double ordinal with positive and negative NaN bias |
| `j0:m16` `highPositive(FF)Z`, `j0:m17` `lowNegative(FF)Z` | Direct +1 and -1 predicates with corresponding NaN bias |
| `j0:m18` `nestedOperands(FF)Z` | Negation around each labeled float operand before comparison |
| `j0:m19` `nestedResult(FF)Z` | Addition around the signed comparison result before the predicate |

High-biased comparisons return `1` on unordered values and low-biased
comparisons return `-1`. Both orders of opposite signed zero compare equal.
Left NaN still requires right-operand evaluation. If the right operand
throws, the observer must receive that same exception object after trace `12`.
The nested predicates are the complexity challenge. They put the producer
calls below `fneg` and the comparison result below `iadd`, respectively.

```sh
fixture=quality/fixtures/atoms/float-comparison-effects/jvm
python3 -B "$fixture/verify.py" --binary /path/to/diana \
  --output .cache/fixtures/float-comparison-effects/jvm/report.json
cargo nextest run --locked -p diana -E 'test(=recovered_jvm_float_comparisons_preserve_operand_effects)'
```

This requires Python 3 and a JDK (Java 17 or later). `verify.py` compiles the
runner once, publishes the original full class with Diana, recompiles the
exact returned Java unchanged, and runs both with that same runner class. The
receipt locates the original and rebuilt bytecode, exact publication, independent
`javap` output, and both execution traces. Exit 0 indicates all 75 scenarios
passed, 1 indicates execution differences, and 2 indicates a setup failure.
Omit `--binary` to run just the independent original control.

On OpenJDK 21.0.12.1 the original class passed 75 of 75 scenarios. The frozen
Diana 0.4.15 baseline at `60814a33` passed 40. `j0:m4` returned the expected
Boolean while executing producer trace `121212`. The throwing `j0:m7` case
returned normally with trace `1`. The simple comparison and materialized
ordinal controls passed. The repair's debug build passed all 75 scenarios,
including both nested predicates. [Semantic review](review.md)
