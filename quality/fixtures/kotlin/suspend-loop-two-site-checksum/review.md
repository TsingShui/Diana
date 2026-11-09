# Two-site checksum semantic review

This LLM review compares the authored Kotlin/Java model, the Smali input and the
current Engine output for `total`, path by path. The candidate selects one
`while` over the original head predicate and keeps entry/reentry, resume,
marker and invalid-state code as ABI attachments. Structural checks and the
fixture test are supporting evidence only; no helper, scheduler or final-plan
interpreter was executed.

Initial route. The ABI prefix still checks the frame instance, the label high
bit, creates the frame when needed and reads `result`, the marker and `label`.
The source body starts at the original case-zero arm: `v25 = p0` (limit),
`v26 = p1` (bias), `v33 = 0` (sum), `v35 = 0` (index), `v52 = v26`
(checksum), `v39 = v25` (limit copy used by the predicate). The candidate keeps
the arm's original `throwOnFailure` as an ABI comment and places the head
predicate immediately after the initializers.

Head and normal exit. `while (v35 < v39 [exit: v53 = v52; v43 = v33;])` states
the original `if-ge v6, v4, :finished` test with its original operand order. The
exit copies publish the current checksum and sum into the registers the shared
exit tail reads. The false route reaches `L72`, whose original
`add-int/2addr v5, v10` becomes `return Boxing.boxInt((v43 + v53))`, so a
completed or nonentered loop returns sum plus checksum exactly once.

Site one. The four state stores (`I$0` limit, `I$1` sum, `I$2` index,
`I$3` checksum) and `label = 1` remain ordered ABI comments before the call.
`v27 = SuspendChecksum.load(v35)` keeps the original argument and marks the
continuation as inferred. The marker comparison and its two original
destinations stay visible, and the normal copies `v53 = v52; v49 = v27;
v47 = v35; v43 = v33` reproduce the original post-call values. At `L50`,
`v28 = (v49 as Number?).intValue()` is the original check-cast and
`intValue`, then `record(v53)` records the pre-iteration checksum, matching
`record(v10)`. `if (v28 < 0) goto L72` is the authored `break`: the negative
route reaches the shared exit with the copies already published, and it skips
the second load, both arithmetic updates, the post-update record and the index
increment. That matches the authored body and the README row for a negative
third load.

Site two. The stores now also save the first result in `I$4` and publish
`label = 2`. `v29 = (v28 + 1)` keeps the original `add-int/lit8`, and
`v30 = SuspendChecksum.load(v29)` is the second suspension with the same
implicit continuation treatment. The marker branch and normal copies
`v54 = v53; v51 = v30; v50 = v28; v48 = v47; v44 = v43` retain the original
saved values. `L64` converts the second result, computes
`v32 = (v50 + v31)` (first plus second), adds it to the saved sum in
`v33 = (v44 + v32)`, advances the checksum in `v34 = (v54 + v48)`,
records the updated sum, increments the index in `v35 = (v48 + 1)`, publishes
`v52 = v34` and falls through to the head. Every authored effect occurs once
and in the authored order.

Resume paths. State one restores `I$3`, `I$0`, `I$1` and `I$2`, checks
the resumed failure, then delivers the same values as the normal copies and
continues at `L50`, so the checksum record still runs exactly once after the
first call. State two restores `I$3`, `I$0`, `I$1`, `I$2` and `I$4`,
checks failure, delivers the second-site copies and continues at `L64`, so the
pre-second-load record is not repeated. Both resumed routes therefore preserve
the authored side of suspension for every effect. The callback's zero
limit/bias are replaced by `I$0`/`I$3` on either resume, as in the input.

Marker and invalid routes. Both marker comparisons still target `L75`, whose
`return v10` is the physical `return-object v2`; its tail effects are deferred
until the matching callback completion. The dispatch default still throws the
original `IllegalStateException` message after an adjusted label, before any
source work. `throwOnFailure` remains before each resumed use, and helper or
record failures keep their original order relative to the captured values.

Arithmetic checks. With limit 2 and bias 5 the sequence is load(0), record(5),
load(1), record(2), load(1), record(5), load(3), record(10) and the exit returns
16. With limit 3 the next iteration returns 32. A third load returning -3 exits
after record(5) with 7 and performs no further second-site work. These agree
with the README table.

Not established or executed: helper implementations, scheduler behavior,
frame-validity or class-loading failures, and the original Kotlin source shape
beyond the authored model. The review found no contradiction in the modeled
paths.
