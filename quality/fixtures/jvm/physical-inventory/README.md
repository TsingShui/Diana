# Physical JVM definitions and recoverable archive entries

The ClassFile and archives are independently authored under the repository's
license. `Main.java` explains the base class. The standard-library Python
recipe writes the original ClassFile structures and instructions so every
position used by the regression is independent of a Java compiler. The
versioned definition follows the same code and uses a different string value.

Run the recipe from the repository root. It requires Python 3 and writes only to
the specified scratch directory.

```sh
python3 quality/fixtures/jvm/physical-inventory/prepare.py \
  .cache/fixtures/jvm-physical-inventory
cargo nextest run --locked -p diana --test jvm_query_test --test jvm_outline_test
```

## Inputs and expected facts

- `Main.class` declares `Lfixture/jvm/Main;`, extends `Lfixture/jvm/Base;`,
  and implements `Lfixture/jvm/Marker;`. `forward(I)I` and `forward(J)J`
  are separate declared methods. The latter returns its original 64-bit
  argument. `forward(I)I` branches to return zero when its argument is zero.
  Otherwise its instruction at byte offset 5 calls `Lfixture/jvm/Helper;->twice(I)I`
  and returns that result. The method at ordinal 3 loads the string at
  constant-pool index 8 and returns it. The pool also contains a reference to
  `Helper.unused()V` with no corresponding instruction.
- `inventory.jar` contains the base class at `j1` and the same declared
  class in the Java 17 view at `j5`. Their string constants contain
  `root` and `overlay` respectively, followed by a NUL code unit and
  U+1F331. `j4` declares `Lfixture/jvm/Helper;` under the unrelated entry
  name `misleading/NamedFromPath.class`. The archive includes an interface
  method with no `Code` attribute and both stored and deflated class entries.
- `damaged.jar` adds an incomplete constant-pool record at `j7` and a
  bad ZIP CRC at `j8`. Both entries are independently unavailable.
  `Lfixture/jvm/After;` at `j9` and the five earlier definitions remain
  available. The published result identifies the two failed coordinates.
- `bundle.jar` names a nested JAR in `META-INF/versions.list`. Its class is
  `Lfixture/jvm/Nested;` at `j1/j0`. The same bytes are present as an
  undeclared archive resource and contribute no class. The outer definition
  remains independently located at `j2`.
- `DamagedMember.class` uses a class constant in place of the method name
  at ordinal 1. The directory still identifies the other method rows
  exactly. The diagnostic identifies `j0:m1`.
- `DamagedCode.class` appends the undefined opcode `0xcb` to
  `j0:m1` at byte offset 11. The method declaration and its instruction
  prefix remain readable. Both the listing and reference query preserve
  the reference at offset 5 and disclose the unavailable range `11..12`.
- `damaged-bundle.jar` adds an invalid catalog row alongside the valid
  nested JAR entry. The catalog issue belongs to `j0`. The reachable
  outer and nested definitions remain available.
- `Lfixture/jvm/Nested;->probe(I)I` calls
  `Lfixture/jvm/Outer;->increment(I)I` at byte offset 1. Both methods
  preserve the original descriptors and their separate archive coordinates.
  An outline with depth 0 reports the call without following it. With depth
  1 it reaches the outer archive's `j2:m0`, which contains no further calls.
  With the damaged catalog the same target remains a candidate and the catalog
  diagnostic remains visible.
- The constructor at `j1:m0` calls `fixture/jvm/Base.<init>()V` at offset 1.
  The declaration at `j2:m0` in turn calls `java/lang/Object.<init>()V` at
  offset 1. The outline records this last symbol without a target locator
  because the archive contains no definition of `java/lang/Object`.

- `Dynamic.class` has a `ldc_w` at `j0:m0` and `invokedynamic` at `j0:m1`,
  each at byte offset 0. Their bootstrap rows both reach the `seed` method
  handle through a nested dynamic constant. The load's own row also takes
  the string `nested-bootstrap-note` directly. Each site reaches the nested
  row once, even when that row is supplied twice to the same call.
- `DamagedDynamic.class` puts an ordinary method constant in the bootstrap
  method slot for the load. The load's arguments and the invokedynamic
  site's references remain independently visible. The error covers bytes
  `0..3` of `j0:m0`.
