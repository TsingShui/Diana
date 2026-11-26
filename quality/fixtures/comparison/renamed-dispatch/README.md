# Renamed dispatch

Two independently assembled, original Apache-2.0 inputs model a release with a renamed package, class and method set. Each Java file explains the accompanying Smali class. The before and after DEX files occupy separate coordinate spaces. No mapping file is supplied to Diana.

## Independent expectations

- The helper `Llast/w;->v(II)I` corresponds to `Lnext/h;->j(II)I`. Both rotate the first integer left by the second and XOR the result with 53. The local register changes from v0 to v2. Both versions have two callers.
- The old caller `Llast/c;->u(II)I` and new caller `Lnext/m;->r(II)I` call their corresponding rotate-and-XOR helpers with the original argument order, then return Integer.highestOneBit of the result. At (4, 1) both return 32. After aligning the renamed helper, this caller has no changed operation.
- `Llast/c;->s(II)I` corresponds to `Lnext/m;->f(II)I`. With (1, 2), the old result is 17 and the new result is 19. With (2, 2), the old caller computes (2 rotated left by 2) XOR 53 = 61. The new caller returns 19 without calling its helper. With (4, 1), the old helper receives (4, 1) and returns 61. The new helper receives (1, 4) and returns 37. These examples distinguish the literal, condition and argument changes independently.
- `Llast/c;->x(I)I` and `Llast/c;->y(I)I` have identical bodies and no callers. The same holds for `Lnext/m;->b(I)I` and `Lnext/m;->d(I)I`. Each returns its argument XOR 82. Either pairing preserves all available evidence. A table index, position or short name cannot establish their individual correspondences.
- Two unrelated forwards have the same opcode arrangement. `Llast/c;->n(I)I` calls Integer.reverse. `Lnext/m;->p(I)I` calls Integer.bitCount. At 1 they return Integer.MIN_VALUE and 1 respectively. Their shape supplies no evidence for identical content or common provenance.

## Preparation and inspection

Prepare the inputs from the repository root. Set SMALI_CLASSPATH to the locally installed assembler classpath. This pair was assembled with Smali 3.0.9, OpenJDK 21.0.12.1 and API 24.

```sh
slug=renamed-dispatch
for side in before after; do
  src="quality/fixtures/comparison/$slug/$side"
  out=".cache/fixtures/$slug/$side"
  mkdir -p "$out" &&
    java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble --api 24 -o "$out/classes.dex" "$src" &&
    cp "$out/classes.dex" "$src/classes.dex" || exit 1
  diana methods "$src/classes.dex" --no-limit
done
diana diff quality/fixtures/comparison/renamed-dispatch/before/classes.dex \
  quality/fixtures/comparison/renamed-dispatch/after/classes.dex --format json
```

The baseline command at 7be3f1e rejected the diff subcommand. The candidate is the working-tree DEX comparison implementation based on that revision. [Semantic review](review.md) records inspection of the two assembled inputs. The [challenge](../handler-inline-challenge/) adds inlining and a changed recovery path. The CLI regression also compares the existing [algorithm input](../../algorithms/algo-dp/) to its [R8 build and independently recorded method mapping](../../obfuscated/obf-dp-r8/).

The [multidex recipe](../../export/multidex-resources/) is also exercised by the diff CLI regression. The complete and partial archives share both the arithmetic root and the independent branch root. Comparing those archives retains the two intact method sets and reports each unreadable root on the new side. The truncated header and bad ZIP CRC remain distinguishable from a successful empty image.

Run the paired CLI regressions from the repository root.

```sh
cargo nextest run --locked -p diana --test diff_test
```

On the candidate based on 7be3f1e, the moved-method, changed-dispatch, duplicate-body and partial-archive assertions passed. The candidate get-class outputs for all four class versions in this pair exactly matched the reviewed baseline outputs. The comparisons execute the CLI and inspect its published facts.

The full six-test diff target passed on the candidate, together with the export and recovery CLI regression targets (16 tests in that run). A previous candidate missed the two primitive-array algorithms in the R8 case and admitted a root with a bad ZIP CRC in the partial archive. The same tests exposed both omissions before the fixes and passed afterward. The baseline at 7be3f1e rejected all six diff requests as an unrecognized command.
