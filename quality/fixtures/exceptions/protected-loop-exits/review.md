# Protected loop abrupt exits review

## Scope and verdict

Compared the same prepared DEX on baseline7354242 and this candidate through the
ordinary Engine entry. Direct LLM reading covers the Java explanation, actual
Smali, concrete helper bodies, every independent README scenario and complete
subject output. The repair generalizes from GuardArms.loop to nested protected
throws and preserves the return/shared-entry/recovering-handler refusals.
This is a bounded semantic judgement with product checks, not executed behavioral
PASS. No scenario executor, Java recompilation or device run occurred.

## Nested throw and literal cleanup

nested retains row0/sum0 initialization, outer head row<rows, column0 on each
outer iteration, inner head column<cols, key=row*10+column, then probe. A selected
probe constructs the one original guard object, loads its string, invokes its
constructor and throws before read or either pending update. These construction
events remain explicit, ordered and inside the same natural protection owner;
no external constructor implementation is inferred.

The other path calls read, adds its result, increments column and tests the inner
head again. Inner exhaustion increments row before the outer head. For two rows
and columns the returned values1,2,11,12 total26. A stop at1 skips the second
read; a stop at10 preserves both first-row reads before throwing. Probe NFE and
read Error take those distinct event prefixes. Zero/negative rows skip both
loops; zero columns still advances row but never calls either helper.

One finally owns cleanup7 then cleanup8. A body exception remains pending after
successful cleanup. Mode3 throws at the first call and skips the second; mode4
faults on the second after successful cleanup7. Mode5 reaches the key1 read
Error before that second-call fault, which replaces it. With zero rows mode5
has no body Error to replace: only the second cleanup throws. The same reasoning
covers ordinary return replacement. The matched literal/call pairs retain their
original operations and anchors; literals are not swallowed into body work.

The baseline exposes the protected throw after the loops through an explicit
chain/goto. The candidate supplies the enclosing P6 try before lexical-loop
composition, allowing that one finite private path into both containing loops'
lexical sets without changing their cores or continue targets. The inner Guard
then uses the supplied branch plan; P7/P9 do not discover protection or retry it.

## A return which deliberately skips cleanup

early's actual body return has no intervening cleanup copy. The candidate keeps
one natural catch around the loop, not finally. At row==stop the return still
precedes all cleanup text. Thus stop0 returns0 without even the selected mode5
cleanup fault, and stop1 returns the first read's1 with just event200. Ordinary
exhaustion runs the two post-loop cleanup calls. Read Error runs the handler's
same two calls and rethrows; a handler cleanup fault replaces that pending Error
without retrying cleanup. Both original value-return terminals remain printed.
Matching identical normal/handler cleanup calls alone does not license a finally.

## Foreign and recovering paths

foreign retains two original entrances to one construction target: the pre-loop
branch and the loop probe. The candidate output is unchanged from baseline.
It retains explicit c0 sites/capture/cleanup and one shared throw object; no loop
claims its tail private. A pre-loop throw skips probe/read even for zero rows.
Its cleanup still runs7 then8 and mode5 replaces the guard exception at the second
call. Normal exhaustion and an in-loop stop retain their distinct event prefixes.

recover is also unchanged. Its catch performs cleanup7/8 then row+2 and goto to
the original labelled loop head; normal reads add and advance row+1. Recovering
at row1 therefore skips row2 and next reads row3, producing5. Probe failure skips
read201, while read failure records it. Failed cleanup8 prevents row+2 and the
rejoin. Zero rows execute no helper, so its selected cleanup fault is unreachable.
The handler is not an abrupt enclosing owner; its scope cannot license a new
private throw corridor or post-loop finally. The explicit rejoin remains visible.

## Raw selector and rejected draft

The candidate's first broader corpus inspection revealed a real regression:
a raw default targeting the next Sequence child became empty before case0's
goto. That would dispatch default to case0. The draft was rejected, not accepted
because tests or diagnostic counts were otherwise clean. P6 now retains explicit
original selector gotos without a selected Switch interface. P7 also excludes
selector-phase gotos from statement-successor collapse; that next statement is
not the next label printed in the raw header. P9 remains unchanged.

switchExit challenges the correction with a protected private default exit and
an actual case-entry cycle. Every selector arm retains its own explicit target.
Default reaches mark90 then a labelled outer-loop break, never mark10/20/30 or
row update. Cases0/1 retain the single cyclic graph and count increment order;
count resets each outer iteration. With bound3, entry0 produces10,20,10,20 and
adds6 per row; entry1 produces20,10,20 and adds5. Case2 marks30/adds3 per row.
Their shared row update executes once after case work and exits the switch,
then repeats the outer head. Cleanup and its two fault positions remain outside
the selector; zero rows dispatch no case. No case tail or selector was cloned.
The graph and construction notation remain readable limits, not Java legality
gates or whole-result uncertainty badges.

## Supporting evidence

Five new ordinary-Engine checks cover ownership, original continuations,
construction sharing, original terminal/call/copy anchors and supplied transfer
spelling. Baseline source with the same final fixture/test input passes the two
refusals and fails three recovery/spelling checks. This establishes structural
sensitivity only; the return case's baseline was behaviorally readable too.
An initial test used the wrong Terminal field and another assumed class-level
short type spelling on a method request; both test errors were corrected before
passing, without changing scenario expectations.

The prior regression set includes the original pending-rethrow outside recovery,
per-iteration cleanup, monitor release/copies, update-skipping edges, cast order,
case joins and terminal-guard false work. Corpus comparison preserves original
P2/P3/P4/P5 facts, operation/terminal coverage, original edges, chains, locks,
handler receives and copy obligations. Changed current text receives renewed
review; unchanged products reuse their recorded review. These checks and source
readings do not independently execute the final emission plan.
