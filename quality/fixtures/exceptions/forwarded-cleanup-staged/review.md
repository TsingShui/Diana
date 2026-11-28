# Semantic review

The left capture forwards the Throwable through the bridge and combines
the left receiver with `11` and `4294967297L`. The right capture skips
the bridge and combines the right receiver with `22` and `8589934594L`.
They both pass the forwarded tag. The direct capture supplies `33` and
`12884901891L` with the direct receiver and tag. Both forwarded inputs
depend on the ingress even though the decompiler can use one source
variable for each pair of scalar values.

The candidate assigns these arguments before each corresponding start
call. The assignment is local and the method returns immediately if
start succeeds. Both throwing forwarded lanes retain their own arguments
until cleanup executes. The cleanup receives the original caught object.
If it succeeds, the outward handler receives the original rethrow. If
cleanup throws, the same outward handler receives the replacement
exception. A failure thrown by the outward handler escapes and does
not trigger that handler or cleanup again.

The model checks the recorded arguments, receiver, exception identity,
and event order. The right normal-return path is checked by inspection
of the original Smali and candidate, with left and direct normal-return
paths also covered by execution.
