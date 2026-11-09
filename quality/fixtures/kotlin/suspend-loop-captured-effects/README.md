# Captured effects across a suspend loop

This independently authored fixture is a complexity challenge for
[`kotlin-suspend-state`](../kotlin-suspend-state/Case.kt), specifically `Case.total`.
The author retains the interior continuation reentry and adds an independently
updated checksum, a captured expression, and effectful preparation and recording
on both sides of suspension. The repair requires equal saved/resumed and normal
merge values, without moving preparation past state publication. Sources are
project-authored under the repository license; no application code was copied.

The subject is `Lfixture/SuspendLoop;->total(IILkotlin/coroutines/Continuation;)Ljava/lang/Object;`.
Resolve it by its original descriptor/prototype, not an assumed method table index.
`Case.java` explains the ordinary run. Smali fixes the continuation ABI and is the
input authority. The Java explanation does not expose its Object boxing, marker
return, fresh/resumed frame selection, or invalid-state exception.

## Independent scenarios and helper model

The native helpers have this explicit model. `prepare(i)` records `prepare(i)`
and returns `i + 1`. `record(v)` records its value and normally returns.
`load(a, continuation)` records `load(a)` and normally produces boxed `2 * a`,
either immediately or after returning the suspended marker and later receiving
that same result. An injected helper failure throws `IllegalStateException` at
the selected call. A resumed failure passes through `ResultKt.throwOnFailure`
before executing the shared tail. There is no implemented helper/runtime executor
for these scenarios; the model supports LLM semantic review.
Resumption in this model stores the result in the same frame, sets its label
high bit, and calls the selected original method again. The minimal Frame input
supplies storage and construction; it is not an executed coroutine scheduler.

| Inputs and route | Independent result and ordered effects |
|---|---|
| `limit = 0, bias = 7` or `limit = -2, bias = 7` | Return 7; no helper calls. |
| `limit = 1, bias = 2`, immediate or resumed | `prepare(0), record(2), load(1), record(4)`; return 6. |
| `limit = 3, bias = 2`, any mixture of immediate/resumed calls | `prepare(0), record(2), load(1), record(4), prepare(1), record(2), load(2), record(10), prepare(2), record(3), load(3), record(19)`; return 24. |
| `limit = 2, bias = -1` | `prepare(0), record(-1), load(1), record(1), prepare(1), record(-1), load(2), record(4)`; return 4. |
| `limit = 3, bias = 2`, load argument 2 fails immediately or upon resumption | The first iteration completes; then `prepare(1), record(2), load(2)` throws. No `record(10)`, index update, or next preparation occurs. |
| Preparation or pre-load record fails | No state publication or load occurs after that failure. |
| Post-load record fails | Sum/checksum calculation precedes failure; index update and next preparation do not occur. |
| ABI label other than 0/1 after entry adjustment | Throw the original invalid-state exception/message before initial/resumed body work. |

Prepared variants in `Case.smali` are explicit recovery boundaries:
`wrongSpill` restores the index field as the captured value; its resumed result
differs from the normal route. `changedCapture` increments the captured value
after saving it; immediate/resumed results differ by one per affected iteration.
`resumeEffect` records the index before checking the resumed failure; it has an
extra resumed-only effect. These methods MUST retain explicit ABI control; they
cannot use the inferred source-body presentation of `total`.

## Preparation and evidence

Assemble once, then inspect the same DEX on the baseline and candidate:

```sh
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/kotlin/suspend-loop-captured-effects \
  -o .cache/fixtures/suspend-loop-captured-effects/classes.dex
cp .cache/fixtures/suspend-loop-captured-effects/classes.dex \
  quality/fixtures/kotlin/suspend-loop-captured-effects/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/kotlin/suspend-loop-captured-effects/classes.dex \
  --exec '{"cmd":"classes"}'
```

Preparation used Smali 2.5.2, OpenJDK 21.0.12.1, and API 24; these are observed
versions, not admission pins. The prepared input has class definition 1 for
SuspendLoop, and MethodId 6 for total. The original descriptors above remain the
portable selectors. Initial baseline was `7354242`; integration baseline was
`8785d2f`. Candidate engine revision was `4c6a3d1`.

The same prepared DEX changes from the baseline's explicit multi-entry graph to
the candidate's while body for total. All three divergent methods remain
explicit. Three new fixture tests and the full 94-test engine nextest run passed.
The 92-fixture dev comparison changed only the discovery fixture and this
challenge, with no increased stops, missing members, or rendering issues. All
other fixture texts/issue lists were identical to integration baseline.
Workspace format, all-target/all-feature Clippy, and dev build passed. The CLI
text and JSON publications agreed for the discovery method. No release run or
final-plan behavioral execution was performed.
