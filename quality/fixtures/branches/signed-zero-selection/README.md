# Signed zeros across a control join

This independently authored DEX challenge accompanies the
[JVM signed-zero selection case](jvm/). A branch selects equal numeric
zeros with different raw bits. The receiving call, static field write
or protected helper can observe the sign. This DEX fixture adds a float
return control and includes an original typed exception range around a
post-join helper.

| Original member in `Lfixture/SignedSelections;` | Locator | Independent case |
| --- | --- | --- |
| `rawFloat(Z)I` | `e0:l0:c0:m3` | True passes bits `80000000` to the conversion call. False passes zero. |
| `rawDouble(Z)J` | `e0:l0:c0:m2` | True passes zero. False passes bits `8000000000000000`. |
| `storeFloat(Z)V` | `e0:l0:c0:m6` | Write the same selected float to `staticFloat:F`. |
| `storeDouble(Z)V` | `e0:l0:c0:m5` | Write the same selected double to `staticDouble:D`. |
| `selectFloat(Z)F` | `e0:l0:c0:m4` | Return the selected float through separate physical exits. |
| `protectedUse(ZLjava/lang/ArithmeticException;)I` | `e0:l0:c0:m1` | Call the helper with the selected float and the same fault reference. |

The narrow join loads zero with `const/4` or negative-zero bits with
`const/high16`. The wide join uses `const-wide/16` and `const-wide/high16`
with the opposite branch layout. The bit conversion call and field write
consume the selected register after the join. The direct-return control
uses no joined consumer.

The protected range spans code units `[6, 10)` and starts with the call to
`inspectFloat(FLjava/lang/ArithmeticException;)I`. That helper records
the raw argument in `lastBits:I` before testing the fault. On success it
returns those bits. On failure it throws the supplied exception. The handler
starts at 11, records the same exception in `caught`, and reads the bits
written by the helper. The observer initializes both destination fields to
positive one, the helper result slot to seven, and the caught field to null.
Each scenario runs in a fresh process.

## Preparation and checks

Apktool 2.11.1's embedded Smali assembler used API 24 through the local
`Assemble.java` adapter. The following command records the invocation shape.
`APKTOOL_JAR` is a local jar and `SMALI_ADAPTER` is an adapter accepting the
Smali directory, output filename, and API.

```sh
fixture=quality/fixtures/branches/signed-zero-selection
work=.cache/fixtures/signed-zero-selection
mkdir -p "$work"
java -cp "$APKTOOL_JAR" "$SMALI_ADAPTER" "$fixture" "$work/classes.dex" 24
cp "$work/classes.dex" "$fixture/classes.dex"
```

The prepared input has SHA-256
`f25e5338bdcce0794546f501417b23ab629cc19108341f932856542001f86db8`.
Reassembly with the local tool produced the same bytes. The oracle checks
the original method and field identities, follows each input's DEX branches,
and models the raw-bit conversion calls without altering their bits. It
records the raw field writes and catch transfer. The fixed expectations in
[scenarios.json](scenarios.json) are also checked by executing the authored
Java source. A second run compiles the exact complete Diana class publication
and executes it with the same observer and inputs.

```sh
python3 -B quality/decbench run --binary /path/to/diana \
  --suite dex-signed-zero-selection
```

The independent DEX model and authored Java agree on all 14 cases. The
frozen Diana 0.4.20 publication fails six of those cases and passes eight.
The working-tree candidate passes all 14. See the
[semantic review](review.md) for the first differing observations and scope.
