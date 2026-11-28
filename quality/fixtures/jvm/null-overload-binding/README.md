# Null invocation overload binding

These independently authored Java sources compile to JVM 17. The prepared
artifact contains `NullableOverload.class`, including the overloaded generic
definitions. The helper and observer remain on the compiler and runtime
classpaths.

| Original method | Selected descriptor | Expected execution |
| --- | --- | --- |
| `object` (`j0:m1`) | `pick(Ljava/lang/Object;)Ljava/lang/String;` | `object/1` |
| `integer` (`j0:m2`) | `pick(Ljava/lang/Integer;)Ljava/lang/String;` | `integer/2` |
| `guarded` (`j0:m3`) | `choose(ILjava/lang/Object;)Ljava/lang/String;` | `guarded-object/56` or `leading-fault/58` |
| `bounded` (`j0:m4`) | `NullEffects.bounded(Ljava/lang/Object;)Ljava/lang/String;` | `bounded/9` |

The `object` call projects the receiver's type parameter to `Number`. The
`guarded` challenge projects it to `Number[]` and adds a variable-arity
competitor, an effectful leading operand, and a typed handler. On success the
leading operand returns 42 after effect 5, and the selected overload records
effect 6. On failure the leading operand throws the supplied
`IllegalStateException`, so the consumer is skipped and the handler records
effect 8 after checking exception identity. The competing overload records
effect 7.

The `bounded` control reaches an external generic method whose erased
parameter is `Object`. Its original signature also constrains the type
parameter to `Runnable`. The standalone artifact omits that declaration.
Its bare null leaves the helper's bound admissible.

`verify.py` checks original JVM invokes and the guarded exception table with
`javap`. It recompiles the full class and the selected-method publications.
For the selected endpoint it adds the independently authored overload members
to the class wrapper around the recovered method. The verifier checks the
recompiled invoke prototypes, guarded call coverage, and observed effects. The
standalone runs used OpenJDK 21.0.12.1 targeting Java 17.

```sh
fixture=quality/fixtures/jvm/null-overload-binding
work=.cache/fixtures/null-overload-binding
python3 -B "$fixture/verify.py" --binary target/debug/diana --output "$work/proof.json"
python3 -B quality/decbench prepare --prepared "$work/prepared"
python3 -B quality/decbench run --prepared "$work/prepared" \
  --binary target/debug/diana --suite jvm-null-overload-binding \
  --output "$work/decbench.json"
```

Diana 0.4.28 passes six checks and fails four on the selected methods. The
full-class source casts two null arguments to the erased formal and cannot
be recompiled for execution, which leaves ten checks unavailable. Diana
0.4.29 passes all 20 checks, including the original call prototypes on
both endpoints and the guarded exception ranges.
