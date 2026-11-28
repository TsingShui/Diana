# Static initializer field owners

Each scenario selects an original class that shares a simple name with a
class in a second package. Both classes define `public static int value`.
The original initializer writes the second class and leaves its own field
at the default value. These cases isolate owner identity independently of
the blank-static-final semantics in the companion fixture.

| Scenario | Indexed initializer | Original ordered effect | Expected observable values |
| --- | --- | --- | --- |
| Prefix | `e0:l0:c2:m3::Lorigin/PrefixOwner;-><clinit>()V` | Store 7 to `Loutside/PrefixOwner;->value:I` at code unit 1 | own 0, outside 7 |
| After an effect | `e0:l0:c1:m1::Lorigin/LateOwner;-><clinit>()V` | Run the begin hook, then store 5 to `Loutside/LateOwner;->value:I` at code unit 4 | begin sees outside 0, own 0, outside 5 |

The raw original checks verify each initializer and observer method's
descriptor and locator. They decode both field reference tables and
match the full owner, name, type, register, and instruction order. The
observer is at `e0:l0:c2:m4` for PrefixOwner and `e0:l0:c1:m2` for
LateOwner. The immutable original Java classes independently run both
scenarios. The recovered control replaces all four classes with exact,
complete class publications from the same supplied DEX. The hook and
observer remain the same in both Java executions.

Both cases fail on Diana 0.4.13.
PrefixOwner publishes `value = 7` in the original owning class's field
declaration. LateOwner retains the begin call and writes `value = 5` to
its own field in its initializer. Both outside values remain zero.
The 0.4.14 publication retains the fully qualified field references in
their original order, and both scenarios pass.

Assemble the [authored instructions](smali) with APKTool 2.11.1 and API
24. The prepared `classes.dex` SHA-256 is
`2231dc8ac6e9c5eb6504cce61cf15b44068047c9824fffffba34e87216ab3d1b`.

From the repository root, run the default [fixture suite](decbench.json)
against a Diana executable.

```sh
python3 -B quality/decbench run --suite dex-static-initializer-field-owners --binary /path/to/diana
```
