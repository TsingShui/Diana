# A throw moves across an unchanged try interval

This original Apache-2.0 fixture compares two versions of `Lguard/Conditional;->run(Ljava/lang/RuntimeException;I)Ljava/lang/Object;`. Each Java file models the observable branch and catch behavior of the accompanying Smali. The Smali also retains a shared string operand before the protected interval and an unreachable block between the two terminal blocks.

Both DEX files protect code units [3, 7) and deliver a RuntimeException to the handler at 10. The old conditional sends key zero to the throw at 6. The new conditional sends the same key to the throw at 9. Only the old throw is protected. Nonzero keys return the supplied object on both sides.

With `slug=protected-terminal-permutation`, use the [comparison fixture assembly procedure](../renamed-dispatch/#preparation-and-inspection). The checked-in inputs were assembled with Smali 3.0.9, OpenJDK 21.0.12.1 and API 24.

```sh
diana diff quality/fixtures/comparison/protected-terminal-permutation/before/classes.dex \
  quality/fixtures/comparison/protected-terminal-permutation/after/classes.dex --format json
cargo nextest run --locked -p diana --test diff_protection_test
```

The endpoint-only comparison classified the paired method as unchanged. The corrected comparison reports one changed exception site with old offset 6 and new offset 9. The [semantic review](review.md) records the path comparison. The [switch challenge](../switch-protection-permutation/) exchanges the protection of two throwing arms.
