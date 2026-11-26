# Protected branch join review

I compared the ordinary `get-class` output with both Smali methods, including
each throwing invocation. The initial and updated builds emitted identical
class text for this DEX.

In `route`, the value saved before entering the try is the argument to the
selected arm. The conditional expression selects only one invocation, with
zero selecting `left`. The outer call receives that arm's result. Its return
is assigned to a separate local. A typed failure in either call reaches
`recover` with the saved value. Normal execution passes the common result to
`after`. The placement of `before` and `after` keeps their own failures outside
the handler. A failure in `recover` leaves through the handler.

In `loopRoute`, the first iteration uses the value from `before`. When an arm
fails, the carried local still has its pre-arm value. On success, a zero arm
result is copied directly to the carried local. A nonzero result is copied
there before invoking `join`. A matching failure of `join` therefore recovers
the selected arm's result. On success the assignment replaces it. Each
successful arm reaches the decrement once. The next iteration tests the
updated count. A nonpositive initial count skips the loop. The output places
the integer test, decrement and back edge inside the try. These operations
cannot throw a synchronous Java exception. It keeps `before`, `after` and
handler calls in their original exception domains.

The native operations stand for the modeled effects in the fixture README.
This is a source-path review. Native methods were not executed.

I also compared the ordinary output for the six unrolled stages of
`wideRoute` before and after the exception index and wide sibling lookup
changes. Both emitted the same class. The initializer sums the two inputs
and calls `before` once. At each stage the assignment to the current local
first evaluates `left` on that local. A failure leaves it holding the
incoming value. On success the assignment adds that stage's number, so the
next call to `right` sees the adjusted value. A failure there reaches the
same handler with the adjusted value still live. Subtracting the stage
number therefore produces exactly the argument specified for each of the
two failure paths, including integer wraparound. The output has six distinct
typed handler bodies. Each returns immediately on recovery. Each normal exit
passes the right result to an unprotected `after`. The first five results
enter the next stage, and the sixth is returned. Nonmatching exceptions and
failures in `after` or `recover` escape. I did not execute the native calls.
