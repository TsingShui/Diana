# Fixture: terminal-guard-arms

## Current verdict — protected loop ownership

Compared the same prepared DEX on7354242 and this candidate. loop now receives
one enclosing P6 finally and a private protected throw corridor. Its initialization
remains before the scope; probe then either the original construction/throw or
read/add/row update execute inside the while. Cleanup runs once afterward.
The unchanged scenario matrix still gives15 for limit2/hit7, skips the selected
read on a hit, cleans once on predicate/read failure, and replaces pending Error
on mode3 cleanup failure. Zero limit runs cleanup40 without probing.

This supersedes the missing-natural-product observation below, not the original
effect expectations. The [fresh nested challenge](../../exceptions/protected-loop-exits/review.md)
adds two loops, literal cleanup order, a return that skips cleanup and real
foreign/recovering refusals. The check now requires the owning P6 facts before
observing Guard; no downstream reconstruction route was introduced. Other
members' current text/products are unchanged and reuse their preceding review.
Original P2/P3/P4/P5 facts agree. This is bounded LLM reading and structural
evidence, not behavioral execution.

## Prior LLM semantic judgement

Compared authored Java/Smali, concrete helpers and complete ordinary Engine
source on the same prepared DEX, baseline29f1b82 versus the commit containing
this review. Each subject's independent scenarios preceded its first inspection.
Recorded paths agree. No scenario executor, device run or Java recompilation
ran; this is bounded manual judgement, not behavioral PASS.

### Saved parameter and actual other-edge Phi copy

select saves the original base before selecting9 or entering the probe path.
Flag0 skips probe/read, marks50 and returns9+base. Flag1 probes0, reads1 and
returns8 without mark50. Flag2 probes1, then selects the original base, marks50
and returns twice base, including-2 for MAX with32-bit wrapping. Read/predicate
faults skip all later work. The saved parameter is not overwritten by the Phi.

The original guard's taken edge enters the Phi, while its opposite edge returns.
P7 therefore supplies an inverted Guard and the exact other edge. Actual P5 plan0
contains one executable parameter-to-Phi Assign; plan1's literal9 requirement has
zero steps. The candidate prints that parameter assignment after the guard, not
at its predicate and not twice. Its Copy/Transfer anchors occur once and after
the exit's terminal span. This is real assignment coverage, not a plan-ID claim.

### A guard is not a single nested If

outer's first successful probe marks30, selects7, then marks70/returns7. Its other
path probes which: success reads3/returns10 without mark50/70; failure marks50,
selects8, then marks70/returns8. Read failure propagates after the two probes and
read event. The inner Guard plus later false work remains inside the outer else
scope; that whole body is not misclassified as one else-if. Flattening it out of
the outer branch would wrongly run mark50 on the first successful probe, which
the current plan/source do not do.

### Recovering catch and missing producer guards

recover's successful probe enters the original try/read. Normal read returns9;
ISE records22, catches/marks60 and jumps to the original shared mark70/return4
without mark50. The other probe result marks50/70 and returns4. Error bypasses
this typed catch. The complete true arm includes an opaque Try and the other arm
contains a real shared-entry Graph, so neither supplies the linear guard proof.
The else and explicit shared labels remain. A return keyword inside that Try is
not treated as a proof that recovery cannot continue.

loop retains explicit protection rather than gaining a fabricated natural If or
finally. Its available P6 product has the original c0 sites, capture, cleanup
copies and gotos. It is a missing-natural-product negative, not positive guard/
finally coverage. The readable paths still agree: hit7/limit2 reads7/8 and returns15
after cleanup; hit1 throws after the first update, hit0 before the first read.
Read/predicate faults clean once, normal cleanup failure replaces return, and
mode3 cleanup IAE replaces pending Error. Zero limit still runs cleanup40.

### Cleanup before a guard, construction and declaration scope

after reads before its one natural finally, then probes value-8. code1 returns8
after21,41,10; code2 returns10 after22,42,11,50. Read ISE survives cleanup; failed
normal cleanup and pending-Error replacement skip probe/mark50. The candidate's
chosen original guard edge can return value+1; its opposite return remains after
the guard. This polarity difference changes no event or result.

throwing probes once before constructing the guard ISE or running mark50/read.
The selected throw skips both normal effects. Failed normal read preserves its
own ISE/Error identity rather than inventing the guard exception. Construction
and operand order stay at the original selected branch.

declared returns0 after its first probe or reads2, marks its returned9 and returns10.
The read's initializer is now in the actual parent scope after the guard; its
statement/call/commit remain at that position. A read fault skips the value mark.
The test establishes the exact original Store/local_initializer relationship,
not just the presence of declaration text.

voided retains the actual early void return inside its chosen guard. Probe0
marks30 and returns without50/70; probe1 marks50/70 and returns without30.
Method-tail omission does not remove that guard exit. Helpers preserve their
wrapping trace, event-before-result/fault order and original arguments.

## Owning implementation and rejected assumptions

P7 consumes existing selected control/statement schedules. One common block-end
classification serves continuation omission and guard selection. The entire exit
arm must be plain linear Sequences/Blocks, ordinary fallthrough prefixes and an
actual printed return/throw. Required labels, continuations, nonstatement actions
and opaque controls anywhere in that arm decline it. No CFG/dominance computation,
symbolic replay, owner retry, external fact or alternative analysis is introduced.
Deferred origin spans alone are metadata: composed calls still execute at their
existing returned uses, so that flag is not falsely treated as a control transfer.

The first draft handled only the original true edge. Fresh select/outer exposed
the opposite-edge case, so the producer now records inversion and exact edges.
The complete-arm restriction was strengthened before judging the recovering-Try
negative; it is not a last-return-keyword scan. These drafts are not reported as
completed generalization. Source inputs/scenario values were not weakened.

P7 declaration placement consumes the chosen Guard orientation and puts the
remaining body in its actual parent scope. Guards do not count as a single If
for else-if composition. P9 consumes the supplied plan only: selected exit arm,
close, other-edge copies/one Transfer anchor, complete remaining body. It neither
chooses eligibility nor skips the other arm. No for recovery is reintroduced.

## Supporting checks and limits

Run `cargo nextest run --locked -p diana-decomp --test terminal_guards --no-fail-fast`
from the repository root. Nine ordinary-Engine checks cover the discovery loop,
actual inverted-edge Phi copy and spans, parent else scope, opaque recovery,
cleanup ordering, constructed throw, early void return, initializer scope and
explicit missing-producer refusal. Default/all-feature package suites each pass
83 tests across 19 binaries. These are product/anchor checks, not scenario execution.

Within the recorded cases the guard display is effective without lost false
work, altered effects or hidden updates. Arbitrary inputs, concurrency and faults
excluded by the README model remain outside this judgement.

Package Clippy with warnings denied, release/dbg_shell builds, format/diff and
desensitization checks passed. Same-DEX inspection observed 90 fixtures, 141
classes and 685 methods without stopped members or render failures/issues.
Exact P2/P3/P4/P5 and complete P6 products agree with 29f1b82; only display plans,
declaration placement and source change. Fixed-baseline original-coordinate
projections also preserve original facts. All 41 current changed fixture texts
have current path reviews; counts are observations, not quality scores or a
semantic certificate. Required behavior execution remains unimplemented/unrun.
