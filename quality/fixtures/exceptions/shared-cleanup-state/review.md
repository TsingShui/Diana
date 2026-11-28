# Semantic review

An initial clock failure occurs outside the cleanup scope. Successful
scheduling reaches the wait and refresh. A failed schedule or a failed
refresh arrives at cancellation with the initial timestamp. A successful
refresh commits a new value before reacquisition. An exceptional wait,
save, or interruption callback takes the exceptional refresh and rethrows
the original fault if the refresh and reacquisition succeed.

Diana 0.4.5 presents one `finally` but provides zero or the old timestamp
to the outer catch on ten modeled paths. The recovered 0.4.6 body saves
the selected value before a throwing operation or explicit rethrow can enter
the cleanup and also on the successful normal exit. A single `finally`
performs cancellation. Its enclosing catch restores the snapshot from the
executed path. A failure in cancellation replaces the pending exception
and preserves the selected timestamp. A successful ordinary or handled
interruption runs cancellation once and returns without a diagnostic.

The DEX witness and DecBench runner exercise the supplied recording hooks.
The written review covers those paths and the physical exception entries.
