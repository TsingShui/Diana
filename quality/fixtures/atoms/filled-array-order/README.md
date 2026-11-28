# Filled array allocation order

This independently authored fixture reduces the DEX ordering observed in
FairEmail 1.2338 `TupleFolderSort.getSortTitle(Context)`. The original calls
`getDisplayName` and reads the account name before `filled-new-array`.
Diana 0.4.9 placed both operations inside the recovered initializer. Java
allocates an array before evaluating its initializer expressions. The Smali
sources contain original material under the repository license.
[Dalvik bytecode](https://source.android.com/docs/core/runtime/dalvik-bytecode)
[Java array initializers](https://docs.oracle.com/javase/specs/jls/se21/html/jls-10.html#jls-10.6)

## Original behavior

| Original method | Locator | DEX operations |
| --- | --- | --- |
| `ArrayOrder.filled()[I` | `e0:l0:c0:m1` | Calls at `0x01` and `0x06`, then `filled-new-array` at `0x0a` |
| `ArrayOrder.stored()[I` | `e0:l0:c0:m6` | `new-array` at `0x01`, calls at `0x04` and `0x0c`, and stores at `0x09` and `0x11` |
| `ArrayOrderProtected.protectedFilled()[Ljava/lang/Object;` | `e0:l0:c1:m9` | Calls at `0x04` and `0x0b`, then `filled-new-array/range` at `0x0f`. Slots 0 and 2 reference the same original object. |
| `ArrayOrder.staged()[Ljava/lang/Object;` | `e0:l0:c0:m5` | Calls at `0x04` and `0x0b`, then `filled-new-array/range` at `0x13` and publication at `0x19` |

The complexity challenge `protectedFilled` shares a producer value between two
slots, evaluates a distinct second producer, and handles the two producer
faults in one typed catch with a stable stage value of 80. An allocation
failure carries `OutOfMemoryError` past that `RuntimeException` handler. The
`staged` control observes stage and references at every throwing operation and
checks the exact fault in its catch-all. `stored` demonstrates allocation
before elements and returns the same integer values as `filled`.

The four methods share 17 [independent scenarios](scenarios.json). DecBench
interprets the original instructions with bounded hooks and executes the
authored Java reference. For recovered Java it inserts the same allocation
hook at the array creation point and executes the remaining published method
with its original expressions, values and catches. A modeled allocation error
uses a particular exception instance and leaves the host memory budget intact.

## Preparation and replay

Apktool 2.11.1 and OpenJDK 21.0.12.1 prepared the DEX at API 24. Its SHA-256 is
`918453e82d6ceb2d17931770f3c3bbdde6e63f0b05ca149b81a24e97a0ee727a`.

```sh
fixture=quality/fixtures/atoms/filled-array-order
prepared=.cache/fixtures/filled-array-order
mkdir -p "$prepared"
java -cp .cache/decbench-repair/smali/apktool.jar \
  .cache/decbench-repair/smali/Assemble.java "$fixture" "$prepared/classes.dex" 24
cmp "$prepared/classes.dex" "$fixture/classes.dex"
python3 quality/decbench run --suite dex-filled-array-allocation-order --binary /path/to/diana
```

Diana 0.4.9 (`81925e1a`) failed all four `filled` scenarios and three of the
four `protectedFilled` scenarios. The 0.4.10 candidate passes all 17 behavior
scenarios. Both publish all four original methods without diagnostics.
Replacing the staged return with a clone of the published array triggers a
separate return-identity mismatch in the executable model.
[Semantic review](review.md)
