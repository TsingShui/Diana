# Quiet versus executable final-other copy phases

## Independent expectations

Authored AFTER the origin-only copy-arm repair and BEFORE first inspection.
Discovery: walk(III)I in
[mutable-iteration-step](../../loops/mutable-iteration-step/README.md). The retained
trigger is a compact no-body other arm with complete zero-step P5 obligations.
Added interaction: two consecutive conditions, a call-produced selected value,
visible uses before/after the join and a parameter-to-Phi assignment that must
NOT disappear. Actual P5 steps must establish both copy roles before this fixture
can claim challenge coverage.

Owner Lfixture/QuietCopies;. Subjects pair(IIII)I, seed(I)I and mark(I)V. Helpers
exist in the DEX. seed marks base+30 then returns base+1; mark updates trace*31+
event. All int arithmetic wraps32 bits. trace starts0; no asynchronous/linkage/
allocation fault or concurrent trace mutation is modeled. No constructor subject.

| pair arguments (base,a,b,c) | Expected return | Ordered mark arguments |
|---|---|---|
| (3,0,0,0) | 7 | 33,4,4,3 |
| (3,1,0,0) | 14 | 33,4,11,3 |
| (3,0,1,0) | 51 | 33,4,11,40 |
| (3,0,0,1) | 44 | 33,4,4,40 |
| (-2,1,0,1) | 46 | 28,-1,6,40 |
| (2147483647,0,0,0) | -1 | -2147483619,-2147483648,-2147483648,2147483647 |

The first false condition must preserve selected's original value with no
invented assignment. The second false condition must initialize the distinct
choice Phi from base before its mark/return, not reuse the changed selected.
Short-circuit order and one original copy/transfer anchor per selected edge stay.
Existing mutable-step no-body boundary transfers must still print even when
their copy lists are empty. No behavioral execution or Java recompilation is
claimed.

## Literal-origin positive challenge

Initial pair has real executable assignments on BOTH final-other edges because
the call result and arithmetic result have distinct P5 storage. It remains a
guard, not claimed positive evidence. Added quiet(IIII)I with independent
expectations BEFORE its first inspection: seed still executes once, but selected
starts at the original literal0 before the first condition. Its second condition
retains the same parameter-Phi pressure. This requires the existing P5 literal
storage reuse guarantee, not a change to coalescing or a deleted copy.

Expected quiet arguments/return/events: (3,0,0,0) returns3 with33,0,0,3;
(3,1,0,0) returns10 with33,0,7,3; (3,0,1,0) returns47 with33,0,7,40;
(3,0,0,1) returns40 with33,0,0,40; (-2,1,0,1) returns47 with28,0,7,40;
(2147483647,0,0,0) returns2147483647 with-2147483619,0,0,2147483647.
The unused seed result must not remove its call/event.

## Preparation

Assemble once using the repository Smali procedure, API24, into the ignored
fixture cache; copy only classes.dex here. Compare that same DEX on the recorded
pre-repair working tree based on ebbc003 (after ten repairs) and candidate through
ordinary dbg_shell. Prepared with Smali2.5.2, API24 and OpenJDK21.0.12.1.
Actual P5 quiet's first final-other plan0 is complete with a nonempty identity
requirement and zero steps; second plan2 has one Assign from the original base
parameter. Pair has actual assignments at both joins. The same prepared DEX is
used by both binaries. Candidate quiet removes only the origin-only empty else;
pair and the executable second else remain unchanged. See review.md.
