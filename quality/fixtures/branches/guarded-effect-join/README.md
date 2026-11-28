# Guarded effect before a shared continuation

This independently authored fixture reduces the value placement problem
observed in two [AntennaPod methods](../../../reports/antennapod-3.12.3.md).
The original owner is `Lfixture/GuardedEffectJoin;`. Both
`run(ZZ)Ljava/lang/Object;` and `runWithCleanup(ZZ)Ljava/lang/Object;`
conditionally call `prepare`, forward the joined reference through a move,
call `publish`, and pass the earlier reference to `consume`.
The typed handler observes failures from the protected body.

The second method extends the mechanism with an outer cleanup covering the
body and typed recovery. Separate normal, recovered, and escaping paths
execute `cleanup` once. It also makes the later consumer or the typed
recovery throw an unmatched exception. The [scenario table](scenarios.json)
records the independent ordered traces and outputs for both methods.
Smali declares the helper methods native. The Java explanation supplies
recording helpers and drives the independently specified model.

## Preparation and observations

The DEX was assembled into `.cache/fixtures/guarded-effect-join/classes.dex`
with the Smali builder bundled in [Apktool 2.11.1](https://github.com/iBotPeaches/Apktool/releases/tag/v2.11.1)
on OpenJDK 21.0.12.1, API 24. The source launcher called
`SmaliBuilder.buildFile` with `DexBuilder(new Opcodes(24, -1))` and
wrote the result with `DexBuilder.writeTo`. The prepared input beside
the sources is the DEX used in both observations.

```sh
python3 quality/decbench prepare
python3 quality/decbench run --binary /path/to/baseline \
  --suite dex-guarded-effect-join --suite dex-guarded-source-order
python3 quality/decbench run --binary /path/to/candidate \
  --suite dex-guarded-effect-join --suite dex-guarded-source-order
```

Diana 0.4.2 at `49af4a17` moved `prepare` to the argument of `consume`,
after `publish`. Three of four direct scenarios and five of six cleanup
scenarios failed. The working-tree candidate at the same base revision
passed all ten and all eleven scenarios of the prior guarded source-order
suite. These outcomes compare execution in the authored Java model.
[Semantic review](review.md)
