# JVM record object methods

`javac` emits a shared ObjectMethods bootstrap row for the generated
`equals`, `hashCode` and `toString` methods. Diana retains each original
operation, its runtime inputs and the ordered component field handles.
The operation's library algorithm remains symbolic. [ObjectMethods bootstrap](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/lang/runtime/ObjectMethods.html)

The challenge combines primitive, floating-point, nullable reference and
array components. An empty record exercises the empty component name list.
Variant class files change the bootstrap name string or invalidate a getter's
constant-pool reference while leaving the original `Record` component table
and the independent `incremented` method available.

```sh
python3 quality/fixtures/jvm/record-bootstrap/prepare.py .cache/fixtures/record-bootstrap
java -cp .cache/fixtures/record-bootstrap/classes fixture.record.Verify
java -m jdk.jdeps/com.sun.tools.javap.Main -v -p \
  .cache/fixtures/record-bootstrap/classes/fixture/record/RecordProbe.class
cargo nextest run --locked -p diana --test jvm_record_test
```

The fixture uses OpenJDK 21.0.12.1 targeting Java 17. The JVM checks execute
the generated original classes. Their recovered Java-like text contains
explicit record operations and is checked separately through the CLI.
