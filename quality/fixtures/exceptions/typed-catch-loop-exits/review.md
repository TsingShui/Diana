# Fixture: typed-catch-loop-exits

## Current verdict — while/do-while architecture correction

Read normal and shared typed recovery: cleanup precedes addition and the visible
counter tail; value2 skips only addition, negative return and threshold break
skip update. Uncaught/replacement faults execute neither update nor next read.
Adjacent catch aliases and original priority remain.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Compared the same prepared DEX with completed-eleven-repair bda9a49. Only
derived label/control coordinates change. Development-only correspondence found
identical printed words, original L/H program points and identical printed
lexical-scope bindings across every method publication; declaration-only members
are exact. Original P2/P3/P4/P5 facts agree. The unchanged semantic path review
below is reused, not replaced by a naming score or a claim of behavioral
execution.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


## Prior verdict — iteration-tail review

Java, Smali, helpers and final scan(II)I were reread. The original row update
now has one for-header spelling, eliminating the value-2 goto. Value 2 skips sum
but still updates row before retesting; negative return and sum>=200 break skip
that update after cleanup. Error/cleanup replacement and catch priority are
unchanged. The body/predicate/update split comes from P6, without relaxing the
primary-boundary no-retry rule. The new nested-tail challenge covers both counters
and two cases that must remain ordinary while.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior LLM semantic review

The following finally iteration also recovers the per-iteration cleanup as
finally. Direct reread confirms cleanup still precedes the value-2 skip/update,
sum break and negative return; pending Error survives a successful cleanup and
is replaced by mode-6 cleanup failure. The loop is not absorbed into this try:
its unguarded normal cleanup prevents widening the protected scope over the
whole loop. The corresponding structural guard is in `finally_scopes.rs`.
`../branching-finally-suffix/` reviews the distinct whole-loop cleanup challenge.
No behavior scenario execution is claimed by this follow-up.


Compared the Java explanation, Smali, concrete read/mark/cleanup helper bodies
and ordinary Engine output. Baseline is `ebbc003`; the candidate working tree
adds ordered catch groups, inherited protection intervals, nonthrowing entry
closure, private handler loop-entry construction and matched cleanup. These are direct semantic reading judgments, not behavioral execution.

### Semantics

| Original method | Judgment and reasoning |
|---|---|
| `scan(II)I` | Equivalent on the README scenarios: values are selected before cleanup; normal cleanup runs once before any continue, break or return; typed failures enter one shared body; Error enters the outer cleanup; cleanup failure escapes without a retry. |
| `priority(I)I` | Equivalent: the first NFE clause selects 100; the intervening RuntimeException clause selects -2 for NPE/ISE before the later NPE alias. Its original ordered explicit chain remains intact. |
| `read(II)I` | Equivalent: mark(10+row) precedes the mode-selected throw or row+1 return. Modes 4/6 share the same AssertionError constructor path. |
| `cleanup(II)V` | Equivalent: mark(30+row) precedes a mode-5/6 IAE or normal return. |
| `mark(I)V` | Equivalent: the old trace is multiplied by 31, then the supplied event is added and stored using the same int arithmetic. |

Normal mode skips adding row 1's value 2 only after cleanup, then returns 4 for
three rows. Modes 1/2 accumulate two recovered values of 100 and break after the
second cleanup. Mode 3 cleans once before returning -2. Mode 4 closes once then
rethrows the original Error. Mode 5 throws from normal cleanup. Mode 6 reaches
cleanup with a pending Error and replaces it with the IAE. Both failing cleanup
calls are outside the protection they implement. Zero rows execute neither helper.

The prior candidate exposed another instance of the same ownership cause: reach
from a catch through the loop could include the outer cleanup entry. The final
interval boundary excludes later original clauses from that handler's private
content. The typed catch and outer cleanup now have distinct source owners.

## Recovery and complexity challenge

The baseline supplies no natural try in this class; scan's typed body and cleanup
are protection graphs. The candidate supplies two nested natural protection regions, now including the
matched outer finally and one compound NFE/NPE header, with the broad
RuntimeException clause still second.
The nonadjacent-alias priority method remains explicit rather than being reordered.

The first catch-group candidate still printed scan as `graph<Irreducible>`:
handlers rejoin the cleanup/backedge while normal dominance treats them as
independent analysis roots. The subsequent owning-stage loop repair proves a
closed real entry without changing that normal dominance graph. The final
candidate has one natural while and the two nested tries, not a dispatcher.

The value-2 path still jumps to the counter update before the next condition;
printing a plain continue there would incorrectly skip the increment. The
sum>=200 break leaves after cleanup, and the negative-value return does the same.
The Error cleanup/rethrow and cleanup-failure replacement paths are unchanged.

After the loop repair, `../nested-catch-rejoins/` adds two nested loops with held
cleanup and a real pre-loop handler arrival guard. Both private loops recover,
while the foreign arrival stays explicit. Its review records the path comparison
and the maximal-SCC limitation found and removed during that challenge.

## Supporting evidence

Three structural checks in `crates/diana-decomp/tests/catch_groups.rs` use ordinary
Engine delivery for this fixture and the discovery fixture. They check ordered
clause coverage, one shared capture/body, enclosing cleanup ownership, natural
protection and one original call/capture anchor. They do not execute the scenarios. Three additional loop-entry checks cover the
single loop, nested monitor challenge and foreign-arrival refusal. All 25 package
checks passed with all features; the loop repair changed only these two fixture
texts across the 77-fixture comparison.

```sh
cargo nextest run --locked -p diana-decomp --test catch_groups
```

No Java recompilation or independent behavioral executor was run. Constructor/VM
failures beyond the README assumptions and arbitrary concurrent trace writes are
not reviewed scenarios.
