# Reordered branch blocks

This original Apache-2.0 pair extends [renamed dispatch](../renamed-dispatch/) with code motion. The same secondary conditional, two distinctive arithmetic blocks and shared return occur in a different byte order in the updated DEX. Each conditional has an explicit goto on its fallthrough path. The Java files explain the selected computations and the Smali fixes physical placement. The source exposes two original members in Llayout/Route; in both inputs.

## Independent expectations

- `Llayout/Route;->stable(II)I` returns the value minus three for a nonpositive mode or a zero value. Otherwise it multiplies the value by three. With (0, 5), (1, 0) and (1, 5) both versions return 2, -3 and 15, respectively. All operations on integer values have DEX 32-bit semantics. Relocation preserves every successor and produces no instruction change.
- `Llayout/Route;->redirect(II)I` retains the same two conditions and the same two arithmetic arms. In the old input a nonpositive mode goes directly to XOR 13. In the update that edge goes to AND 7. With (0, 5) the old method returns 8 and the updated method returns 5. For positive mode both versions use secondary dispatch: (1, 0) returns 13 and (1, 10) returns 2. Both arithmetic arms remain reachable. The sole change is the taken successor of if-lez at offset 0, from the old XOR at code-unit offset 6 to the new AND at offset 3.

## Preparation and inspection

Use the [parent assembly procedure](../renamed-dispatch/#preparation-and-inspection) with `slug=reordered-branches`. The checked-in inputs were assembled using Smali 3.0.9, OpenJDK 21.0.12.1 and API 24.

```sh
diana diff quality/fixtures/comparison/reordered-branches/before/classes.dex \
  quality/fixtures/comparison/reordered-branches/after/classes.dex
cargo nextest run --locked -p diana --test diff_test -E 'test(block_relocation)'
```

The comparison candidate based on 7be3f1e with ordered instruction alignment reported eleven changes in each method. It classified stable as changed, and obscured redirect's single changed edge with layout differences. The [semantic review](review.md) covers the ordinary get-class results and original control flow.

With block alignment on the same base revision, all seven tests in `cargo nextest run --locked -p diana --test diff_test` passed. The candidate reported stable as unchanged and identified a single instruction delta in redirect. The two prepared DEX files were held fixed across the failing and passing comparisons.
