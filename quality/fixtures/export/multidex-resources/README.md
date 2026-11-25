# Multidex resource export

This archive fixture extends the bare [arithmetic DEX](../../atoms/expr-arith/)
with a [second root](../../branches/branch-ifz/), a compiled manifest and stored
and deflated resources. The assembly recipe and manifest bytes are original
Apache-2.0 test material. It reuses the prepared DEX files without rebuilding
their Java or Smali sources. The baseline revision is `eac4bb4`. The candidate
is the Diana 0.3.0 export implementation. The archive was prepared with
Python 3.13.14. The CLI test uses its standard library to assemble the input in a private test directory.

## Independent expectations

- Bare input has one defined class, `Lfixture/ExprArith;`. Its file belongs at
  `sources/fixture/ExprArith.java`. The path also identifies this definition in
  the complete archive. Its arithmetic operations and exceptions have the
  semantics of the original Java source.
- The complete archive adds `Lfixture/BranchIfz;` to that package.
  `bothPaths(0)` sets state to 3 and returns 1. A nonzero argument sets
  state to 4 and returns 0. The nested DEX under `assets/` is an opaque
  resource. Its three class definitions do not enter the source directory.
- `AndroidManifest.xml` has one manifest element with no attributes or children.
  The exported tree contains its readable XML and the original binary bytes.
  `resources.arsc` is deliberately opaque. The exported raw payload repeats
  all 256 byte values 2,048 times followed by the three bytes `end`. The eight-byte
  test library in `lib/arm64-v8a/` retains all its bytes, including NUL and 0xff.
- The partial archive adds the [direct-initialization case](../../classes/direct-initialization/)
  together with a truncated root, a bzip2 resource, and incorrect ZIP CRCs on
  a stored asset and an otherwise valid DEX root. The broken constructor method has a local diagnostic.
  The direct call on the allocated Child and the ordinary Child construction
  survive in the same class. Both other complete roots and every intact
  supported resource survive.
  The report identifies both unavailable roots and the two unavailable resources.
- The duplicate archive adds the [direct constructor fixture](../../classes/direct-initialization/)
  and its [effectful variant](../../exceptions/direct-initialization-effects/).
  Both define `Lfixture/Parent;` and `Lfixture/Child;`. Only the second Parent
  records event 2 after its Object constructor and before its value store. A
  failure in that event prevents the store. The Child constructors both store
  99 after their Parent returns. Both distinct Parent bodies and both Child
  definitions retain their original coordinates and files despite collisions.
- The nested archive adds the [static member class](../../classes/class-inner-static/).
  Its two definitions occupy separate files. The outer factory constructs one
  Holder using the argument, and `useHolder` invokes the factory followed by
  `doubled`. Holder initializes its field, doubles the value with Java int
  overflow and returns the updated field. A construction failure prevents the
  call. Only the Holder file contains the doubled method body.
- The unsafe-path archive includes a relative path above the resource directory.
  The export preserves entry 6 at a safe alternate location, the other
  resources at their original paths and the destination's sibling sentinel.
- An existing destination with user files rejects publication. Those files
  retain their original contents. Two processes exporting different bare inputs
  to the same new directory produce one winning tree. Its sole Java file
  matches the successful input, and both staging directories are cleaned up.

Run the CLI regression from the repository root:

```sh
cargo nextest run --locked -p diana --test export_test
```

For manual inspection, create an archive at a new path:

```sh
mkdir -p .cache/fixtures/multidex-resources
python3 quality/fixtures/export/multidex-resources/prepare.py \
  .cache/fixtures/multidex-resources/input.apk complete
```
