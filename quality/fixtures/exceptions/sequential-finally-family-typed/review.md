# Semantic review

Reviewed `e0:l0:c0:m2` in Diana 0.4.3 against the original DEX, the
independently authored nested Java control and all seven scenario traces.

The ordered DEX dispatch sends a matching stage 2 fault to the typed
handler and lets it finish into stage 3. The publication preserves the
typed dispatch, the identity delivered to the handler and the order of
the stage calls. It omits cleanup after a successful stage 3 and directly
propagates a stage 3 failure without cleanup. The original completes
either path through cleanup on the original receiver and token.

Stage 2 failures that miss the typed handler and same-type stage 1
failures skip the typed handler in both versions. The original routes
both to cleanup and rethrows the exact pending fault after cleanup
succeeds. The publication completes normally in those paths. A failure
from the typed handler has the same difference. If both the typed
handler and cleanup throw, both versions reach cleanup and propagate
the cleanup fault.

The modeled traces establish those differences for the authored helpers
and scenario inputs. The publication reports no recovery diagnostics.

The cleanup-family candidate keeps stage 2 in a typed catch nested
inside the common protection. A matching failure reaches the typed
handler and a successful recovery resumes at stage 3. Unmatched faults
and failures in the typed handler escape through the common cleanup.
Successful completion takes the same cleanup, and a cleanup fault
replaces a pending failure. DecBench checks the order, arguments, and
exception object on these paths.
