# JVM annotation default values

These sources are independently authored under the [repository license](../../../../../LICENSE).
[Defaults.java](Defaults.java) declares 16 defaults and one required element.
The fixture uses ordinary Javac output with the original `AnnotationDefault`
attributes on `LDefaults;` method definitions.
[Class-file attribute](https://docs.oracle.com/javase/specs/jvms/se25/html/jvms-4.html#jvms-4.7.22)

| Original elements | Observed value |
| --- | --- |
| `number()I`, `enabled()Z`, `byteValue()B`, `shortValue()S` | 37, true, -7, 300 |
| `letter()C`, `wide()J`, `floatValue()F`, `doubleValue()D` | Newline, 9876543210, negative float zero, 1.25 |
| `title()Ljava/lang/String;`, `token()Ljava/lang/Class;`, `choice()LChoice;` | Escaped text, `int[][].class`, `Choice.LAST` |
| `order()[I`, `empty()[Ljava/lang/String;` | `[2, 5, 2]`, empty array |
| `types()[Ljava/lang/Class;` (`j0:m13`) | `String`, `void`, `long[]`, `java.time.Instant` |
| `nested()LLabel;` (`j0:m14`), `children()[LLabel;` (`j0:m15`) | One nested annotation and an ordered array of two nested annotations |
| `required()Ljava/lang/String;` (`j0:m16`) | No default |

[The witness](CheckDefaults.java) reads the 17 declared elements through
`Method.getDefaultValue()` and observes a compiled annotation application
that omits every defaulted element. The verifier compiles it once and uses
it unchanged when it recompiles each exact Diana publication. It also
checks the raw xref for the default-only `Instant` class literal against the
byte offset independently decoded from the original input.

The [class-file fixture reader](classfile.py) derives four damaged inputs
from the same original bytes. A duplicate attribute and a trailing byte
both affect `number()I` (`j0:m0`). A third variant uses a valid long value
in that integer element's attribute. An invalid nested element-name pool
index affects `nested()LLabel;` (`j0:m14`). Each damaged attribute must report a
method-local issue. The recovered declaration keeps all independent defaults,
and the affected method has no published default. The unchanged witness
checks the remaining values after recompiling the exact available output.
The malformed nested default's xref still reports the independently located
type reference from its checked prefix.

On OpenJDK 21.0.12.1, the original matches all 18 observations. Frozen Diana
0.4.22 publishes the methods and reports all 16 defaults as unsupported. The
exact recompiled output passes only the missing-default control. The raw xref
already reports its original offset, and the damaged inputs also lose legal
sibling defaults. The repaired debug build passes all 18 ordinary observations
and isolates all four malformed cases. Both raw xref checks preserve their
original offsets. [Semantic review](review.md)

```sh
fixture=quality/fixtures/classes/annotation-default-values/jvm
python3 -B "$fixture/verify.py" --binary /path/to/diana \
  --output .cache/fixtures/annotation-default-values/jvm/report.json
cargo nextest run --locked -p diana -E 'test(=recovered_jvm_annotation_defaults_preserve_values_and_localize_damage)'
```

Use Python 3 and JDK 17 or later. The receipt preserves the original Javac
class, `javap` listing, damaged inputs, exact publications and runtime traces.
Omit `--binary` to verify just the original fixture.
