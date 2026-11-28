# JVM NaN payload bits and constant preparation

The fixture is independently authored under the [repository license](../../../../../LICENSE).
[The generator](prepare.py) emits two standalone JVM classes with explicitly
encoded float and double quiet NaN bits. `LNaNWork;` contains executable
methods. `LNaNChild;` declares two NaN fields with `ConstantValue` attributes
and no `<clinit>`.

| Original member | Original operations and expected bits |
| --- | --- |
| `LNaNWork;->floatNaN()F` (`j0:m0`) | `ldc` then `freturn`, `7fc01234` |
| `LNaNWork;->doubleNaN()D` (`j0:m1`) | `ldc2_w` then `dreturn`, `fff8000000005678` |
| `LNaNWork;->selectFloatBits(Z)I` (`j0:m2`) | `ldc` of `7fc01234` on true or `ffc05678` on false, then `Float.floatToRawIntBits` |
| `LNaNWork;->selectDoubleBits(Z)J` (`j0:m3`) | `ldc2_w` of `fff8000000005678` on true or `7ff8000000004321` on false, then `Double.doubleToRawLongBits` |
| `LNaNWork;->negativeZero()F` (`j0:m4`) | `ldc` negative zero, `80000000` |
| `LNaNWork;->positiveInfinity()D` (`j0:m5`) | `ldc2_w` positive infinity, `7ff0000000000000` |
| `LNaNChild;->activate()I` (`j0:m0`) | Returns 3 after active use of the class |

`NaNChild.F_NAN:F` and `NaNChild.D_NAN:D` share the bits from the first two
methods. The unchanged `ObservingParent` reads both fields recursively while
its own initializer runs. Both reads observe the assigned payloads because
JVM `ConstantValue` assignment precedes superclass initialization. The
[ordinary Java control](runtime-fields/NaNChild.java) with runtime bit
conversion at field initialization has the same payloads after the child
initializes. Its parent observes prepared zeros instead.
[JVMS initialization order](https://docs.oracle.com/javase/specs/jvms/se25/html/jvms-5.html#jvms-5.5)

The verifier recompiles the exact recovered `NaNWork` and runs the same
compiled [witness](CheckNaNPayloads.java) used on the original. The
`NaNChild` declaration contract uses the exact `float-bits` and `double-bits`
values tagged `JVM ConstantValue`. These are Java-like representations of
preparation facts. The verifier checks their original field identities, raw bits
and absence of a recovered class initializer directly in the publication.
`javap` and JVM execution separately establish the original and runtime-control
field attributes, payload bits and timing.

With OpenJDK 21.0.12.1, the original passes all eight method cases and the
field preparation checks. Frozen Diana 0.4.21 collapses all six custom NaN
method values to canonical NaN and passes the two controls. It also drops
the raw payload from both prepared field declarations. The repaired debug
publication passes all eight method cases and both field display checks.
[Semantic review](review.md)

```sh
fixture=quality/fixtures/atoms/nan-payload-bits/jvm
python3 -B "$fixture/verify.py" --binary /path/to/diana \
  --output .cache/fixtures/nan-payload-bits/jvm/report.json
cargo nextest run --locked -p diana -E 'test(=recovered_jvm_nan_literals_preserve_payloads_and_constant_preparation)'
```

Use Python 3 and JDK 17 or later. Omit `--binary` to run only the original
classes and the runtime field control. The receipt records digests, `javap`
listings, exact Diana publications, execution traces and field display checks.
