# DEX array-data fill effects

This independently authored fixture exercises `fill-array-data` with
a primitive array, a callback producing the destination, and a protected
path that reads the original destination after a failed fill. ART checks
the destination for null, then checks the full payload count against its
length, then copies the payload bytes into the existing array.
[ART interpreter](https://android.googlesource.com/platform/art/+/9dae5b4/runtime/interpreter/interpreter_switch_impl.cc)

All selected members belong to `Lfixture/FillData;`. Offsets in the
table are DEX code units.

| Original member | Locator | Fill and payload | Original obligation |
| --- | --- | --- | --- |
| `fillEmpty([I)V` | `e0:l0:c0:m0` | Fill at 0, payload at 4 | Check the receiver even when the element count is zero |
| `fillFloat([F)V` | `e0:l0:c0:m1` | Fill at 0, payload at 4 | Copy `1.5f` and negative zero, retaining the tail |
| `fillFloatBits([F)V` | `e0:l0:c0:m2` | Fill at 0, payload at 4 | Preserve both signed-zero bit patterns and quiet NaN payload `0x7fc12345` |
| `fillFromReceiver()V` | `e0:l0:c0:m3` | Fill at 4, payload at 8 | Evaluate the receiver once and fill the aliased original array |
| `fillTwo([I)V` | `e0:l0:c0:m4` | Fill at 0, payload at 4 | Reject a short target before any store |
| `protectedFill([ILjava/lang/ArithmeticException;)I` | `e0:l0:c0:m5` | Fill at 4, payload at 20 | Catch bounds at the fill and read the initial cell; retain an earlier receiver fault by identity |

The [bounded DEX witness](raw_proof.py) decodes the exact payload widths,
counts and raw values, the receiver producer and the selected catch
entries. It models the null and extent checks before bulk copy. In
the protected short case the bounds failure at code unit 4 is caught
before the `aget` at 11 sees the original value 7. A supplied receiver
exception reaches a different typed catch, preserves object identity,
and prevents the fill. The reference [Java samples](FillData.java)
independently check this contract with primitive bulk copies and an
explicit null-sensitive empty case.

The [default suite](decbench.json) recompiles the entire exact class
publication with the unchanged [observer](CheckFillData.java). It
checks [14 authored scenarios](scenarios.json) against the raw replay
and actual Java observations. The raw replay models the original DEX
instructions and the listed faults. Its results cover the selected
payloads and handlers.

Frozen Diana 0.4.18 passes 26 checks and fails six. It repeats receiver
evaluation, writes an array cell before detecting insufficient length,
drops the null check when the payload is empty, and renders the
noncanonical NaN as `Float.NaN`. In the protected case the bounds
handler observes a partial write instead of the original cell. The
typed signed-zero, normal prefix, receiver-exception and other
compatibility controls preserve their original results.
The patched engine passes all 32 checks, including all 14 executed
scenarios on the same DEX. The recovered short fill preserves the
original cell and the typed payload retains its original IEEE bits.

The DEX was assembled with APKTool 2.11.1 at API 24 using
`SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`. Its SHA-256 is
`7085797575f054fe80f32a9f4b00f9ae63bddb58fb1e64e91c29f07936253510`.

```sh
mkdir -p .cache/decbench
fixture=quality/fixtures/atoms/fill-array-data-contract
PYTHONPATH=quality/decbench python3 -B "$fixture/raw_proof.py" \
    --output .cache/decbench/fill-array-data-original.json
python3 -B quality/decbench run --suite dex-fill-array-data-contract \
    --binary /path/to/diana --output .cache/decbench/fill-array-data.json
cargo nextest run -p diana-decomp --test fill_array_data_contract_test
```
