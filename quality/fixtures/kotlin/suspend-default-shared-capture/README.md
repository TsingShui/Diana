# Shared capture and an effectful dispatch default

This project-authored complexity challenge follows the repair discovered by
[`suspend-two-step-branch`](../suspend-two-step-branch/README.md). The same
unowned switch dispatch has a default body placed after its printed rows.
This challenge adds a live seed captured across both suspension sites, together
with an observable invalid-state effect before the original exception. It was
authored after the dispatch-spelling repair, before inspecting its output.
Sources use the repository license.

The subject is
`Lfixture/SuspendCapturedDefault;->choose(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;`.
Java/Kotlin explain normal logical behavior. Smali also defines an invalid-state
path absent from those explanations: record(-99), then construct/throw the
original invalid-state error. The input callback stores result and the label
high bit before calling choose with zero seed. Both resumes restore the actual
seed from I$1; state two also restores first from I$0.

## Independent helper model and scenarios

The helper model matches the parent: load(a) records its argument and yields
boxed 2*a, immediately or after suspension; record(v) records the value.
Selected calls can throw. Valid frames and successful frame-field/type resolution
are assumptions. No scheduler/helper executor is implemented.

| Scenario | Expected result and effects |
|---|---|
| Seed -1; first completion immediate/resumed | load(-1), record(-2), return -1; no second load. |
| Seed 0; any site completion route | load(0), record(0), load(1), record(2), return 2. |
| Seed 2; any site completion route | load(2), record(4), load(5), record(10), return 16. The original seed contributes after the second result. |
| First load failure | Propagate before first record or another load. |
| Second load failure, seed 2 | load(2), record(4), load(5), then propagate; no record(10) or result. |
| Adjusted label 7 | record(-99), then throw the invalid-state error; no load. |
| Default record fails | record(-99) fails before exception allocation/initialization; preserve that failure. |
| Physical marker return | Skip that site's conversion and subsequent work until the callback reenters the matching tail. |

The repair requires every dispatch row to reach its delivered original target
or scope. A default row cannot silently enter case zero, skip record(-99), or
execute logical helper work. The original selected-edge copies and live seed
must remain intact. Snapshot or helper-call counts alone do not establish this
behavior; review must follow the labeled output paths.

## Preparation

```sh
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/kotlin/suspend-default-shared-capture \
  -o .cache/fixtures/suspend-default-shared-capture/classes.dex
cp .cache/fixtures/suspend-default-shared-capture/classes.dex \
  quality/fixtures/kotlin/suspend-default-shared-capture/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/kotlin/suspend-default-shared-capture/classes.dex \
  --exec '{"cmd":"classes"}' --exec '{"cmd":"p9-all","text_only":true}'
```

Engine baseline is `64aaa04`. Compare this same prepared DEX with the dispatch
repair candidate and record the review and executed checks separately.
