# Inlined arithmetic with a changed handler

This original Apache-2.0 challenge extends [renamed dispatch](../renamed-dispatch/) by moving part of a method into its caller and changing which failures the caller catches. The Java and Smali pairs give each input its own implementation.

## Independent expectations

The entry in both versions is `Ledge/Gate;->evaluate(I[II)I`. The old helper `Ledge/Kernel;->mix(I)I` multiplies its input by seven, XORs with 49, subtracts three and reverses its bytes. The caller invokes it before the try region. The new entry contains that instruction sequence directly before the array read. There is no new Kernel definition. A possible fragment relationship and the exact Gate correspondence can coexist. An inline candidate needs source and destination ranges to identify which part moved. A common subsequence alone cannot establish the history that produced it.

For a first argument of 0, the mixed value is 0x2e000000. A divisor array containing 2 at index 0 gives 0x17000000 in both versions. A divisor of 0 reaches the arithmetic handler and returns -1 in both. A null array throws NullPointerException before the division. An invalid index into a non-null array throws ArrayIndexOutOfBoundsException. Those exceptions escape the old method and return -1 from the new method. The handler target returns the same sentinel on both sides. The changed catch type determines the new paths.

A second class recycles two full short signatures. The old `Ledge/Alias;->a(I)I` corresponds to the new `Ledge/Alias;->b(I)I`: it calls Integer.reverse and XORs the result with 31. The old b corresponds to the new a: it calls Integer.bitCount and adds 5. For the input 1 their respective results are 0x8000001f and 6. Method spelling alone gives the opposite pairing.

## Preparation and inspection

Use the [parent assembly procedure](../renamed-dispatch/#preparation-and-inspection) with `slug=handler-inline-challenge`. Smali 3.0.9, OpenJDK 21.0.12.1 and API 24 assembled the two checked-in inputs. Baseline and candidate revisions are the same as the parent. Run the CLI comparison from the repository root.

```sh
diana diff quality/fixtures/comparison/handler-inline-challenge/before/classes.dex \
  quality/fixtures/comparison/handler-inline-challenge/after/classes.dex --format json
```

The [semantic review](review.md) records the normal and exceptional paths and the scope of the observations.

Run the paired CLI regressions from the repository root.

```sh
cargo nextest run --locked -p diana --test diff_test
```

On the candidate based on 7be3f1e, the inline/handler and recycled-signature tests both passed. The candidate get-class outputs for all five class versions in this pair exactly matched the reviewed baseline outputs. These checks execute the CLI and inspect its results.
