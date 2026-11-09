# Two suspend sites with a conditional second call

This project-authored input extends the coroutine fixture set beyond the
[closed total loop](../kotlin-suspend-state/README.md). It targets three dispatch
states, two distinct suspension results, a conditional early return between
them, and a saved scalar used after the second resume. Sources use the repository
license. No application code or third-party decompiler output was copied.

`Case.java` explains the normal run; `Case.kt` shows its logical suspend body.
Neither is compiled for preparation. `Case.smali` fixes the static continuation
ABI, Object boxing, marker returns, and original resume paths. `Frame.smali`
contains the callback: store the result, set the label high bit, and invoke
choose again with the same frame. The callback's zero seed is unused on resumed
paths. The subject is
`Lfixture/SuspendBranch;->choose(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;`.

## Independent helper model and scenarios

`load(a)` records `load(a)` and normally yields boxed `2 * a`, immediately or
after suspension. `record(v)` records its value. An injected failure throws at
the selected helper call. Completion calls the provided frame callback with the
boxed value or failure. Valid frames and successful frame-field/type resolution
are model assumptions; scheduler/runtime execution is not implemented.

| Scenario | Expected result and effect order |
|---|---|
| seed -1; immediate or resumed first load | `load(-1), record(-2)`; return -1; no second load. |
| seed 0; either site immediate/resumed | `load(0), record(0), load(1), record(2)`; return 2. |
| seed 2; either site immediate/resumed | `load(2), record(4), load(5), record(10)`; return 14. |
| First load fails immediately or upon resume | `load(seed)` throws before recording, branching, or another load. |
| First record fails | First load and record occur; second load and return do not. |
| Second load fails immediately or upon resume, seed 2 | `load(2), record(4), load(5)` throws; no record(10) or sum return. |
| Second record fails | Both loads and both records occur; return is skipped. |
| Marker from either site | Physical method returns the marker before that site's conversion/record; completion later reaches exactly that site's original tail. |
| Invalid adjusted label | Throw the original invalid-state exception/message before helper work. |

State one supplies the first result. State two restores the first int from I$0
and supplies the second result. These entries cannot be conflated. A future
source-body recovery MUST retain the negative branch and the first record before
the conditional second call. Current recovery status is recorded after inspection;
structural checks are not a helper-effect executor.

## Preparation

```sh
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/kotlin/suspend-two-step-branch \
  -o .cache/fixtures/suspend-two-step-branch/classes.dex
cp .cache/fixtures/suspend-two-step-branch/classes.dex \
  quality/fixtures/kotlin/suspend-two-step-branch/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/kotlin/suspend-two-step-branch/classes.dex \
  --exec '{"cmd":"classes"}' --exec '{"cmd":"p9-all","text_only":true}'
```

Engine baseline: `64aaa04`. Expectations above precede candidate-output inspection.
