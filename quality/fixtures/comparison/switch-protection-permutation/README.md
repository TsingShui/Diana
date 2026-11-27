# Sparse switch arms exchange protection

This original Apache-2.0 challenge extends the [conditional protection fixture](../protected-terminal-permutation/) with a sparse switch, two distinct thrown values and an ordered typed and catch-all handler chain. Each version contains `Lguard/Switches;->dispatch(Ljava/lang/RuntimeException;Ljava/lang/RuntimeException;I)I` and `preserved` with the same prototype.

Key -3 throws the first argument. Key 17 throws the second. Other keys return zero. In the old dispatch, only the first throw is protected. In the new dispatch, only the second is protected. A protected IllegalArgumentException selects -3. Other protected failures select -5. Each unprotected throw escapes. Both versions of the control method `preserved` protect both throws in the same clause order.

With `slug=switch-protection-permutation`, use the [assembly procedure](../renamed-dispatch/#preparation-and-inspection). Smali 3.0.9, OpenJDK 21.0.12.1 and API 24 produced the checked-in inputs.

```sh
diana diff quality/fixtures/comparison/switch-protection-permutation/before/classes.dex \
  quality/fixtures/comparison/switch-protection-permutation/after/classes.dex --format json
cargo nextest run --locked -p diana --test diff_protection_test
```

The endpoint-only comparison classified both methods as unchanged. The corrected result locates two coverage changes in dispatch and retains the unchanged control. The [review](review.md) compares the original routes and recovered output.
