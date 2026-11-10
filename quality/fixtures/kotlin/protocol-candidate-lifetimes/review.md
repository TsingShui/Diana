# Semantic review of the outside-observer challenge

Reviewed with the same prepared DEX at baseline `f565f88` and the current
replacement. The class texts are identical. This is LLM path review and product
inspection, not executed coroutine/VM acceptance.

The initial run initializes sum, index and captured to zero and seed from the
original parameter. Consequently zero iterations still construct the tail box
with (0,seed), observe it, record limit, and return seed. Moving a body-only
checksum expression into this tail would lose its zero-iteration definition;
the delivered source instead keeps the original initialized storage.

Each iteration evaluates Prepare(index) before the checksum, first Record and
state/spill stores. Load retains its original argument and continuation. A normal
reply and its matching reentry restore the same checksum, sum, seed and index.
The normal copy commits the checksum into the storage read by the shared tail.
Its materialization cannot disappear through repeated-body-only forwarding,
because the post-loop ReadBox constructor observes that storage. Canonical
ProtocolCall metadata retains the actual resume nodes and four original field
pairs; it does not replace the corridor with all ABI nodes.

After a successful load, Number conversion precedes the sum update, body box
construction and Observe. Seed updates after Observe and before the second
Record and index increment. The tail box follows loop completion. Its Observe
precedes Record(limit), which precedes final boxing/return. With the declared
helper replies this gives the independently recorded 14 and 28 results and
ordered traces for one and two iterations. The tail sees the checksum captured
before the final load, rather than a checksum recomputed after the final update.

A load fault or resumed failure stops before the downstream body box and tail.
A body allocation failure stops before its Observe and later updates. A tail
allocation failure, including zero iterations, stops before its Observe,
Record(limit) and return. Invalid states retain the original exception
construction and throw. Class initialization, marker acquisition, super
construction and native helper behaviors retain the README model assumptions;
other runtime/callee behavior is unrun.

Removing published raw candidates changes storage and ownership of analysis
products, while the one selected grammar and its original source associations
retain these paths. The seven compared protocol inputs keep their previous
source order and identities. Existing path reviews are reused for unchanged
inputs/output; the new tail and outside-read paths are reviewed above. Machine
checks separately cover canonical associations, copy ownership and retained
storage. No coroutine execution or universal equivalence is claimed.
