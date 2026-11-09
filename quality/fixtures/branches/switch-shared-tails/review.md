# Switch shared-tail review

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `IllegalArgumentException v13`. The original captured object is bound once at exceptional
entry, before the unchanged receiving/body actions. Its exact-state uses remain
inside that handler and have no capture-phase, Phi or copy dependency requiring
an outer assignment. Normal execution does not bind or run the handler.

After accounting for that binding and its removed private declaration/assignment,
the complete remaining output is exact. Original P1/P3/P4/P5 products agree.
Thus the earlier independently reasoned normal, caught, cleanup, rethrow and
secondary-failure paths below are reused: their calls, arguments, values,
priority, transfers and effect order did not change. The renewed LLM judgment
finds no contradictory capture lifetime or exception identity. Structural
comparison supports this reading rather than supplying behavioral proof.

The [fresh monitor/binding challenge](../../sync/monitor-surface-boundaries/review.md)
covers capture identity, receiving/value flow, nested failures and escaping
storage. Current fixture/model regressions are separate executed evidence;
this record does not claim VM/device execution for this input.

## Prior verdict — terminal-follow conjunction, 2026-11-09

`cleanup` now prints `if (p0 == 3 || p0 == 4) throw new
IllegalStateException("cleanup"); else return;`. The two membership outcomes and
the original throw/return paths are unchanged; the rest of the fixture text is
unchanged in the candidate comparison.

## Scope and verdict

Compared the same prepared DEX on dd70d20 and this candidate through ordinary
Engine delivery. Direct LLM reading covers Java, authoritative Smali, concrete
helpers and every independent README scenario. The repair generalizes the
ComplexSwitchNested.run trigger without cloning code, changing original loop
continuations or moving selected assignments. This is a bounded source/product
judgement, not executed behavioral PASS; no executor/device/Java run occurred.

## Shared ownership and normal paths

route retains row/sum0, the head test, bonus carrier preparation and one original
selector per iteration. Case0 marks10+row and adds1; case1 owns its inner loop
and typed recovery. Both normal completions reach A once, adding3. Case2 prepares
bonus+row and bypasses A; case3 directly assigns its bonus carrier into the Phi
storage and reaches B. B's mark50+row precedes row increment and the next head.

With two rows, case0 therefore returns8; case2 overwrites the total with7 then8
rather than accumulating; case3 returns7 after both direct assignments. Those
selected assignments are executable steps, not origin-only copy requirements.
The other cases' gotos target B's original external label, not case3's header,
so they do not reset the prepared sum to bonus. Copies remain on their original
edges and have one original anchor. The empty case3 child completes to B, not A.
Default marks90 and breaks the outer while before either tail or update. Empty
or negative row counts skip selection and execute only cleanup70.

In case1 normal inner0/1 reads return1/2 in the first row and11/12 in the second.
Exhaustion includes A in both rows, returning32. A cut at0 exits after each first
read, skips the remaining read, inner update and A, and returns12. A cut at1
includes both reads but skips A, returning26. The inner continue still returns
to its actual head; the outward original goto runs B before row update. No for
form, update header or invented continue target is introduced.

## Recovery, faults and finalization

Mode1 throws IAE at keys0/10 after their mark. The original typed catch marks30
and adds5, then continues to the same inner comparison/update. Together with
keys1/11 and A, the two rows return30. The catch, its capture and both value
routes remain inside case1. It is not treated as a terminating arm; ordinary
read/typed-handler obligations retain their original ordered c1 disclosure.

Mode2 reaches Error at key1 after the first read/update and second mark. It
bypasses the typed IAE catch, A, B and row increment, and runs the outer cleanup
before rethrow. Mode4 reaches the same prefix but cleanup ISE replaces Error.
With cut0, key1 is unreachable: mode2 returns12, and mode4 has only a cleanup
fault after both successful first reads. Mode3 on case0 replaces normal result8
only after its normal marks and updates. The single canonical cleanup retains
both original call anchors and the pending-rethrow terminal's implicit anchor.
No cleanup is moved under the inner typed catch or duplicated on scope exit.

The baseline already exposes these effects with raw selector gotos and explicit
shared labels. The candidate improves ownership/display, not the helper model:
case1's complete Loop/Try products become a private case child, while A/B have
one outer owner. Whole-scope, original-entry, construction and copy checks still
apply; normal dominance/SCC facts are not recomputed.

## Foreign-entry refusal

foreign is unchanged. The pre-selector flag branch and selected case0 reach one
original mark10 block. Natural case ownership would hide that extra entrance,
so selection stays explicit rather than cloning it. Flagged paths return11
regardless of key. Normal case1 returns12, case2 bypasses A and returns10, and
default returns-1 before both tails. Each mark and arithmetic tail remains once.
Existing literal case-entry cycles and protected raw-selector cycles also retain
their original explicit layouts under the regression checks.

## Evidence limits and test maintenance

Six ordinary-Engine checks inspect parent/child ownership, original continuations,
actual selected assignments, source anchors and refusals. Baseline source with
the same prepared DEX/new tests passes one refusal and fails five recovery/display checks.
This is product sensitivity only. A test initially referenced EmissionControl.id;
that type is indexed rather than carrying an ID, so the test was corrected to
use its arena index before passing. No scenario expectation changed.

Three older checks assumed the previous raw/shared-negative layout. Current
checks require the newly established P6 case interface and one external tail,
while preserving single-completion checks for every printed unconditional exit.
The genuine raw cyclic selector still requires explicit gotos. Former raw
ComplexSwitchNested and fetch checks move to positive ownership/exit coverage;
they are not exempted from original transfer/copy/marker checks. These source
readings and structural checks do not independently execute the final plan.

## Integration copy-phase correction

The concurrent multi-site inputs exposed a real candidate-induced display
regression: an owned If's empty false child overrode the earlier copy-aware
classification and selected NoElse despite executable false-edge assignments.
Those assignments appeared between the predicate and opening brace, leaving its
planned marker-return jump unconditional. That draft was rejected; clean counts
or updated raw-dispatch assertions were not acceptance.

P7 now requires the same quiet-copy fact for this late owned-child decision.
P9 remains unchanged and consumes Both, so each edge's assignments execute
inside its chosen arm. The additional conditional challenge was authored after
this correction, before its first inspection. It has two copied values and two
external suffix entries: case0/flag0 executes only B and returns5 for7,2;
flag1 executes A/B and returns7. Case1 returns8, case2 returns4, and default
returns-9 with no marks. MAX_VALUE plus1 wraps before subtracting1, preserving
MAX_VALUE. All selected copy anchors remain once and false copies follow the
else brace. The exact same final test/DEX fails on draft835d78a with NoElse,
then passes with Both; this is product sensitivity, not behavioral execution.

The concurrent dispatch tests now accept either explicit original gotos or a
producer-supplied case whose entry/follow equals the original target, never raw
fallthrough. They retain call counts, captures, snapshots, marker/cleanup routes
and refusal of multi-site source-body projection. Existing helper/scenario
expectations were not changed to absorb a mismatch.
