# JVM archive export

`Edition` computes twice the input plus one in the base definition and three
times the input plus seven in the versioned definition. `Helper` squares its
wide input. The outer file has an intentionally inaccurate entry name for the
base class. That name identifies an archive record; the checked class bytes
supply the class's binary name and source package.

```sh
python3 quality/fixtures/jvm/archive-export/prepare.py \
  .cache/fixtures/jvm-archive-export
cargo nextest run --locked -p diana --test jvm_export_test
```

The recipe uses Python 3 and the OpenJDK compiler module for Java 17 class
files. Generated binaries remain in the scratch directory.

The archive contains the base and variant definitions at `j1` and `j2`.
A catalog declares the JAR at `j4`, whose `j0` is `Helper`. A complete export
publishes the variant by its original coordinate and derives the base class
path from its binary name. The manifest, service file, native library and
large nested resource retain their original bytes. The last resource exercises
streaming reads across transfer chunks. The partial variant adds a malformed
class at `j6` and a resource at `j7` whose name cannot form a portable output
path. The run must retain all other definitions, diagnose `j6`, and relocate
`j7` inside the output tree.
