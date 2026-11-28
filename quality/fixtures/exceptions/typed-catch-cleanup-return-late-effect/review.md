# Post-close effect review

On successful nonempty input the return value is the captured object and the
effects are 123465. Empty input takes the close and null completion with
effects 145. These agree in the authored source, normal-layout control,
and reordered candidate.

A body failure reaches the suppression cleanup, then the typed handler and
owner close. If cursor close throws during ordinary execution, effect 6 is
unreachable. Those paths return null with effects 12345 and keep the
suppressed object's identity where applicable. The candidate retains the
handler's null even when the item was assigned before the fault.

A failure at effect 6 occurs after a successful cursor close. It reaches
the same typed completion, so the result is null with effects 123465.
The recovered output closes the cursor once and then runs owner close.
If owner close also faults, its original exception object replaces the
pending null. The monitor is released on each completion.

The root cause and boundary proof are the same as in the
[base review](../typed-catch-cleanup-return/review.md). The added action
exercises the fused cleanup and handler return after cursor close has
already completed.
