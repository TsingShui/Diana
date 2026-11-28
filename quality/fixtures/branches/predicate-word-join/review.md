# Semantic review

The original `instance-of` tests whether the value is a String. Null
and non-String objects produce zero. The candidate evaluates the same
test and explicitly converts its result to zero or one when storing it
in an integer local.

In `select`, the true branch takes the supplied alternative directly.
The candidate select passes that value to the recording call and returns
the same value. The false branch supplies the converted predicate result.
A negative alternative verifies that this storage cannot be reduced to
Boolean zero or one.

In `run`, the array length is read before any iteration. Each step reads
one Boolean. When the step is true, the candidate adds one to the current
integer. Otherwise the integer remains unchanged. The candidate duplicates
the pure choice between these values, then calls the recorder, calls
`absent` if the value is zero, and commits the same integer for the next
iteration. The array access and calls still each occur once per relevant
iteration. With no iterations, the candidate returns the original numeric
predicate. An exception from a recording call bypasses the later call and
the loop update in the candidate. This path was reviewed from the code and
was not exercised by the model. The model also does not exercise null arrays
or integer overflow.

The baseline declares the shared counter Boolean. Its backedge uses
`!= 0`, so the emitted storage cannot represent the value two, and the
baseline arithmetic is unavailable for execution by the model. The
candidate represents both 0/1 seeds and unrestricted integer updates.
