# Two suspend sites with terminal cleanup

This project-authored input combines three coroutine states with a shared
catch-all cleanup path, normal cleanup, and failure checking at both resume
entries. It extends the coroutine fixture set beyond the
[closed total loop](../kotlin-suspend-state/README.md). Sources use the repository
license. The subject is
`Lfixture/SuspendFinally;->guarded(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;`.

Java/Kotlin explain the logical try/finally. Smali fixes its physical ABI:
marker returns bypass cleanup, while completed returns and throws execute cleanup.
Each protected resume check restores id before entering its interval. State two
also restores first. Frame-field restoration itself is outside these protected
intervals. The frame callback stores the result/failure, marks resumed entry,
and calls guarded with zero id; restored I$0 supplies the original cleanup key.

## Independent helper model and scenarios

`load(a)` records `load(a)` and normally yields boxed `2 * a`, immediately or
after suspension. `cleanup(id)` records cleanup and normally returns. Load can
fail immediately or through a resumed failure object. Cleanup can throw a
distinct error, which replaces a pending return or original failure. Valid
frames and successful frame-field/type resolution are model assumptions.
Scheduler execution, allocation failure, and external class resolution are not
modeled or claimed as executed.

| Scenario | Expected result and effect order |
|---|---|
| id 0; any immediate/resumed combination | `load(0), load(1), cleanup(0)`; return 2. |
| id 2; any immediate/resumed combination | `load(2), load(5), cleanup(2)`; return 14. |
| id -1 | `load(-1), load(-1), cleanup(-1)`; return -4. The repeated argument denotes two distinct sites. |
| First load fails immediately or on resume, id 2 | `load(2), cleanup(2)`; propagate original failure if cleanup returns. |
| Second load fails immediately or on resume, id 2 | `load(2), load(5), cleanup(2)`; propagate original failure if cleanup returns. |
| Either result is null | Original Number conversion/dereference fails; cleanup runs once with original id; no fabricated integer. |
| Cleanup throws after successful body | Arithmetic precedes cleanup; cleanup error replaces the return. |
| Cleanup throws during exceptional completion | The cleanup error replaces the original failure; no synthesized suppression or second cleanup. |
| Marker from either site | Return marker without cleanup; later completion reaches that site's tail/check and eventually performs one terminal cleanup. |
| Invalid adjusted label | Throw invalid-state error outside the cleanup intervals; do not invent cleanup. |

The original int result is captured before normal cleanup and boxed after it.
The handler's cleanup and rethrow are outside all protected intervals, so cleanup
failure cannot recursively reenter that same handler. A future recovery MUST
distinguish physical suspension returns from logical completed returns and keep
resumed failures inside the supplied protection intervals.

## Preparation

```sh
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/kotlin/suspend-finally-two-sites \
  -o .cache/fixtures/suspend-finally-two-sites/classes.dex
cp .cache/fixtures/suspend-finally-two-sites/classes.dex \
  quality/fixtures/kotlin/suspend-finally-two-sites/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/kotlin/suspend-finally-two-sites/classes.dex \
  --exec '{"cmd":"classes"}' --exec '{"cmd":"p9-all","text_only":true}'
```

Engine baseline: `64aaa04`. Expectations precede candidate-output inspection.

## Observed validation

Preparation used Smali 2.5.2, OpenJDK 21.0.12.1, API 24 and DEX 037; these are
observations rather than pins. Engine `dd70d20` delivers both classes and all
five methods without stops or rendering issues. It retains explicit protection
chains, id snapshots, marker returns, cleanup and original rethrow. Its shared
raw-selector fix repairs the baseline `64aaa04` default-row discrepancy.

Fixture revision `69d3b74` adds no production route. Four new fixture checks
and all 103 engine nextest tests passed. A 97-fixture dev comparison against
`dd70d20` found identical texts/issue lists and complete member availability.
Workspace format, all-target/all-feature Clippy and dev build passed. Protected
multi-site source folding remains unestablished. No release, helper/scheduler
execution, or final-output behavioral interpreter was run.
