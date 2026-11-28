# Generic invocation overload binding

These independently authored sources use ordinary Javac targeting JVM 17.
The prepared JAR contains the two recovered classes. The two helper classes
and observers stay on the compiler and runtime classpaths, outside Diana's
input. In particular, the bytecode contains an erased producer descriptor
and the exact chosen overload.

| Original method in the prepared JAR | Original consumer invocation |
| --- | --- |
| `LGenericOperand;-><clinit>()V` (`j0:m1`) | `LGenericEffects;->select(Ljava/lang/Object;)Ljava/lang/String;` |
| `LGenericArrayOperand;->object(I)Ljava/lang/String;` (`j1:m1`) | `LGenericArrayEffects;->choose(ILjava/lang/Object;)Ljava/lang/String;` |
| `LGenericArrayOperand;->array(I)Ljava/lang/String;` (`j1:m2`) | `LGenericArrayEffects;->choose(I[Ljava/lang/String;)Ljava/lang/String;` |

The initializer captures a generic producer in an `Object` local. The
observer expects the original object overload, its return marker, and
the trace `object/12`. The challenge adds an effectful leading argument
and a nested argument inside the generic producer. The trace records those
calls as 1 and 2, the producer as 3, the object overload as 4, and the array
overload as 5. The two consumers check argument identity. The observers
distinguish the exact supplied exceptions by object identity.

| Mode | Original `object` result and trace | Original `array` result and trace |
| --- | --- | --- |
| 0 | object-null / 1234 | array-null / 1235 |
| 1 | object-value / 1234 | array-value / 1235 |
| 2 | nested-fault / 12 | nested-fault / 12 |
| 3 | capture-fault / 123 | capture-fault / 123 |
| 4 | leading-fault / 1 | leading-fault / 1 |
| 5 | leading-fault / 1 | leading-fault / 1 |
| 6 | object-scalar / 1234 | cast-fault / 123 |

Mode 5 requests both the leading and nested faults. The leading argument
prevents evaluation of the nested call. The array control has an original
`checkcast [Ljava/lang/String;`, which throws before the consumer in mode 6.

The verifier checks the original invoke descriptors and array cast with
`javap`, then recompiles both the class and selected-method publications.
For the selected initializer, the oracle supplies the original `RESULT`
declaration that the method-only endpoint omits, and uses the initializer
body exactly as published. The other selected methods recompile directly.
The verifier checks the selected consumer descriptor on both endpoints
and runs the unchanged observers on the recompiled full classes.

```sh
fixture=quality/fixtures/jvm/generic-overload-binding
work=.cache/fixtures/generic-overload-binding
python3 -B "$fixture/verify.py" --binary target/debug/diana --output "$work/proof.json"
python3 -B quality/decbench prepare --prepared "$work/prepared"
python3 -B quality/decbench run --prepared "$work/prepared" \
  --binary target/debug/diana --suite jvm-generic-overload-binding \
  --output "$work/decbench.json"
```

The original classes pass all 15 runtime scenarios. Diana 0.4.27 emits
the wrong argument context on both endpoints for the initializer and
the challenge's object overload. The default suite reports 22 passes and
eight definite failures, including four changed runtime outcomes and four
changed consumer descriptors. The originally array-typed control passes.

Diana 0.4.28 passes all 21 standalone checks. The two publication
endpoints recompile to the original consumer descriptors in all three
methods, and all 15 runtime scenarios match.
