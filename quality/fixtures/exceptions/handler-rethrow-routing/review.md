# Semantic review

The normal setup path comes before any scheduling or waiting. Only
IllegalArgumentException from setup reaches the retry hook. Its true
outcome enters setup again, and its false outcome leaves without a
schedule or cancellation. Throwing the same type from the scheduling
call reaches the exceptional cancellation path, followed by the session
handler with the original exception.

A normal wait and a successfully handled interruption both reach
ordinary cancellation. The interruption marker runs only after the wait
selects its InterruptedException handler. If the marker throws, the
original Smali table sends it to the exceptional cancellation entry.
That entry calls cancel and passes the same thrown object to the
session handler. The candidate now lets the enclosing copy of that
cleanup handle the exception directly. No generated inner cancellation
can rethrow into that copy. The retry handler does not intercept the
exception. If cancel itself throws, its replacement is the only
exception delivered to the session handler.

The secondary catch handles IllegalStateException from the interruption
marker. It performs its recovery action before normal cancellation.
The same type thrown by the recovery action is outside the secondary
protected range and takes exceptional cancellation. An unrelated
interruption-marker failure bypasses the secondary catch in both the
input and output. These distinctions are covered by the logged
scenarios, including exception object identity. A fault thrown from
the session handler was checked by inspection of the DEX table and
candidate scope and was not injected in the model.
