# Encoded DEX constants and blank static finals

The independently authored [provider](DexProvider.smali),
[consumer](DexConsumer.smali), and [event hooks](DexInitEvents.smali)
determine the prepared DEX. The matching Java classes and
[observer](CheckInitialization.java) run each scenario in a fresh JVM.

| Owner | Locator | Original declaration and access |
| --- | --- | --- |
| `Lfixture/DexConsumer;` | `e0:l0:c0` | Literal controls and cross-class runtime getters |
| `Lfixture/DexProvider;` | `e0:l0:c2` | Two encoded static final values and two final fields assigned in `<clinit>` |

The original encoded values are String `compiled` and int 42.
The source-style compile-time getters carry `const-string` and
`const/16` directly. The two runtime getters instead execute
`sget-object` and `sget` at code unit 0. The provider initializer writes
String `runtime` at code unit 2 and int 7 at 6. It reads both values
at 8 and 10, passes them to the begin hook at 12, and calls the completion
hook at 15. Both runtime getters go through the same initialization.

The [scenarios](scenarios.json) exercise the literal-only control,
normal first use through either runtime field, and an initializer fault
on first use through either field. In the failure cases the first active
read receives an `ExceptionInInitializerError` whose cause is the exact
modeled hook exception. The second receives `NoClassDefFoundError`
without a second initialization. Subsequent literal controls still
return the interned literal and 42.

The original instruction witness separately decodes the encoded field
values and executes the bounded getter and initializer bodies using
modeled class state and hooks. The Java controls run the actual JVM
initializer and compare ordered event traces. Both agree on five cases.
The exact two-class reconstruction by the frozen 0.4.13-final executable
passes the literal-only control and fails all four active-use cases.
Its published provider initializes the two blank finals with source
constant initializers, enabling Java to inline the consumer accesses.

The DEX was assembled with Apktool 2.11.1 and API 24 using
`SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`.
Its SHA-256 is
`c1da143c2c45f8e00b44ed1e381d03bbe25893695c79d33e05e75c3b27355101`.

## DecBench

The [default manifest](decbench.json) selects the four consumer methods
with their physical original signatures. The behavior probe loads the
original consumer and provider from the same pinned DEX and compiles
their complete, exact class publications together with the unchanged
observer and modeled hooks. It checks the independent raw original and
the authored Java control before attributing a behavior difference.
The raw replay identifies the provider initializer as `e0:l0:c2:m6`.

| Check | Original consumer anchor |
| --- | --- |
| Literal-only control for both values | `e0:l0:c0:m1`, `compileText()Ljava/lang/String;` |
| Normal and failing first int read | `e0:l0:c0:m2`, `runtimeNumber()I` |
| Normal and failing first String read | `e0:l0:c0:m3`, `runtimeText()Ljava/lang/String;` |

The numeric literal getter at `e0:l0:c0:m0` has independent
method identity and publication checks. It participates in every
behavior scenario. A mismatched original or control yields
`SETUP_ERROR`. When complete class publication or execution cannot
be established, the affected behavior check yields `UNCHECKED`.

The original DEX replay and the Java reference agree on all five authored
traces. The 0.4.13 publication fails four active-use scenarios. The 0.4.14
publication retains both assignments in the static initializer and passes
all five scenarios.

The [JVM companion](jvm/RuntimeFields.java) tests the same effects with
actual class files, including the two original `ConstantValue` attributes.
Its [expected observations](jvm/scenarios.json) agree with the original and
the recompiled 0.4.14 publications.

Run the independent class and paired suite from the repository root.

```sh
python3 -B quality/decbench run --suite dex-blank-static-finals --binary /path/to/diana
cargo nextest run -p diana -E 'test(recovered_classes_preserve_runtime_static_final_initialization)'
```
