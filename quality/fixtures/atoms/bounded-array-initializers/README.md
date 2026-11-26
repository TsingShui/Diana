# Bounded array initializers

This fixture was written for Diana and uses the repository license. It extends
the allocation and indexed stores in [array-basic](../array-basic/) with a
dimension much larger than the method and a read from the array being filled.

The selector is `Lfixture/LargeArray;`. The Java explanation and Smali have the
same allocations, write order and returned array identities. The Smali spells
out consecutive stores for all three methods.

- `partial(I)[I` allocates an array of length 2147483647. If allocation succeeds,
  it stores the argument at index zero and returns the original array. Diana
  can recover that code from the supplied instructions using bounded memory.
- `complete(II)[I` fills both elements with the two arguments, in parameter
  order. Both stores qualify for an array initializer.
- `dependent(I)[I` stores the argument at index zero, reads that element and
  writes it to index one. The allocation must remain available to that read.

The fixture was assembled with Smali 3.0.9 on OpenJDK 21.0.12.1. From the
repository root, with Smali and its dependencies in the shared tool cache, run

```sh
out=.cache/fixtures/bounded-array-initializers
src=quality/fixtures/atoms/bounded-array-initializers
mkdir -p "$out"
java -cp '.cache/fixture-tools/smali/*' com.android.tools.smali.smali.Main assemble "$src/LargeArray.smali" -o "$out/classes.dex" &&
    cp "$out/classes.dex" "$src/classes.dex"
```

The integration check calls `Engine::class_source`. On Linux its subprocess
uses a 512 MiB address-space limit to make an excessive reservation observable.
