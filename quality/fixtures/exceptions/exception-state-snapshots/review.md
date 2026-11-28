# Semantic review

The DEX starts outside the protected range at the initial clock. A fault
there escapes before scheduling. A scheduling fault goes straight to the
exceptional cancellation with the initial timestamp. A refresh clock failure
likewise leaves the old timestamp in place because its result move executes
only on success. The recovered method preserves old state in those paths.

On nonmatching wait, save, or interruption-handler failure, the exceptional
refresh writes the new timestamp before reacquisition and the explicit
rethrow. Diana 0.4.4 projects the outgoing value to the earlier failure site,
before the fresh timestamp exists. Its diagnostic reports timestamp 0 on
those three paths. The recovered 0.4.5 method places the state transfer at the
surviving rethrow after reacquisition. The diagnostic receives 8589934600 and
the original fault identity after one cancellation, as in the original.

A normal refresh, including after handled interruption, commits the new
value before reacquisition. Both successful routes cancel once and return.
If reacquisition or normal cancellation fails, the resulting diagnostic
gets the new timestamp. If cancellation fails while a fault is pending, the
replacement reaches the same reporter and replaces the original. The
outward throwable observed by the harness is the reported reference.

The executable scenarios model hook effects at the calls. They cover the
ranges and fault sites described here. They establish the supplied traces
and do not exercise Android framework implementations.
