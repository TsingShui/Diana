# Suspend loop captured-effect review

This LLM semantic review compares `Case.java`, the four prepared Smali methods,
and output from the ordinary Engine entry. Baseline is
`73542421d671be2fc169b72c7233aa0b5b955fd5`; candidate is the reviewed suspend-loop
change. This is a readability repair. No final-plan behavioral executor is
implemented or claimed as passing.

## Positive normal and resumed paths

For nonpositive limits, the initial sum/index are zero, checksum is bias, and the
head predicate immediately reaches the original boxed return of sum + checksum.
Neither output calls a helper on this path. For a repeated iteration, both
outputs prepare index + 1 before computing captured sum + checksum and recording
the old checksum. All four original field saves and the positive label write
follow those operations and precede load. The candidate retains those writes as
ABI attachments at the same ordered actions. Preparation cannot move through
the publication barrier into load's argument evaluation.

The normal result and resumed `result` share the same original merge. The saved
fields recover limit, checksum, index, and the captured expression. P6's move
normalization identifies these same established values; no guessed source local
or field is substituted. A suspended marker returns before conversion or the
shared tail. A resumed failure is checked before conversion, sum/checksum
calculation, post-load record, or increment. Thus the inferred suspend call has
one shared continuation instead of duplicating a tail for immediate/resumed
routes. The output still explicitly identifies the original ABI binding.

On successful completion, the original Number cast/intValue occurs before
captured + loaded. The checksum calculation follows; recording the new sum
precedes the original index increment. The checksum's latch copy remains after
those operations. Forwarded normal copies read their original sources before
any source update, so they preserve the captured value, previous checksum/index,
and returned object. Their original copies remain in the ABI attachment and
owned products. A null result still encounters the original nullable cast and
subsequent intValue dereference; no new not-null or safe-call behavior is added.

For `(3, 2)`, the reviewed recurrence produces sum/checksum `(4,2)`, `(10,3)`,
and `(19,5)`, then returns 24. For `(2,-1)`, it produces `(1,-1)` and `(4,0)`,
then returns 4. The ordered helper effects agree with the independent README
scenarios. Preparation/pre-record failures precede publication and load.
Load failures skip the shared tail. Post-record failures occur after arithmetic
and before increment/another preparation. Unknown native helper implementations
remain explicitly modeled rather than established from the ABI signature.

## Recovery boundaries

`wrongSpill` restores I$2 as the captured expression, so resumed arithmetic uses
the index rather than the saved sum + checksum. The candidate preserves this
different path and rejects the source view. `changedCapture` saves captured and
then increments it before the immediate call; resumed and normal values differ.
Its unmatched copy remains explicit. `resumeEffect` records the restored index
before checking resumed failure; the extra effect remains visible, including
the failure path. These inputs cannot be declared equivalent to `total`.

The ordinary initializer/invalid-state constructor and exact message remain
visible. Original ABI targets are marked inside the recovered body. No member
is removed. Baseline already has readable explicit code for all variants; the
positive method's source-body improvement is the generalization witness.

The written review finds no contradiction under the specified helper model.
Structural origin/order tests support that judgement. Not run: execution of
native helper effects, coroutine scheduling, or a final-output-plan interpreter.
