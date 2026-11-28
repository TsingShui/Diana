# Crossed numeric overloads with a shared DEX register

The independently authored [Smali](CrossedNumerics.smali) defines the input.
The [Java reference](CrossedNumerics.java) uses equivalent recording overloads.
The original class is `Lfixture/CrossedNumerics;`. This challenge extends
the [scalar use fixture](../scalar-use-views/README.md) with another
argument position, virtual dispatch, and constructors.

The subject `run(CB)V` at `e0:l0:c0:m14` passes one char register to
both argument positions and one byte register to both argument positions.
The selected descriptors still differ. At DEX code units 2 and 19 the
constructors are `(CI)V` and `(IC)V`. The instance calls at 5 and 8
select `mix(CI)V` and `mix(IC)V`. At 11 and 14 they select
`octet(BI)V` and `octet(IB)V`.

Each original overload calls the recorder with its own identifying
literal and both actual argument values. [Scenarios](scenarios.json)
cover char U+FFFF, NUL and U+8000 and byte -128, -1 and 127.
Their integer uses preserve UTF-16 code units and signed byte values.
The control `control(CBI)V` at `e0:l0:c0:m4` intentionally selects
same-type formals. It includes 65536 as an int to detect an unintended
narrowing independently of the call identity.

The prepared input was assembled with Apktool 2.11.1 and OpenJDK
21.0.12.1 using `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo` (API 24). Its SHA-256 is
`9b814ffcaa50a1376f2b240cd73e7993dacda85871b767b590c41e7b4792ed0c`.

On Diana 0.4.6, the original DEX witness and the Java reference agree
on all four scenarios. The exact recovered Java from `run` fails all
three crossed scenarios and resolves to the wrong descriptor at all
six invocation sites. The same-type control passes its behavioral
scenario and all five call bindings.

Diana 0.4.7 passes all four behavioral scenarios and all eleven call
bindings. The integer view appears only at the positions selected by
the original DEX descriptors.

```sh
diana get-method classes.dex 'Lfixture/CrossedNumerics;->run(CB)V' --format json
python3 quality/decbench run --suite dex-crossed-numeric-overloads --binary /path/to/diana
```
