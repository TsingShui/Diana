# Switch shared-tail challenge

## Independent expectations

Authored after the owning P6 repair, before this input's first inspection on
baseline or candidate. Discovery: [ComplexSwitchNested.run](../../composite/complex-switch-nested/review.md).
Its first shared non-case tail has downstream entries bypassed by other cases.
This challenge adds a typed recovery inside a case-local loop, outer finally,
a direct selector entry to a downstream tail with a parameter-to-Phi assignment,
and an actual foreign entrance to a private case. Cases and tails must not be
cloned. Smali is authoritative; Java explains the same behavior.

Owner Lfixture/SharedTails;. Subjects route(IIIII)I and foreign(II)I. trace starts0;
mark appends trace*31+event with wrapping int arithmetic. Concrete read(key,mode)
marks100+key, then mode1 at key%10==0 throws IAE("read"); modes2/4 at key1 throw
AssertionError("read"); otherwise it returns key+1. cleanup(mode) marks70 then
modes3/4 throw ISE("cleanup"). Constructors/allocation succeed; no asynchronous,
linkage or concurrent mutation faults are modeled. These finite scenarios do not
establish termination for arbitrary inputs or execute the final emission plan.

route initializes row/sum0. Before each selector, carry refers to the unchanged
bonus parameter. Case0 marks10+row/adds1 then enters tail A. Case1 loops inner0/1:
read(row*10+inner), add its result, or catch IAE/mark30+inner/add5; a matching cut
copies the current sum into carry and enters B without incrementing inner or
running A. Otherwise inner increments; exhaustion enters A. Case2 marks20+row,
sets carry=bonus+row and enters B. Case3 directly enters B with carry=bonus.
Default marks90+row and exits the outer loop. Tail A adds3 and sets carry=sum;
B copies carry to sum, marks50+row, then increments row. Finally cleans once.

| Subject / arguments | Expected exit | Ordered mark arguments |
|---|---|---|
| route(2,0,-1,7,0) | return8 | 10,50,11,51,70 |
| route(2,1,-1,7,0) | return32 | 100,101,50,110,111,51,70 |
| route(2,1,0,7,0) | return12; both iterations skip A | 100,50,110,51,70 |
| route(2,1,1,7,0) | return26; both iterations skip A | 100,101,50,110,111,51,70 |
| route(2,1,-1,7,1) | return30 after typed recovery on each inner0 | 100,30,101,50,110,30,111,51,70 |
| route(2,1,-1,7,2) | throw original Error before A/B/update | 100,101,70 |
| route(2,1,-1,7,4) | throw cleanup ISE replacing Error | 100,101,70 |
| route(2,1,0,7,2) | return12; key1 fault is unreachable | 100,50,110,51,70 |
| route(2,1,0,7,4) | throw cleanup ISE; no body Error | 100,50,110,51,70 |
| route(2,0,-1,7,3) | throw cleanup ISE replacing normal result8 | 10,50,11,51,70 |
| route(2,2,-1,7,0) | return8, not accumulated prior total | 20,50,21,51,70 |
| route(2,3,-1,7,0) | return7 via original selector assignment each time | 50,51,70 |
| route(2,4,-1,7,0), route(2,-1,-1,7,0) | return0, no tail or update | 90,70 |
| route(0,1,-1,7,2), route(-1,1,-1,7,2) | return0; body fault unreachable | 70 |

The selected case3 assignment executes once at its header, not on case2's or
case1's jumps to original B. Those jumps must skip A and must not reset carry to
bonus. Natural while continue targets the actual head, not B's update work.
Outer cleanup retains normal/result/replacement order; typed recovery stays
inside case1 and does not convert the entire case into a terminal guard.

foreign's pre-selector conditional enters the same case0 block directly when
flag!=0. There is one mark10 and one original tail, not a duplicated special-case
body. Cases0/1 enter A (+3), while case2 bypasses A and enters B (+7). Default
returns-1 without either tail. This real foreign entrance must not be claimed
private to the switch.

| Arguments | Expected result | Ordered mark arguments |
|---|---|---|
| foreign(0,0), foreign(2,1), foreign(3,1) | return11 | 10 |
| foreign(1,0) | return12 | 20 |
| foreign(2,0) | return10 | 30 |
| foreign(3,0) | return-1 | none |

## Copy-only conditional follow-up

Added conditional(IIII)I after the owning P7 correction and before this subject's
first inspection. Concurrent multi-site ABI fixtures exposed an empty child
with executable false-edge assignments: the late empty-child display decision
must not select NoElse or print those assignments between if and its brace.
This adds two copied values, both external suffix destinations and wrapping.

The initial pair is a,b. Case0 chooses B directly when flag0 and A otherwise.
Case1 changes it to a+1,b+2 then reaches A; case2 changes it to a+7,b+8 and
reaches B; default returns-9. A marks60/adds the second value into the first;
B marks61 then returns first-second. Branch assignments must execute only on
the selected edge; neither target may become an unconditional jump.

| Arguments | Expected result | Ordered mark arguments |
|---|---|---|
| conditional(0,0,7,2) | return5 | 61 |
| conditional(0,1,7,2) | return7 | 60,61 |
| conditional(1,0,7,2) | return8 | 60,61 |
| conditional(2,0,7,2) | return4 | 61 |
| conditional(3,0,7,2) | return-9 | none |
| conditional(0,1,2147483647,1) | return2147483647 after wrapping addition/subtraction | 60,61 |

Reassemble and use that same final DEX on baseline, rejected copy-display draft
and candidate. This is a new same-mechanism challenge, not a scenario inferred
from candidate output.

## Preparation

Assemble using Smali2.5.2/API24 in the ignored fixture cache, then publish the
successful classes.dex beside these sources. Compare that same prepared DEX
through ordinary dbg_shell on baseline dd70d20 and the candidate. Preparation,
inspection and checks use that same DEX. Prepared with Smali2.5.2, API24 and
OpenJDK21.0.12.1. Six ordinary-Engine checks cover this input and the discovery/ABI cases. The
same final tests/DEX on baseline source pass the foreign-entry refusal and fail
five recovery/display checks;
this is structural sensitivity, not a behavioral mismatch claim. The copy-only
conditional check separately fails on the rejected shared-tail draft835d78a with
NoElse rather than Both, directly exposing its missing copy-phase guard. See
[review](review.md) for the bounded semantic judgement. No Java/device/scenario
execution is claimed.
