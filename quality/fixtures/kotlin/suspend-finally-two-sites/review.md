# Protected two-site semantic review

This LLM review follows the authored Smali protection intervals and the final
explicit Engine output. The Java/Kotlin explanation describes logical finally;
the Smali input distinguishes physical suspended returns. Baseline `64aaa04`
lost the default transfer; the candidate retains its original invalid-state
target outside cleanup intervals.

Initial id is assigned before the first protected check/load. Both resumes
restore id before their protected failure checks, and the second resume also
restores first. Each output check/cast/call in the supplied intervals retains
the original catch-all chain and id snapshot. Thus a modeled immediate or resumed
load failure reaches cleanup with the original id, rather than the callback's
zero argument. Number conversion failures use that same delivery. The output's
explicit cast and `via Number` call are separate original events.

The successful shared tail computes first + second before normal cleanup, then
boxes the captured int. Id 0 gives 2, id 2 gives 14, and id -1 gives -4, after
the README's two load effects and one cleanup. The handler receives its captured
throwable, obtains the snapshot id, invokes cleanup, and rethrows that same
throwable. Cleanup itself has no edge back into the protected chain, so its
failure replaces the pending result/error without another cleanup. No synthetic
suppressed exception is introduced.

Either marker branch reaches the original marker-return label outside cleanup;
cleanup is deferred until logical success/failure. Invalid adjusted labels reach
their error construction outside protection. No contradiction was found on the
modeled helper/field paths. External resolution/allocation behavior is not modeled
or treated as checked. The source try/finally is not reconstructed; explicit
chains, snapshots, handler and duplicate normal cleanup remain readable facts.
Not run: runtime scheduling, helper effects, or a final-output executor.
