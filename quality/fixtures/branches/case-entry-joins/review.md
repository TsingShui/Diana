# Fixture: case-entry-joins

## Current verdict — terminal guard follow-up

Read the changed terminal alternatives and remaining body. The selected
return/throw keeps its original evaluation, while other copies, calls, accesses
and results remain on the opposite path in their original order. Both-edge guard
orientation preserves the predicate's exact meaning; no data, check or effect is
dropped.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — whole-operation follow-up

Read route's default, case joins, typed recovery and cleanup paths. The
normal-cleanup bypass now exits the selected Try rather than jumping to a hidden
duplicate: read/recovery is skipped only on the original bypass, cleanup runs
once, and later normal work stays outside. Existing case-entry DAG joins retain
one body and nonadjacent gotos. Parameter-entry copies, literal cycles and
non-case sharing retain their explicit guards.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


## Prior verdict — protected-entry follow-up

Java/Smali, helpers and final route(III)I were reread. P6 now retains the valid
single-entry typed read core instead of absorbing a lone optional goto and losing
that entry. The natural typed catch stays inside the outer catch-all; prefix
marks1/2/9 and flag marks4/5 remain outside the typed body, with only outer cleanup
protection. All original clause priorities and pending rethrow routes remain.

Normal/default and typed results remain11/-9/-1 and110/90, with cleanup before
mark40/return. ISE/Error bypass typed recovery, clean and propagate; mode5 normal
cleanup failure does not retry, mode6 replaces Error with IAE. The default goto
still reaches normal cleanup outside the body catch. Other fixture method text
is unchanged and reuses the prior case-entry/copy/cycle judgments.

The fresh [prefix-frontier challenge](../../exceptions/protected-prefix-frontiers/review.md)
requires a common prefix for TWO read sites and retains a genuine multi-entry
negative. This follow-up is bounded direct LLM reading, not executed behavioral
PASS. No device run or Java recompilation ran.

## Prior LLM semantic review

Direct comparison of Java explanation, actual Smali, concrete helper bodies and
final ordinary Engine output. Each additional subject has independent expectations
before its first inspection. The same DEX is consumed on the recorded pre-case-DAG
working tree based on `ebbc003` and the candidate. No Java recompilation, device
run or behavioral executor ran.

| Original method, owner Lfixture/CaseEntryJoins; | Judgment |
|---|---|
| `choice(II)I` | Equivalent; three original predecessors enter one conditional case body without executing unrelated case prefixes. |
| `parameterJoin(II)I` | Equivalent explicit graph; the real direct-entry parameter copy remains selected-only. |
| `literalCycle(I)I`, `cyclic(II)I` | Equivalent original cycles; no acyclic case schedule or generated phase state is invented. |
| `nonCaseTail(II)I` | Equivalent explicit sharing of a tail that is not a selector entry. |
| `route(III)I` | Equivalent unchanged protected recovery/cleanup paths; not positive coverage of the new ordering. |
| `mark`, `matches`, `read`, `cleanup` | Same arguments, event-before-result/throw rules and wrapping arithmetic. |

### Fresh positive case

The initial protected route comparison already placed its shared case3 work after
the outer switch, and therefore needed no new ordering. It remains a guard, not
claimed generalization. Choice was then added with expectations before inspection:
three predecessors, an independent case4, a conditional shared body and later
mark50 exercise the actual case-entry DAG mechanism.

Choice's case1/2 mark their own prefix then jump to the same L11. They skip both
case4 and case5, so no mark4/5 is accidentally executed. Case5 marks5 and falls
through; direct case3 skips mark5. The shared body marks3 once, then flag0 marks6
and selects10; other flags mark7 and select20. All paths then mark50 and return.
Case4 independently marks4/selects40; default marks9/selects-1. The recorded
trace orders and values follow directly, with no cloned shared mark or dispatcher.

The selector labels still execute the same original key choice once. Static
case order changes printed succession, not runtime selector behavior. Only the
last ready predecessor is adjacent to case3; other arrivals keep original gotos
and its one retained label.

### Copy, cycle and non-case guards

