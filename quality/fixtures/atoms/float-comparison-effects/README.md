# Floating comparisons and operand evaluation

This independently authored fixture extends the initial FloatCompare
reproduction with both DEX comparison polarities and both floating
widths. Each selected comparison invokes the left producer exactly
once and then invokes the right producer exactly once. The producers
record digits 1 and 2 respectively in the public trace. The throwing
right producer records its digit and throws the exact caller-supplied
exception before the comparison or its consumer can run.

All selected method identities belong to `Lfixture/FloatCompare;`.

| Original locator | Method | Original operation and use |
| --- | --- | --- |
| `e0:l0:c0:m0` | `doubleFinite()Z` | Finite `cmpg-double` compared to +1 |
| `e0:l0:c0:m1` | `doubleHigh(Ljava/lang/RuntimeException;)I` | NaN `cmpg-double` returned as +1 |
| `e0:l0:c0:m3` | `doubleLow(Ljava/lang/RuntimeException;)Z` | NaN `cmpl-double` compared to -1 |
| `e0:l0:c0:m7` | `lowFloatZero(Ljava/lang/RuntimeException;)Z` | NaN `cmpl-float` tested below zero |
| `e0:l0:c0:m8` | `nanSkips(Ljava/lang/RuntimeException;)Z` | NaN `cmpg-float` compared to +1 |
| `e0:l0:c0:m9` | `one()Z` | Finite `cmpg-float` compared to +1 |
| `e0:l0:c0:m12` | `value()I` | Finite `cmpg-float` returned as +1 |
| `e0:l0:c0:m13` | `zero()Z` | Finite `cmpg-float` tested above zero |

The [original replay](raw_proof.py) resolves the indexed definitions
and decodes producer calls, scalar and paired registers, exact encoded
IEEE values, trace field writes, comparison opcodes, branches and
throws. A successful path runs the actual indexed comparison. A fault
path reaches the original right producer and propagates the live
exception without reaching that comparison. All 12 authored
[scenarios](scenarios.json) expect trace `12`.

The [default DecBench probe](decbench.json) recompiles the exact full
class publication with the unchanged [Java observer](CheckFloat.java).
The independently authored Java control captures both operands before
comparing them. The raw original and Java control agree on all 12
scenarios, including the throwable identity checks.

Frozen Diana 0.4.15 passes 30 checks and fails six. The finite float
and double comparisons against +1 report `121212`. The selected NaN
float-high and double-low predicates report `1` because they skip the
right operand. Their fault scenarios return normally and omit the
original throw. Predicate-zero and materialized-result controls
preserve the original traces. The repaired engine passes all 36
checks, including all 12 executed scenarios on the same prepared DEX.

The [Smali input](FloatCompare.smali) was assembled with APKTool
2.11.1 and API 24 using `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`. The prepared
[DEX](classes.dex) SHA-256 is
`863f63955fe40eecc6709108530f0c9b23310f263b6263823d6a9b81d6668529`.

From the repository root, run the original and the candidate suite.

```sh
mkdir -p .cache/decbench
fixture=quality/fixtures/atoms/float-comparison-effects
PYTHONPATH=quality/decbench python3 -B "$fixture/raw_proof.py" \
    --output .cache/decbench/float-cmp-original.json
PYTHONPATH=quality/decbench python3 -B "$fixture/verify.py" \
    --output .cache/decbench/float-cmp-reference.json
python3 -B quality/decbench run --suite dex-floating-compare-operands \
    --binary /path/to/diana --output .cache/decbench/float-cmp-candidate.json
cargo nextest run -p diana-decomp --test floating_comparison_effects_test
```
