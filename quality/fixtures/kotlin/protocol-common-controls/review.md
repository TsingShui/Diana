# Semantic review of common protocol controls

Reviewed against the same prepared DEX on baseline `5e01494` and the current
replacement. This is an LLM path review, not executed coroutine/VM acceptance.
Original selector: Lfixture/UnifiedBody;,
invokeSuspend(Ljava/lang/Object;)Ljava/lang/Object;.

## Initial and successful paths

The baseline establishes the local SuspensionPlan but keeps the dispatch in its
executable body. Its source-body producer expects a coarse prefix; the DEX has
three original-operation portions before dispatch. The candidate consumes their
actual straight entry/outlet relation and selects ordinary source/ABI/origin
scopes. No operation is invented to combine them.

The candidate retains the initial throwOnFailure before Prepare. Prepare returns
7 under the explicit model and remains before the original state-field write;
that write is an ordered ABI action. Load retains argument 7 and its original
callee. Exactly the continuation occurrence becomes implicit. Its successful
publication writes the same merged storage as normal/resumed DEX delivery. The
original output definition, result commit, copy and transfer remain inspectable.
The normal copy is owned by the call publication and is absent from executable
edge copy lists, preventing duplicate delivery.

After reply 5, the original Number cast and intValue occur before Record(5).
The negative guard is false. ReadBox construction precedes Observe(5,5), and the
same reply is returned. The supplied ReadBox constructor calls its original super
constructor and stores first before second. A nullable cast spelling preserves
DEX's null-accepting cast; the later virtual call keeps its own null failure.
There is no additional not-null check in the selected body.

After reply -2, Record(-2) precedes the taken guard and Boxing(-1). The box and
Observe path is skipped. The candidate retains this guard and the distinct return,
rather than folding both paths into a fabricated result.

## Suspension, resume and failures

On initial Load suspension, the input returns the distinct marker before shared
tail work. The logical suspend call abstracts that boundary; its original marker
predicate/return identities remain origin attachments. Original state dispatch,
resumed failure checking, invalid-state construction and resume copy are readable
ABI attachments, not another executable source body.

On state-1 successful reentry, the input checks the delivered result and reaches
the same merged storage. Prepare/Load do not repeat on that ABI path. Record and
the appropriate tail run after successful delivery. A resumed failure propagates
from throwOnFailure before Record, construction or Observe. Initial Load failure
likewise prevents those later operations. Invalid states retain their own
IllegalStateException construction and throw. Allocation failure after Record(5)
prevents Observe and return; no selected cover moves construction before Record.

These judgments use the helpers and environmental assumptions in README.
General coroutine runtime behavior, external helper implementations and VM
execution are unrun. The review does not certify arbitrary exceptions or null
inputs outside those models.

## Preserved comparisons

The existing protocol-owned-reads, compiled fetch/total, two-site loop, checksum,
negative-spill/resumed-effect and three-site/private-return cases retain their
source calls, constructor reads, branch exits and original ABI targets through
the common grammar. Loop/flow source declarations can now use the ordinary
lexical initializer placement. Their original method identities and instruction
order stay the same; the previous per-form traversal layouts are removed.
Existing path reviews support unchanged input/helper behavior. The scoped machine
checks separately inspect source obligations and run the already implemented
scalar/construction/lock models. Coroutine execution remains unrun.
