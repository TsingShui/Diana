# JVM signed zero selection

These sources are independently authored under the [repository license](../../../../../LICENSE).
[The input](SignedSelections.java) selects opposite zero signs of the same
primitive type. [The witness](CheckSignedSelections.java) checks results and
stored values with `Float.floatToRawIntBits` and `Double.doubleToRawLongBits`.
The owner is `LSignedSelections;`.

| Original member | Selected value and use |
| --- | --- |
| `rawFloat(Z)I` (`j0:m3`) | Signed float zero passed to `floatToRawIntBits` |
| `rawDouble(Z)J` (`j0:m4`) | Signed double zero in the reverse arm order passed to `doubleToRawLongBits` |
| `store(Z)V` (`j0:m5`) | Both static fields updated with their selected signs |
| `nestedDouble(ZZ)J` (`j0:m6`) | Two nested selectors, with positive zero only for outer true and inner false |
| `guardedStore([FIZ)I` (`j0:m7`) | Signed float zero stored and read back inside a protected array operation |

The original bytecode uses `ldc` of the float `-0.0f` or `fconst_0`, and
`ldc2_w` of the double `-0.0d` or `dconst_0`. The protected store joins both
float arms before `fastore` at byte 16. Its normal trace is 12. An invalid
index must return 301, preserve both input cells and record trace 13. A
null array must return 302 and record trace 14. These fault checks run for
both selector outcomes. The controls cover selections returned directly,
same-sign alternatives, one effectful selector evaluation, and four
encoded field constants. The witness reads the fields by reflection.

With OpenJDK 21.0.12.1, the original passes all 25 checks. Frozen Diana
0.4.20 passes 18 after recompilation of the exact complete publication.
The selected argument, store and nested cases lose one of the signed zero
results. The direct return, fault trace and encoded field controls
pass. The repaired debug build passes all 25 checks, and the rebuilt
fields retain the four original `ConstantValue` signs. [Semantic review](review.md)

```sh
fixture=quality/fixtures/branches/signed-zero-selection/jvm
python3 -B "$fixture/verify.py" --binary /path/to/diana \
  --output .cache/fixtures/signed-zero-selection/jvm/report.json
cargo nextest run --locked -p diana -E 'test(=recovered_jvm_signed_zero_selections_preserve_value_bits)'
```

Use Python 3 and JDK 17 or later. The verifier compiles the witness once,
runs the original class and recompiles the exact Diana publication. It
checks all four `ConstantValue` signs with `javap` before and after, and
retains the bytecode listings, exact publication, digests and trace results.
Omit `--binary` to run the original control alone.
