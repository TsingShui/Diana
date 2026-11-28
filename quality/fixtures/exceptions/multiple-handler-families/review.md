# Semantic review

The release baseline at `49af4a17` does not recover either method body.
It reports competing handler parents for the shared protected operation.
The candidate publishes all three ordered exception tables as distinct
Java-like try statements, and preserves the typed catches and their
exception bindings.

If the first operation succeeds, its original branch ends the method.
The candidate returns from the first try on that path. If a matching
exception occurs, the corresponding mark call receives the caught value
before the next protected operation runs. An unmatched exception from
the original operation or from a mark leaves the method. In particular,
the first handler is outside the retry protection in both input and
output. A fault thrown there never enters the retry catch.

The retry operation again ends the method on normal completion. Its two
matching exception branches each record their own caught exception
before reaching the last operation. Only the last operation dispatches
to the InterruptedException handler. An unmatched fault skips that
handler and the later loop continuation.

In the repeated version a nonpositive limit exits before any call. On
each reached iteration the comparison precedes the wait. The candidate
places the pure integer comparison inside the same try as the wait.
This adds no exceptional path to the try. The candidate decrements the
counter after a normal wait or a handled interruption. A fault in the
wait or in the interruption handler leaves before that decrement. The
scenarios exercise one successful wait, repeated interrupted waits,
and termination at a later unmatched fault. They do not inject a fault
into the interruption marker, whose placement was reviewed directly.
