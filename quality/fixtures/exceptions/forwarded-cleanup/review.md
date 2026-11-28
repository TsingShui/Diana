# Semantic review

Diana 0.4.2 and the repair candidate preserve both capture paths. A
successful start returns from its own arm. When the left start throws,
the original catch keeps the left receiver and the value `4294967297L`.
The right catch selects the right receiver and the value `8589934594L`.
The code initializes the path-specific scalar arguments before its call
to start. These assignments affect local storage and both arms return
if start succeeds. Cleanup sees the same values when the call throws.

The shared cleanup receives the Throwable captured on that arm. Its
outward handler receives the original object after a successful cleanup
and the replacement object when cleanup throws. The reporter executes
outside that protection, so its own failure escapes. The model exercises
both receivers, normal return, cleanup replacement, and reporter failure.
The model implements the effects of the input's native declarations.
