# Semantic review

Each iteration starts with a new clock result and its own throwable objects.
The initial clock failure bypasses cancellation and the outer diagnostic,
then reaches the per-iteration handler. The typed interruption path can
resume normally and cancel once. The original outer handler starts with a
capture in one block, then runs its diagnostic and rethrow in a separate
protected range. The per-iteration handler records the delivered reference
and resumes the loop.

The 0.4.5 publication reads an old timestamp or zero on six traces. Diana
0.4.6 stages the selected entry value from the current iteration. A late
fault in round one reports the fresh 8589934600. An early fault in round
two reports its initial 4294967656, including when cancellation replaces
the pending fault. Reversing the order preserves the first old and the
second fresh timestamp. Both ordinary traversals cancel once each and
return after the loop. The same `finally` implements all the cancellation
paths, and the outer catch preserves the diagnostic and delivered references.

The DEX witness and Java models compare the supplied two-round traces using
independent expectations. The review covers the mechanism within that loop.
