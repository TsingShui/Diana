# Semantic review

For `run`, switch key 0 calls `skip` and reaches the row increment.
Keys 1 and 2 call `start`, then `step` for descending positive values.
After the countdown, the read handler covers the read alone. A caught
read contributes `recover(row)` to the total. A failure in the countdown
skips recovery even if it is also an `IllegalArgumentException`. The
default exits directly with the accumulated total. Each exit reaches the
single outer cleanup with the original mode. A fault in that cleanup
replaces an escaping read failure. These branches appear in the same
lexical order in the candidate's ordinary class output.

For `sweep`, keys 0 and 1 run the complete countdown, key 2 only skips,
and the default calls neither helper. A preface fault leaves the method
before the protected row loop. In that loop an empty range returns the
initial total through cleanup. A successful or recovered read increments
the row and adds its selected value before the next header. A different
read exception or a fresh exception from recovery exits the loop through
cleanup. Mode zero calls just `skip(rows)`. Any other mode calls
`step(rows)` followed by `close(mode)`. A failure in the step bypasses
the close. The output uses the same receiver-free static calls, incoming
arguments, exception types and success/failure continuations.

The original Java explanation and the release output with the scope fixes
each passed all twelve JVM scenarios. A deliberate premature return at
the second loop's latch changed the result from 123 to 10 and skipped the
second and third read. A second negative control incorrectly entered
cleanup after a preface fault and recorded an extra close. The independent
driver rejected both mutations.

The release baseline also passes the twelve scenarios. Its concrete difference
is loss of the explicit continue after the protected row update. Falling
through the loop's closing brace has the same effect for these inputs.
The Engine regression distinguishes that transfer in the retrieved source.
The behavioral evidence establishes preservation for the listed scenarios
and demonstrates sensitivity to incorrect scope or latch placement.
