# Semantic review of shared single-site recovery

Compared the same prepared DEX at `429998c` and the common-producer replacement.
This is LLM path review and product inspection; coroutine/VM execution is unrun.
Original selector: Lfixture/OneSite;,
choose(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;.

The baseline keeps a readable original switch and its state-zero/resume arms.
It has one complete call/marker/spill interface, but the former ordinary source
producer excludes a dispatch with only one positive reentry. The replacement
uses that same interface without requiring another call site or a loop. The
separate loop recognizer and its conversion are removed from the ordinary chain.

For initial seed 3, Prepare(3) precedes the saved-seed field store and state-1
publication. Load retains argument 10 and its original ABI target. Its matched
continuation occurrence is implicit in source. Normal completion commits reply
and seed on the original normal edge. The shared Number conversion precedes
Record. A null still passes the nullable cast and fails at the virtual intValue;
no extra not-null check or safe call is introduced.

With reply 5, the negative guard is false. ReadBox construction receives (5,3),
Object super construction precedes both field stores, and Observe follows that
construction. Final boxing returns Integer(8). With reply -2, Record(-2) precedes
the taken guard and Integer(-1) return; construction and Observe are skipped.
The selected ordinary If and tail retain these original orderings.

The state-1 ABI corridor restores the seed and checks the delivered result before
joining the same original reply/seed storage. Prepare and Load do not repeat on
that ABI path. A resumed failure stops before Record and the tail. An initial
Load fault likewise stops before Number conversion/Record. A suspended marker
returns before shared-tail work; the logical suspend view keeps its original
predicate/return correspondence in ABI attachments. Invalid states retain their
original IllegalStateException construction and throw. Box allocation failure
after Record(5) prevents Observe and return. Native/callee behavior and general
runtime failures retain the README assumptions and unrun status.

Four previously recovered single-site loops from kotlin-suspend-state,
protocol-owned-reads, protocol-candidate-lifetimes and suspend-loop-captured-effects
now use the common cyclic source selection. Their executable text is unchanged;
original source path reviews can be reused. ABI marker correspondence moves to
its original source position, with the original destinations and normal copies.
The existing three-copy forwarding case retains its three chosen aliases;
outside checksum observers still reject forwarding. Other prepared Kotlin groups
retain their previous construction/rendering failure counts and readable members.
These checks do not establish equivalence for unmodeled coroutine runtimes.