parameterJoin's selected case3 edge actually contains an executable Phi copy from
p1. A case1/2 arrival instead copies its changed seed before the common addition.
It remains explicit: seed7 returns11,12,10 for keys1,2,3 with marks[1,3],[2,3],[3].
No direct-entry reset runs on a predecessor's arrival. Default returns-1 after
mark9. This guard checks a delivered nonempty copy, not an assumed barrier.

The initial cyclic subject also has parameter-entry copies, so its refusal alone
would not isolate the cyclic-case requirement. Added literalCycle records its own
expectations before inspection and uses an original constant2. Its delivered
selector copies have no execution steps or assignment issues. It still remains
an irreducible original cycle: key1 marks1,2,1; key2 marks2,1,2; default marks9 and
returns2. No explanation-only Java phase variable enters the DEX result. The
parameterized cyclic subject retains its own original count and initial copies.

nonCaseTail's failed matches share -2 code that is not any selector entry. That
arbitrary tail remains explicit rather than being duplicated or treated as an
ordinary case fallthrough. Matches records81/82/83 before its comparison;
success selects10/20/30, failure selects-2, unknown key selects-1 without a call.
The earlier completed-tail regression remains covered by its original decorated
fixture, whose similar-looking shared default is also not a selector entry.

### Protected recovery and replacement

route text is unchanged. Keys1/2 mark their prefix then enter the shared read;
key3 enters directly; default marks9/selects-1 without reading. Successful read
selects1, then the inner flag switch marks4/adds10 or marks5/subtracts10. Normal
cleanup marks30 before mark40/return, giving11 or-9. Typed NFE/NPE recovery marks20
and selects100 before the same flag work, giving110 or90.

ISE/Error bypass typed recovery, run cleanup and preserve the pending object
unless cleanup throws. Mode5's normal cleanup failure is outside the body handler
and cannot retry; mode6 cleanup replaces Error with IAE, without suppression or
mark40. Default still runs cleanup, including a mode5 failure. Concrete original
site chains and the ProtectionBoundary graph remain; no unsupported finally claim
is made to hide those routes.

## Owning repair and prior effects

The P6 case partition already proved disjoint exact bodies and original entry
relationships, but its order constructor permitted only one predecessor. It now
counts all original case predecessors, visits roots in original selector order,
and follows an entry adjacent only when its last predecessor is placed. Each
case body still has one next-case target, and the case graph must be acyclic.
Unknown/foreign entries, split ownership/construction and executable or unavailable
selector copies keep existing admission behavior. No analysis or boundary is retried.

P7/P9 consume the delivered order and original transfer plans. A nonadjacent
arrival keeps its actual case label; only adjacent placement becomes Next. No
external hash/equality implementation is inferred. The string seed therefore
retains all calls/collision/default/null behavior and add-input versus mul-literal
field identity, while its original integer dispatch now has readable case bodies.
The sharedTail seed keeps one mark3 with an original nonadjacent goto; its obsolete
explicit-graph qualification was replaced by a source/body/target assertion.
Parameter/mixed-view and every previous recovery guard remain.

Within these reviewed paths the iteration is effective: the new three-predecessor
conditional body generalizes the seed, and actual copy-free-cycle, parameter-copy,
non-case and protected guards retain their facts and behavior. This is bounded
manual evidence, not universal semantic verification or commit authorization.

## Supporting evidence and limitations

Four new package checks consume ordinary Engine delivery: exact shared-entry
gotos/body count, actual selected-entry Phi copy, a cycle with empty selector
copies, and non-case/protected fragments. All original calls/captures keep one
anchor. These are structural/product checks, not execution of the scenario table.

Final default and all-feature nextest each passed 51 checks. Package Clippy with
warnings denied, release and dbg_shell builds, format/diff and desensitization
checks passed. Corpus comparison covers 83 fixtures,134 classes,635 methods with
no stopped member or render failure/issue. Only the three reviewed fixture texts
change. Original P2/P3/P4/P5 and relevant P6 control/lock facts, selected targets
and phases agree; only case ordering/ownership/transfers and derived layouts
intentionally change. No behavioral execution ran. Faults and
concurrency outside the README model and unreviewed arbitrary inputs remain outside
this judgment.
