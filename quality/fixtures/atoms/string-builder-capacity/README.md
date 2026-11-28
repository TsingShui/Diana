# String builder capacity and producer order

The independently authored [Smali](BuilderCapacity.smali) defines the
original DEX. The [Java reference](BuilderCapacity.java) exercises the JDK
`StringBuilder` and `StringBuffer` with the same ordered producer hooks.
The original owner is `Lfixture/BuilderCapacity;`. The three subjects
share descriptor `(II)Ljava/lang/String;`.

| Method | Locator | Original behavior under a negative capacity |
| --- | --- | --- |
| `builder` | `e0:l0:c0:m1` | `StringBuilder(int)` throws before either producer |
| `buffer` | `e0:l0:c0:m0` | `StringBuffer(int)` throws before either producer |
| `protectedBuilder` | `e0:l0:c0:m4` | The original fault enters the capacity catch |

The constructor call is at code unit 2 in each method. Calls at 6 and 15
produce the two values only after successful construction. Their values
are appended at 10 and 19 and the result is read at 23. The protected
interval [0,27) catches `NegativeArraySizeException` at 28 and
`IllegalStateException` at 36. The two producers can throw distinct
`IllegalStateException` objects or an unmatched
`IllegalArgumentException`. The latter escapes both handlers.

The [scenarios](scenarios.json) cover negative and zero capacity,
successful append, original fault identity from either producer, and
incompatible typed exception delivery. The DEX caller replay with modeled
library calls and the independently executed Java reference agree on all 13
cases. Diana 0.4.10 fails the three negative-capacity cases. Its recovered
methods execute the producers and can incorrectly enter the producer catch.
Diana 0.4.11 passes all 13 cases after restoring the capacity constructor in
each chain.

The input was assembled with Apktool 2.11.1 and OpenJDK 21.0.12.1 using
`SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`
(API 24). The prepared input SHA-256 is
`75b54922bb5440476ed596619bdc256f0c8c4210ad07a2e3fda1fb3d47fb4648`.

```sh
diana get-method quality/fixtures/atoms/string-builder-capacity/classes.dex \
  'Lfixture/BuilderCapacity;->protectedBuilder(II)Ljava/lang/String;' --format json
python3 -B quality/decbench run --suite dex-builder-capacity --binary /path/to/diana
```
