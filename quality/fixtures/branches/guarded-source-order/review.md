# Semantic review

The two methods share a subtype guard and an `InterruptedIOException`
handler. In each original DEX method the first guard consumes `preview` and
chooses between an early effect and null. The join reaches the second guard's
key read, then the optional `confirm`. Both selected results are consumed
later by the merges. The null alternatives leave the unselected effects
unexecuted. [Original paths and code-unit positions](README.md)

The recovered methods retain an early boolean test for `preview` and emit
`confirm` within the subtype block. They defer the guarded call that produced
the first value until the first merge argument. The two effects can then run
in reverse order, even though the normal return still contains values ordered
by their merge positions. An interruption in `confirm` reaches the same
handler but now bypasses the earlier effect.

The challenge uses the first call's mutation as an input to the second guard.
The recovered guard reads the prior state. It can omit the second effect and
the fault that would have reached the handler after `open`. When `open` is the
first throwing effect, the recovered method can call `confirm` before the
interruption. The independent trace records both distinctions.

All authored source controls match the eleven scenario expectations. Execution
of the stock Diana 0.4.2 publication gives six disagreements and five matches.
The raw and recompiled DEX positions independently show the displacement. This
review covers the selected helpers and protected paths described by the fixture.
