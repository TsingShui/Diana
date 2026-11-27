# Removed argument with an independent literal edit

This original Apache-2.0 challenge extends [renamed dispatch](../renamed-dispatch/) with an optimization that changes a helper's declared prototype. The old helper ignores its second argument. The update removes that parameter, changes a literal and changes both caller instructions to pass one argument. The Smali inputs model these changes directly. This pair was assembled without running an optimizer.

## Independent expectations

- `Lx/u;->k(II)I` moves to `Ly/v;->r(I)I`. Both reverse the bits of the first argument, XOR with a literal, count the bits of that result and add the count back to the same result. The literal at offset 4 changes from 31 to 37. At first argument 0, the old return is 31 + 5 = 36 and the updated return is 37 + 3 = 40. Any second argument gives the same old result.
- In both releases `Lapp/Api;->first(I)I` applies Integer.highestOneBit to this helper's result. `Lapp/Api;->second(I)I` applies Integer.reverseBytes. Both old invokes at offset 1 pass the caller's parameter followed by 7. Both new invokes pass only the parameter. The old/new highest-one-bit returns at 0 are both 32, while the byte-reversed returns are 0x24000000 and 0x28000000. The new DEX intentionally retains a dead local assignment of 7 that has no observable Java effect.
- Recover the pairing while preserving the literal delta, the full prototype delta and the ordered argument difference in both callers. A match inferred solely from the same number of arguments cannot reach this case.

## Preparation and reproduction

Use the [shared assembly procedure](../renamed-dispatch/#preparation-and-inspection) with `slug=pruned-argument`. The Java sources explain the corresponding Smali. Smali 3.0.9, OpenJDK 21.0.12.1 and API 24 produced both prepared files.

```sh
diana diff quality/fixtures/comparison/pruned-argument/before/classes.dex \
  quality/fixtures/comparison/pruned-argument/after/classes.dex
cargo nextest run --locked -p diana --test diff_test -E 'test(pruned_argument)'
```

The comparison implementation on base 7be3f1e recovered both public caller pairs and their invoke changes, but left the uniquely corresponding helpers unmatched and omitted their literal delta. The [semantic review](review.md) records the original values and observable paths.

On the revised matcher based on 7be3f1e the CLI regression above passed on the same prepared inputs. The check executes the comparison and inspects the original member pairs and changed DEX sites.
