# Three suspension sites with branch effects

This project-authored complexity challenge follows the acyclic source-body
repair discovered by [suspend-two-step-branch](../suspend-two-step-branch/README.md).
It retains state/result merges and adds a third suspension, two simultaneously
live captured ints, records after each result, and a second early-return branch.
It was authored after that repair and before inspecting candidate output.
Sources use the repository license. Java/Kotlin explain normal behavior; Smali
fixes the ABI, boxing, state paths and exact helpers. Each frame has its own
callback into its corresponding original method.

The subject is `Lfixture/SuspendThree;->walk(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;`.
`normalOnly` has the same prototype and a separate frame: only its immediate
first completion records 777 before the shared tail. This deliberate
normal/resume effect mismatch MUST remain explicit, rather than being folded
into the same source-body view.
`privateReturns` is a second positive challenge with the same logical behavior
but three distinct marker-return nodes. It exercises already selected P6 guard
controls rather than the shared-return raw branches. Its separate callback
reenters privateReturns. This interaction was added after repairing the original
compiled Kotlin fetch guard ownership.
`reorderedStates` adds sparse non-monotone states: 9 is the first call, 2 the
second and 5 the third. Its separate callback and private marker returns retain
walk's logical behavior and all independent scenarios. It was added after
repairing selected guard/case completion ownership; recovery cannot confuse
sorted state keys with source execution order or retain ABI case breaks.

## Independent model and scenarios

load(a) records its argument and normally yields boxed 2*a, immediately or after
suspension. record(v) records the value. A selected helper call can throw;
completion uses the provided frame callback. Valid frames and successful field
resolution are assumptions. Scheduling/helpers are not actually executed.

| Scenario | Independent result and effects |
|---|---|
| walk(-1), first site immediate/resumed | load(-1), record(-2), return -1; later sites are skipped. |
| walk(0), any completion combination | load(0), record(0), load(1), record(2), load(2), record(4); return 6. |
| walk(2), any completion combination | load(2), record(4), load(5), record(10), load(14), record(28); return 42. |
| Second load in walk(2) is modeled to yield 0 | load(2), record(4), load(5), record(0); return 4; no third load. This overrides that call's normal model so the zero branch is reachable. |
| Any load fails immediately or on resume | Earlier records remain; that site's record and all later work are skipped. |
| A record fails | Its result has been obtained; later loads and returns are skipped. |
| Either captured int is required at state 3 | Restore first from I$0 and second from I$1; the callback's zero seed cannot replace them. |
| normalOnly(0), first immediate | load(0), record(777), record(0), load(1), record(2), load(2), record(4); return 6. |
| normalOnly(0), first suspended/resumed | Same result, but no record(777). The extra effect is exclusive to immediate completion. |
| Marker or invalid state | Preserve the original marker return or invalid-state exception before its respective tail. |

The repair must distinguish each positive state, match the right result/capture
copies, preserve both branch effects, and reject the normal-only effect corridor.
Readability and structural checks are supporting evidence, not an implemented
behavior executor or a declaration that all coroutine inputs are supported.

## Preparation

```sh
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/kotlin/suspend-three-step-effects \
  -o .cache/fixtures/suspend-three-step-effects/classes.dex
cp .cache/fixtures/suspend-three-step-effects/classes.dex \
  quality/fixtures/kotlin/suspend-three-step-effects/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/kotlin/suspend-three-step-effects/classes.dex \
  --exec '{"cmd":"classes"}' --exec '{"cmd":"p9-all","text_only":true}'
```

Initial engine baseline is `6178bdd`. Assembly and actual review/check outcomes
are recorded after preparing the input once for both revisions.
