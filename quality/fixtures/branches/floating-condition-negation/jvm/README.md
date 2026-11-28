# JVM unordered branch negation

These sources are independently authored under the [repository license](../../../../../LICENSE).
Javac compiles the asymmetric `if` and `else` arms to a shared terminal
return. The large arm appends digits 1 through 5 to an integer trace. The
small arm appends digit 9. The `gated` variant also calls `open(gate)` to
record digit 8 on the ordered left-less-than-right path. The separate
return control uses two physical returns for the same comparison.

| Original member | Initial branch |
| --- | --- |
| `LFloatingBranches;->floatLow(FF)V` (`j0:m2`) | `fcmpg` then `ifge` |
| `LFloatingBranches;->floatHigh(FF)V` (`j0:m3`) | `fcmpl` then `ifle` |
| `LFloatingBranches;->doubleLow(DD)V` (`j0:m4`) | `dcmpg` then `ifge` |
| `LFloatingBranches;->doubleHigh(DD)V` (`j0:m5`) | `dcmpl` then `ifle` |
| `LFloatingBranches;->gated(FFZ)V` (`j0:m7`) | `fcmpg` then `ifge`, followed by `open` on fallthrough |
| `LFloatingBranches;->separateReturns(FF)V` (`j0:m8`) | `fcmpg` then `ifge`, with two returns |
| `LFloatingBranches;->integralLow(II)V` (`j0:m9`) | `if_icmpge` with a shared return |
| `LFloatingBranches;->selfEqual(D)I` (`j0:m10`) | Two loads of the same double, `dcmpl` then `ifne` |

[The independent witness](CheckFloatingBranches.java) resets the trace before
all 47 scenarios. For float and double it checks both finite orderings, equal
operands, and NaN on the left, right or both sides. Unordered and equal
values select the small arm. For the gated comparison, `open` is called
only when the left operand is less than the right. The two gate values
then distinguish traces `812345` and `89`. The small arm still records only
`9` on every unordered input. Integral controls use equal operands and both
orderings of the minimum and maximum signed integers.
The self-equality case returns 9 for a finite double and 1 for NaN.

With OpenJDK 21.0.12.1 the original passes 47 scenarios. Frozen Diana 0.4.19
passes 28 after recompiling the exact full publication. It recovers `>=` as
the complement of float or double `<`, and `<=` as the complement of `>`.
Both comparisons are false on NaN, so the branch falls through and takes
an incorrect trace. In `gated`, this also executes the otherwise skipped
`open` call. In `selfEqual`, it returns 9 unconditionally.
The repair's debug build passes all 47 scenarios.
[Semantic review](review.md)

```sh
fixture=quality/fixtures/branches/floating-condition-negation/jvm
python3 -B "$fixture/verify.py" --binary /path/to/diana \
  --output .cache/fixtures/floating-condition-negation/jvm/report.json
cargo nextest run --locked -p diana -E 'test(=recovered_jvm_floating_condition_negation_preserves_unordered_paths)'
```

Use Python 3 and JDK 17 or later. The verifier checks original comparison
opcodes, short-arm targets and terminal join shape with `javap`. It compiles
the witness once, runs the original class, and then executes the exact Diana
publication recompiled with the same JDK. The receipt records scenario results,
raw bytecode listing, original class digest and exact publication. Omit
`--binary` to verify only the original class.
