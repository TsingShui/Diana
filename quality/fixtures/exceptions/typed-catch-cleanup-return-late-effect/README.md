# Typed catch cleanup return with a post-close effect

This independently authored challenge extends the
[return-lane fixture](../typed-catch-cleanup-return/). The original member is
`Lfixture/ReturnLanes;->lookup(Ljava/lang/Object;Ljava/lang/RuntimeException;Ljava/lang/RuntimeException;Ljava/lang/RuntimeException;Ljava/lang/RuntimeException;)Ljava/lang/Object;`
at `e0:l0:c0:m1`.

On a nonempty normal path, a potentially throwing effect 6 runs after
cursor close and before owner close. Its scope includes typed recovery
and owner cleanup. Its scope follows cursor close and the body suppression
handler. A fault in effect 6 returns null after owner close, with no second
cursor close. Earlier body and cursor-close faults skip effect 6. An
owner-close fault replaces the pending return from either path.

The [scenario expectations](scenarios.json) preserve exact return and fault
identities, effect order, suppression, and monitor release. The former layout
and authored source each matched all 14 scenarios. In the tracked reordered
layout, Diana 0.4.29 leaks the captured item in six scenarios. See the
[semantic review](review.md) for the path comparison.

This DEX was assembled with Apktool 2.11.1's embedded Smali at API 24, using
the same procedure as the base fixture. Run the challenge directly.

```sh
work=.cache/fixtures/typed-catch-cleanup-return-late-effect
python3 -B quality/decbench run --binary target/debug/diana \
  --suite dex-typed-catch-cleanup-return-late-effect --output "$work/decbench.json"
```

Diana 0.4.30 passes all 14 scenarios on the tracked layout.
