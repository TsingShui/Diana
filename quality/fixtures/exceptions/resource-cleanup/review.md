# Fixture: resource-cleanup

## Current LLM semantic judgement

Read the authored Java/Smali model, independent README expectations and complete
ordinary Engine output on the same prepared DEX, bda9a49 versus candidate.
Recorded normal/fault paths agree. This is manual path reading, not behavioral
execution; no scenario executor, device run or Java recompilation ran.

loop initializes total/index before entry, opens the current handle, reads, adds,
closes and then updates once. Zero iterations has no calls. Failed acquisition
has no close. Read failure closes once and rethrows its captured identity; a
failed close on that path is added to the primary. A normal close failure escapes
without retry/update. Thus loop(2) retains return10 and O0,R0,C0,O1,R1,C1.

nested opens1 before opening2. Either read failure closes2 then1, retaining the
primary and ordered suppressions. Failed second acquisition closes only1. Failed
normal close2 becomes the outer primary and close1 failure is suppressed onto it;
failed normal close1 escapes without retry. Its pure total still precedes close2,
even though it is now outside the smaller read-only try. nested(5) returns35
under the normal model. The pure sum cannot throw or alter cleanup order.

sharedHandler's one catch remains outside the loop, covering both loop reads
and the later read. Initial total0 is still established before its first read.
The post-loop pure sum remains before normal close. A failure at either read
uses one shared capture/close and ordered suppression; that handler is not cloned
into or privatized by the loop. sharedHandler(1) retains two reads and return0.

Native helpers remain declarations with original identities; the engine does
not acquire the README model as external facts. Four existing ordinary-Engine
product checks retain protected loop coverage, nested handler boundaries and
the shared-handler guard. They are not execution of the scenario table.

The [pure-tail challenge](../pure-tail-finally/review.md) records whole-operation
portion ownership; [carrier checks](../../atoms/refined-carriers/review.md) cover
its expression consumers. Original P2/P3/P4/P5 facts and original-coordinate
P6 edges, exceptions, locks and copy phases remain. Within the recorded paths,
new for tails and smaller pure-only protected placement retain effects and
faults; arbitrary inputs and README-excluded helper faults remain unreviewed.
