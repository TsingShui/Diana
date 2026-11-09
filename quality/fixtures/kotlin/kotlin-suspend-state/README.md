# Suspend state discovery

`Case.kt` and its existing prepared `classes.dex` are the discovery input.
This iteration selects `Lfixture/Case;->total(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;`.
Class definition 3 and engine MethodId 11 identify that member in this prepared
DEX; use descriptor/prototype resolution for another build.

The Kotlin source supplies the independent normal behavior. Nonpositive limits
return boxed zero without load; limit 1 loads index 0 and returns zero; limit 4
loads indexes 0, 1, 2, 3 in order and returns 12. Sum is updated before index.
The original helper returns boxed value * 2 for nonnegative arguments and throws
`IllegalStateException("negative")` otherwise. The total loop's nonnegative
indexes do not reach that negative helper branch.

The DEX retains label zero initial entry and label one resume entry into the
loop tail. The resumed frame restores limit/index/captured sum and checks failure
before conversion or update. The baseline selected an irreducible loop graph.
Acceptance for this readability repair is a recovered `while` body and one
implicit-continuation load call, while retaining the Java-style ABI signature,
boxing, state/error paths, identities and uncertainty. The signature adds the
inferred `suspend` modifier. The newly authored
[captured-effect fixture](../suspend-loop-captured-effects/README.md) challenges
the same mechanism; its negative methods retain explicit paths.

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/kotlin/kotlin-suspend-state/classes.dex \
  --exec '{"cmd":"p6","class":3,"method":11,"details":true}' \
  --exec '{"cmd":"p9","class":3,"method":11}'
cargo nextest run -p diana-decomp --test suspend_loop_bodies --locked
```

Baseline: `73542421d671be2fc169b72c7233aa0b5b955fd5`. The candidate uses the same
original DEX and the ordinary Engine chain. `review.md` records path reasoning;
structural tests are supporting evidence. No implemented final-plan behavioral
executor or coroutine scheduling execution is claimed. The existing DEX was
reused without reassembly; this iteration did not reestablish its old compiler
environment.
