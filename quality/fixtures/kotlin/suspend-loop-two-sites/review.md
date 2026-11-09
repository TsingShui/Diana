# Two-site loop semantic review

This LLM review compares authored Java/Kotlin, Smali and Engine output using the
README model. The current Engine selects one `while` over the original head
predicate; entry/reentry, resume, marker and invalid-state code remain ABI
attachments. Structural checks and the fixture test are supporting evidence
only; no helper, scheduler or final-plan interpreter was executed.

Initial route. The ABI prefix keeps the frame instance check, high-bit label
test, frame creation and `result`/`marker`/`label` reads. The source body
starts at the original case-zero arm: `v22 = p0` (limit), `v29 = 0` (sum),
`v30 = 0` (index), `v33 = v22`. The head then prints as
`while (v30 < v33 [exit: v37 = v29;])`, preserving the original
`if-ge v6, v4, :finished` operands. The exit copy publishes the current sum for
the shared exit tail, whose original `invoke-static Boxing.boxInt` and
`return-object` become `return Boxing.boxInt(v37)`.

Site one. The three state stores (`I$0` limit, `I$1` sum, `I$2` index) and
`label = 1` remain ordered ABI comments before `v23 = SuspendTwoSites.load(v30)`.
The continuation argument is implicit and marked as an inference. The marker
comparison keeps both original destinations, and the normal copies
`v43 = v23; v41 = v30; v37 = v29` deliver the same values as the resumed route.
`v24 = (v43 as Number?).intValue()` is the original cast and conversion, then
`record(v24)` matches `record(v7)`.

Between-site exit. `if (v24 < 0) { goto L65; }` preserves `if-ltz v7, :finished`
and the authored `break`. The route reaches the shared exit with `v37` already
holding the current sum from the post-call copies, so no second load, arithmetic
or increment runs. That matches the README row for a negative third load.

Site two. The stores also save the first result in `I$3` and publish
`label = 2`. `v25 = (v24 + 1)` keeps the original `add-int/lit8`, and
`v26 = SuspendTwoSites.load(v25)` is the second suspension. The normal copies
`v45 = v26; v44 = v24; v42 = v41; v38 = v37` restore the saved values at
`L59`. `v27 = (v45 as Number?).intValue()`, `v28 = (v44 + v27)` and
`v29 = (v38 + v28)` reproduce first plus second added to the saved sum, then
`v30 = (v42 + 1)` increments the index and falls through to the head. Each
authored effect occurs once, in the authored order.

Resume paths. State one restores `I$0`, `I$1` and `I$2`, checks the resumed
failure, delivers the site-one copies and continues at `L46`, so the record
still runs exactly once. State two restores `I$0` through `I$3`, checks
failure, delivers the site-two copies and continues at `L59`, so no between-site
effect repeats. The callback's zero limit is replaced by `I$0` on either
resume.

Marker and invalid routes. Both marker comparisons still target the original
`return v2` node, which prints as an ABI `return v9`; tail effects wait for the
matching callback completion. The dispatch default keeps the original
`IllegalStateException` message before any source work.

Arithmetic checks. For limit 2, contributions 2 and 8 give 10; for limit 3 the
next contribution 14 gives 24. The third load returning -3 exits with the first
iteration's sum 2 after record(-3). The two argument-one calls refer to distinct
site/iteration positions, so a fault at the second versus third load has the
separate effects specified in README. Record or second-load failure does not
update sum or index. No contradiction was found under the modeled field/helper
assumptions.

Original references, shared tail copies, state/error paths and call origins are
retained. The earlier explicit `switch` and irreducible graph are no longer
needed for this input; cyclic shapes outside one closed head-tested cycle still
keep that explicit layout. Not run: helper effects, scheduler behavior, or a
final-plan behavioral interpreter.
