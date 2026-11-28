# JVM reads of prepared static final fields

The fixture is independently authored under the [repository license](../../../../../LICENSE).
[The generator](prepare.py) emits `LDefaultChild;` with `ObservableParent` as
its direct superclass and a forwarding constructor. `PREPARED_ZERO:I`,
`PREPARED_NULL:Ljava/lang/String;`, `PREPARED_BYTE:B` and `PREPARED_SHORT:S`
are static final fields with no `ConstantValue` attribute. The child has no
`<clinit>` or `putstatic`. `COMPILED_ZERO:I` and
`COMPILED_TEXT:Ljava/lang/String;` carry the encoded constant attributes.

The compiler produces the reader from [ordinary Java](ReadDefaultChild.java).
`LReadDefaultChild;->preparedZero()I` (`j0:m1`) and
`preparedNull()Ljava/lang/String;` (`j0:m2`) both execute `getstatic` at byte 0
and return at byte 3. The appended `preparedByte()B` (`j0:m5`) and
`preparedShort()S` (`j0:m6`) read the corresponding primitive fields. The
compiled constant reads use literals. The verifier checks the field attributes
and exact `getstatic` targets independently with `javap`.

[The scenarios](scenarios.json) start each run in a fresh JVM. The first
active read must run the parent initializer even though the declaring child
has no initializer body. The normal paths observe the parent effects before
the first field value, then check the two narrow fields return zero. A parent
failure must propagate its original cause through `ExceptionInInitializerError`.
The next active read must observe `NoClassDefFoundError`. The two compiled
constant values can be read before, after or without any initialization.

With OpenJDK 21.0.12.1, the original passes all five scenarios. Frozen
Diana 0.4.16 at `cd042a31` passes two. It publishes the implicit primitive
values as literal initializers and recompilation drops three `getstatic`
instructions. Diana 0.4.17 passes all five and preserves the four original
active reads and two `ConstantValue` attributes. Javac emits default-valued
assignments in the rebuilt child. The original bytecode and rebuilt bytecode
are recorded separately. [Semantic review](review.md)

```sh
fixture=quality/fixtures/classes/prepared-static-final/jvm
python3 -B "$fixture/verify.py" --binary /path/to/diana \
  --output .cache/fixtures/prepared-static-final/jvm/report.json
cargo nextest run --locked -p diana -E 'test(=recovered_jvm_prepared_static_finals_preserve_active_use)'
```

Python 3 and JDK 17 or later suffice. The verifier generates the child,
compiles its reader and unchanged host, and runs the original. It recompiles
the two exact Diana publications and runs them with the same host and runner
classes. The receipt records digests, `javap` output paths, runtime results
and exact publications. Omit `--binary` to run the original only.
