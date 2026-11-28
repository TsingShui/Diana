# JVM interface initializer snapshot review

`InterfaceSnapshot.<clinit>` captures `SnapshotEffects.INSTANCE` at byte 12,
stores it in a local and supplies that reference to the replacement call.
The original `putstatic RESULT` loads the same local. The replacement writes
`AFTER` into the volatile field, so a later field read would observe a different
object. The original result is `BEFORE` and the trace from the call is 1.
Diana 0.4.25 reads the field once in the replacement argument and again in
the initializer result. Recompiled output still passes `BEFORE` to the
call, then publishes `AFTER`.

The protected original saves the reference before entering its typed
exception range. The normal path assigns the saved reference after
replacement. When replacement throws, the handler sees both that saved
reference and the original exception. The trace is 124. The 0.4.25
publication reads the volatile field in the protected call, the handler,
and the result. Its normal path publishes `AFTER`. Its fault path publishes
`AFTER` and reaches the handler with `AFTER` and the same exception, giving
trace 194.

`StableSnapshot.<clinit>` loads the final `StableOwner.INSTANCE` before
calling the replacement. The first active access initializes `StableOwner`
and appends 3 to the trace. Replacement appends 1 and changes a different
field, so the saved reference remains `BEFORE`. The 0.4.25 output passes this
control with trace 31 even though it also repeats the immutable field read.

The Diana 0.4.26 candidate saves the captured reference in a local in each
initializer helper. Its recompiled helpers each contain one `INSTANCE`
read. In the protected helper that read is at byte 0, before the typed range
starting at byte 4. Both the replacement and the handler load the same local,
and the helper returns it. The normal and fault paths publish `BEFORE` with
traces 1 and 124. The stable control preserves its trace 31.
