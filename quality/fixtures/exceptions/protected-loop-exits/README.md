# Protected loop abrupt exits

## Independent expectations

Authored Java/Smali, no third-party input. This fresh complexity challenge follows
the P6 repair for [GuardArms.loop](../../branches/terminal-guard-arms/README.md).
Expectations below precede the first inspection of this DEX on either revision.
The original single catch-all covers a loop and its abrupt corridor; terminal
throw and normal cleanup have no common normal postdominator. nested adds a
second loop, visible independent row/column updates, throwing probe/read helpers,
and two literal-argument cleanup calls. early adds a protected return which must
NOT execute cleanup. foreign adds a genuine pre-loop entrance to the throw tail;
that tail must not become private to the loop or be cloned.

Owner: Lfixture/ProtectedExits;. Subjects nested(IIII)I, early(III)I,
foreign(IIII)I, recover(III)I and switchExit(IIII)I. Smali is authoritative. Java explains equivalent behavior,
including the shared throw target in foreign. Helpers are concrete in this DEX:

- mark(event) appends trace*31+event using wrapping int arithmetic; trace starts0.
- probe(key,stop,mode) marks100+key, throws NFE("probe") when mode1 and key1,
  otherwise returns1 iff key==stop, else0.
- read(key,mode) marks200+key, throws AssertionError("read") when mode2/5 and
  key1, otherwise returns key+1.
- cleanup(code,mode) marks300+code, throws IAE("cleanup") when mode3 and code7,
  or when mode4/5 and code8. Cleanup7 precedes cleanup8; a fault skips later work.

Constructors/allocation succeed in these scenarios. No asynchronous/linkage
fault, concurrent trace writer or external declaration model is assumed by the
engine. The loop scenarios below terminate; integer operations retain DEX
32-bit wrapping. No behavioral executor, device or Java execution is claimed.

| Subject / arguments | Expected exit | Ordered mark arguments |
|---|---|---|
| nested(2,2,-1,0) | return26 | 100,200,101,201,110,210,111,211,307,308 |
| nested(2,2,1,0) | throw guard ISE before second read/update | 100,200,101,307,308 |
| nested(2,2,10,0) | throw guard ISE at second row | 100,200,101,201,110,307,308 |
| nested(2,2,-1,1) | throw probe NFE | 100,200,101,307,308 |
| nested(2,2,-1,2) | throw read Error | 100,200,101,201,307,308 |
| nested(2,2,-1,3) | throw first-cleanup IAE after body | 100,200,101,201,110,210,111,211,307 |
| nested(2,2,-1,4) | throw second-cleanup IAE after body | 100,200,101,201,110,210,111,211,307,308 |
| nested(2,2,-1,5) | throw second-cleanup IAE replacing Error | 100,200,101,201,307,308 |
| nested(0,2,-1,5) | throw second-cleanup IAE; no body Error is reachable | 307,308 |
| nested(2,0,-1,0), nested(-1,2,-1,0) | return0 | 307,308 |
| early(3,1,0) | return1; no cleanup | 200 |
| early(3,0,5) | return0; cleanup fault is unreachable | none |
| early(2,-1,0) | return3 | 200,201,307,308 |
| early(2,-1,2) | throw Error | 200,201,307,308 |
| early(2,-1,5) | throw IAE replacing Error | 200,201,307,308 |
| early(0,-1,0) | return0 | 307,308 |
| foreign(2,-1,0,0) | return3 | 100,200,101,201,307,308 |
| foreign(2,1,0,0) | throw guard ISE | 100,200,101,307,308 |
| foreign(2,-1,0,1), foreign(0,-1,0,1) | throw guard ISE before loop | 307,308 |
| foreign(2,-1,5,1) | throw second-cleanup IAE replacing ISE | 307,308 |

nested must expose two while loops under one finally, one guard construction and
throw, and cleanup7/8 once in that order. An inner failure skips its read (for a
probe/guard fault), both pending updates and all later iterations. Early return
in early deliberately has no duplicated normal cleanup: moving its post-loop
cleanup into finally would change the first two early scenarios. Its natural
catch and private return are acceptable; a matched finally is not. foreign's
pre-loop and body entrances retain one shared construction, exact original
targets and copies. Explicit protection there is a valid refusal, not a defect.

## Additional recovery refusal

Authored after the first three subjects' inspection, before recover's first
inspection on either revision. recover adds a per-iteration catch-all which
cleans up, advances row by2 and resumes the loop. Normal iterations advance by1.
A recovering handler is not an abrupt enclosing owner; its cleanup must stay at
the original recovery point, not become one post-loop finally.

| Arguments | Expected exit | Ordered mark arguments |
|---|---|---|
| recover(4,1,0), recover(4,-1,1) | return5 | 100,200,101,307,308,103,203 |
| recover(4,-1,2) | return5 | 100,200,101,201,307,308,103,203 |
| recover(4,1,4) | throw IAE before recovery row+2 | 100,200,101,307,308 |
| recover(0,1,4) | return0; cleanup fault unreachable | none |

Reassemble once for the final comparison of all subjects; both binaries consume
that same prepared DEX. This is a compatibility/refusal case, not a second
positive repair witness.

## Raw-selector regression challenge

Authored after fixing the candidate-induced raw-default regression in
ComplexSwitchNested.run, before switchExit's first inspection. Its protected
loop has a private default exit with mark90 before normal cleanup. Cases0/1
form an actual case-entry cycle; they cannot acquire the case-entry DAG layout.
Raw selector arms must keep explicit original gotos, including a default whose
target happens to be the next outer Sequence child. Otherwise default would
fall into case0. Each iteration resets count0; case0 marks10, adds1, increments
count then enters case1; case1 marks20, adds2, increments count then repeats
case0 while count<bound, else updates row. Case2 marks30/adds3 then updates row.
No original selector, mark, update or copy may be duplicated.

| Arguments | Expected exit | Ordered mark arguments |
|---|---|---|
| switchExit(2,0,3,0) | return12 | 10,20,10,20,10,20,10,20,307,308 |
| switchExit(2,1,3,0) | return10 | 20,10,20,20,10,20,307,308 |
| switchExit(2,2,3,0) | return6 | 30,30,307,308 |
| switchExit(2,3,3,0), switchExit(2,-1,3,0) | return0 via private default exit | 90,307,308 |
| switchExit(1,0,0,0) | return3 | 10,20,307,308 |
| switchExit(2,3,3,3) | throw first-cleanup IAE | 90,307 |
| switchExit(1,1,3,4) | throw second-cleanup IAE | 20,10,20,307,308 |
| switchExit(0,3,3,5) | throw second-cleanup IAE; default not reached | 307,308 |

## Preparation

Assemble with Smali2.5.2 and API24 under the ignored fixture cache, then copy only
the successful classes.dex beside these sources. Compare that same prepared
DEX on baseline73542421d671be2fc169b72c7233aa0b5b955fd5 and the candidate through
ordinary dbg_shell p6/p9. Prepared with Smali2.5.2, API24 and OpenJDK21.0.12.1.
Five ordinary-Engine checks pass on the candidate. On the same prepared input and
baseline source, the two refusal checks pass and three recovery/transfer checks
fail; this is product sensitivity, not behavioral execution. The
[manual review](review.md) records the path-by-path judgement and rejected draft.
