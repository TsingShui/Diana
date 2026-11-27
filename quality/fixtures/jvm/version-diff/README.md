# JVM method comparison

The discovery case is a pair of class files whose method loads the same string,
but the new constant-pool index requires `ldc_w`. The comparison should keep
the declaration and report its original method positions as unchanged. A class
file and a JAR use the same method comparison.

The challenge retains the pool renumbering and changes a conditional branch's
destination, the default of a lookup switch with sparse signed keys, the
priority of two overlapping exception ranges, generic method metadata and an
ordered bootstrap argument. A second load of the same dynamic constant is
replaced by a separate constant with equal bootstrap data. The physical
position of a class changes between the two JARs without changing its owner.
A Fabric bundle puts the paired class in a declared nested JAR. A damaged
archive adds one malformed sibling class and an unknown opcode after a known
changed prefix in another method. Two archives containing duplicate
definitions exercise unresolved linkage identities.

## Prepare and inspect

```sh
python3 quality/fixtures/jvm/version-diff/prepare.py .cache/fixtures/version-diff
java -m jdk.jdeps/com.sun.tools.javap.Main -v -p .cache/fixtures/version-diff/changed.class
java -m jdk.compiler/com.sun.tools.javac.Main -d .cache/fixtures/version-diff/run \
  quality/fixtures/jvm/version-diff/Bootstrap.java \
  quality/fixtures/jvm/version-diff/Verify.java
java -cp .cache/fixtures/version-diff/run Verify .cache/fixtures/version-diff
cargo nextest run --locked -p diana --test jvm_diff_test
```

The preparation script assembles ClassFile inputs with Python 3. The Java test
executes the independently authored bytecode and inspects reflected declarations
with OpenJDK 21.0.12.1. Run the ordinary Diana queries for the assembled class
files, root JARs and nested JARs. Generated class files and reports stay in
scratch space.
