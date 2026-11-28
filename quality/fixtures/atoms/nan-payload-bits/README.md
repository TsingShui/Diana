# Quiet NaN payload bits

This independently authored fixture tests original DEX constants that carry
quiet NaNs. The [JVM case](jvm/) exercises the sibling input format. The
protected DEX consumer adds an observable write and a typed catch after
selecting a wide NaN.

| Original member in `Lfixture/ScalarNan;` | Locator | Independent observation |
| --- | --- | --- |
| `bits(Z)I` | `e0:l0:c0:m0` | The true path passes `7fc01234` to the raw bits call. The false path passes `ffc04321`. |
| `canonicalFloat()F` | `e0:l0:c0:m1` | The returned carrier is `7fc00000`. |
| `doubleValue()D` | `e0:l0:c0:m2` | The returned carrier is `fff8000000005678`. |
| `floatValue()F` | `e0:l0:c0:m3` | The returned carrier is `7fc01234`. |
| `protectedBits(ZLjava/lang/ArithmeticException;)J` | `e0:l0:c0:m5` | Choose `7ff8000000009abc` on true and `fff8000000005678` on false, then call the helper with both original arguments. |
| `store()V` | `e0:l0:c0:m6` | Write `fff8000000005678` to `stored:D` during the call. |

In the protected case, `inspectDouble(DLjava/lang/ArithmeticException;)J`
converts its first argument to raw bits and writes them to `lastBits:J`.
It returns that value if its second argument is null. Otherwise it throws
the exact supplied exception. The caller catches that exception at the
original typed handler, writes it to `caught`, and reads the raw bits already
written by the helper. The observer initializes `stored` to positive one,
`lastBits` to seven, and `caught` to null for each fresh process.

The cases use quiet NaNs throughout. No floating-point arithmetic follows
the literals. The observable operations are return, bit conversion, field
write and exception propagation. [The scenarios](scenarios.json) state the
expected hexadecimal bits and, where applicable, the observed write and
fault identity.

## Preparation and checks

Apktool 2.11.1's embedded Smali assembler generated this DEX at API 24
through the local `Assemble.java` adapter. The adapter accepts the Smali
directory, output filename, and API. The prepared DEX has SHA-256
`6728778c3fa2898342a61d0fdf55cdee697670d25afa761c53d446c92cf80614`.

```sh
fixture=quality/fixtures/atoms/nan-payload-bits
work=.cache/fixtures/nan-payload-bits
mkdir -p "$work"
java -cp "$APKTOOL_JAR" "$SMALI_ADAPTER" "$fixture" "$work/classes.dex" 24
cp "$work/classes.dex" "$fixture/classes.dex"
```

The bounded probe independently reads original instructions, original
owners and descriptors, the branch targets, selected literal words, and
the exact protected helper boundary. It models the original return and
store paths using integer containers for quiet NaN bits. It then compiles
the authored Java explanation and the exact Diana class publication in
separate scratch directories, and executes the same observer on both.

```sh
python3 -B quality/decbench run --binary /path/to/diana \
  --suite dex-nan-payload-bits
```

The authored Java and DEX evidence agree on all ten scenarios. The frozen
Diana 0.4.21 publication loses the original bits in nine cases. The canonical
NaN compatibility control passes. The repaired publication passes all ten
scenarios, including the original write and exact exception identity in both
protected fault cases. See the [semantic review](review.md).
