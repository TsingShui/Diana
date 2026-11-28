# Floating comparison negation at a joined terminal branch

The independently authored [DEX source](BranchFloat.smali) gives a float
and a double comparison two arms that share a physical return. The
larger arm calls the marker five times. The smaller arm calls it once
with 9. The direct-return controls use the same tests with separate
physical returns. The gated case adds a boolean input that can also
direct execution to the smaller arm.

| Original member in `Lfixture/BranchFloat;` | Locator | Original comparison |
| --- | --- | --- |
| `dFalls(DD)V` | `e0:l0:c0:m0` | `cmpl-double` and `if-lez`, separate returns |
| `dJoinFalls(DD)V` | `e0:l0:c0:m1` | `cmpl-double` and `if-lez`, shared return |
| `fFalls(FF)V` | `e0:l0:c0:m2` | `cmpg-float` and `if-gez`, separate returns |
| `fJoinBoolean(FFZ)V` | `e0:l0:c0:m3` | `cmpg-float`, `if-gez` and the gate |
| `fJoinFalls(FF)V` | `e0:l0:c0:m4` | `cmpg-float` and `if-gez`, shared return |
| `iJoinFalls(II)V` | `e0:l0:c0:m5` | Integer `if-ge`, shared return |

The float and double comparisons start at code unit 0. The first
conditional branch follows at 2. Unordered values yield +1 for
`cmpg-float` and -1 for `cmpl-double`, so both original branch tests
send NaN inputs to the smaller arm. The 16 [scenarios](scenarios.json)
include either unordered operand, ordered values, equality across the
two signed zeros, both gate outcomes and integer extrema.

The [oracle](verify.py) reads original method identities, register
operands, branch targets, compare polarity and the marker helper's
read, arithmetic and write. Its bounded DEX replay checks effect order on
the selected paths. The independently authored [Java reference](BranchFloat.java)
and [observer](CheckBranchFloat.java) match all 16 original traces.
The [default DecBench suite](decbench.json) recompiles the complete exact
Diana output with the same observer.

Optimized Diana 0.4.19 passes 29 checks and fails five. After tail
linearization, its joined float guard uses `>=` and its joined double
guard uses `<=`. Those guards are false on the unordered operands.
All five failures record 12345 where the original records 9. The
finite, integer and separate-return controls pass.
The patched engine retains the logical negation of each floating
comparison and passes all 34 checks. The same-input comparison records
five repairs with unchanged coverage and no new failures.

The DEX was assembled with APKTool 2.11.1 and API 24 through
`SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`. Its SHA-256 is
`a8fecb9c2df9b68f476756c64f61da1698e4e530a8538fdf8130a8bfe413b12c`.

```sh
mkdir -p .cache/decbench
fixture=quality/fixtures/branches/floating-condition-negation
PYTHONPATH=quality/decbench python3 -B "$fixture/verify.py" \
    --output .cache/decbench/floating-condition-original.json
python3 -B quality/decbench run --suite dex-floating-condition-negation \
    --binary /path/to/diana --output .cache/decbench/floating-condition.json
cargo nextest run -p diana-decomp --test floating_condition_negation_test
```
