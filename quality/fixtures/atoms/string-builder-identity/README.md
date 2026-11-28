# Builder result text and reference identity

The [Smali](BuilderIdentity.smali) determines the prepared DEX.
The independently authored [Java reference](BuilderIdentity.java) uses
the JDK builder and buffer implementations. The original class is
`Lfixture/BuilderIdentity;`.

Before each selected invocation, the observer interns the expected
nonempty text. The method then passes the actual builder result to
`observe(String)`. The observer records the text and whether the
result aliases the already interned string.

| Subject | Original locator | Original selected value |
| --- | --- | --- |
| `arithmeticBuffer()V` | `e0:l0:c0:m0` | Signed byte narrowing of 127 + 2 gives -127, appended as int to `number=` |
| `branchedString(Z)V` | `e0:l0:c0:m1` | The incoming branch chooses `left` or `right` after the `route=` seed |
| `dynamicSeed(I)V` | `e0:l0:c0:m2` | The incoming signed integer follows the nonempty `value=` seed |
| `literalBuilder()V` | `e0:l0:c0:m3` | Two String literals form `prefixsuffix` |
| `primitiveBuffer()V` | `e0:l0:c0:m5` | Int 37, char U+003A and boolean true form `37:true` |
| `singleObjectLiteral()V` | `e0:l0:c0:m6` | One Object append of the string literal `payload` |
| `staticString()V` | `e0:l0:c0:m7` | The encoded static final String `SUFFIX` follows the `field=` seed |

The raw witness decodes the original method and validates the exact
constructor, append, result, and observer sites. For the first method
it evaluates the effect-free arithmetic prefix from the decoded
operands at code units 0, 2 and 4 and replays the remaining instructions
with the computed value. For the last method it independently decodes
the DEX static-value table. The replay distinguishes original literal
references from builder results. The actual Java execution verifies
the library result identity for these nonempty values.

The original DEX witness and Java reference agree on all nine
[scenarios](scenarios.json). Diana 0.4.12 preserves all nine texts.
Its exact recovered Java aliases the existing interned string in the
two-literal, primitive, arithmetic/narrowing, and static constant field
cases. The single Object append and the runtime integer and branch
controls preserve the original distinct reference.
The 0.4.13 publication passes all nine checks. Direct literal chains
retain the builder calls. A concatenation that could otherwise become
a Java constant expression uses a nonconstant String operand and keeps
its runtime result.

The JVM companion [RuntimeConcat.java](RuntimeConcat.java) exercises
literal chains, equal branch values, JVM runtime concatenation, and
compile-time constants. The independent [check program](CheckRuntimeConcat.java)
compares text, identity against a literal, and identity across two calls.
The original and the recompiled 0.4.13 class publication pass all 20
checks. The equivalent 0.4.12 publication fails 10 identity checks.

The DEX was assembled with Apktool 2.11.1 and OpenJDK 21.0.12.1 using
`SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`.
Its SHA-256 is
`830ff548d89edd47817db4706714e05987fc03ecfe101ee20890526f22e0896a`.

```sh
diana get-method quality/fixtures/atoms/string-builder-identity/classes.dex \
  'Lfixture/BuilderIdentity;->arithmeticBuffer()V' --format json
python3 -B quality/decbench run --suite dex-builder-string-identity --binary /path/to/diana
cargo nextest run -p diana -E 'test(recovered_class_preserves_runtime_string_reference_identity)'
```
