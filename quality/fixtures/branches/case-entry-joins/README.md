# Several original predecessors of one case entry

## Independent expectations

Authored Java/Smali, no third-party input. Seeds: branch-switch-string's original
integer dispatch and switch-fallthrough.sharedTail. The initial repair orders
an acyclic case-entry graph once, keeping nonadjacent predecessors as explicit
gotos to the same printed case label. It does not infer an external string
protocol or clone a shared body.

The fresh route subject adds typed recovery, a nested switch and enclosing
cleanup/replacement around the shared case body. The parameter guard retains
selected-entry copy restrictions; the cyclic guard keeps a genuine cycle between
two selector entries. Original owner: `Lfixture/CaseEntryJoins;`. Subjects:
`choice(II)I`, `route(III)I`, `parameterJoin(II)I`, `cyclic(II)I`,
`nonCaseTail(II)I`, `literalCycle(I)I`. Concrete mark/read/cleanup/matches bodies
are in the DEX. Java's implicit constructor is not a Smali subject.

Trace begins at zero; mark and arithmetic wrap as int. Standard exception
construction succeeds. No asynchronous/linkage/allocation fault or concurrent
trace writer is modeled. In route, both original case1 and case2 enter case3;
Java explains the prefixes separately to avoid a synthetic goto/state machine.
There is only one original read/catch/nested-switch body.

| Subject and arguments | Expected exit | Ordered mark events |
|---|---|---|
| choice(1,1) | return 20 | 1,3,7,50 |
| choice(2,0) | return 10 | 2,3,6,50 |
| choice(3,0) | return 10 | 3,6,50 |
| choice(5,0) | return 10 | 5,3,6,50 |
| choice(4,0) | return 40 | 4,50 |
| choice(99,0) | return -1 | 9,50 |
| nonCaseTail(1,1) | return 10 | 81 |
| nonCaseTail(2,0) | return -2 | 82 |
| nonCaseTail(3,3) | return 30 | 83 |
| nonCaseTail(99,0) | return -1 | none |
| route(1,0,0) | return 11 | 1,10,4,30,40 |
| route(2,1,0) | return -9 | 2,10,5,30,40 |
| route(3,0,0) | return 11 | 10,4,30,40 |
| route(1,0,1) or route(1,0,2) | return 110 | 1,10,20,4,30,40 |
| route(2,1,1) | return 90 | 2,10,20,5,30,40 |
| route(99,0,1) | return -1 | 9,30,40; no read/catch mark |
| route(1,0,3) | throw original ISE("row") | 1,10,30; no nested flag work/mark40 |
| route(1,0,4) | throw original Error | 1,10,30 |
| route(1,0,5) | throw normal-cleanup IAE | 1,10,4,30; no mark40 |
| route(1,0,6) | throw IAE replacing Error | 1,10,30; no suppression/retry |
| route(99,0,5) | throw normal-cleanup IAE | 9,30 |
| parameterJoin(1,7) | return 11 | 1,3 |
| parameterJoin(2,7) | return 12 | 2,3 |
| parameterJoin(3,7) | return 10 | 3 |
| parameterJoin(99,7) | return -1 | 9 |
| literalCycle(1) | return 0 | 1,2,1 |
| literalCycle(2) | return 0 | 2,1,2 |
| literalCycle(99) | return 2 | 9 |
| cyclic(1,2) | return 0 | 1,2,1 |
| cyclic(2,0) | return 0 | 2 |
| cyclic(99,3) | return 3 | 9 |

Additional subjects were recorded before their first inspection: choice has
THREE predecessor entries (1,2,5) into case3, a conditional shared body and an
independent case4, followed by mark50. Only one predecessor may be placed adjacent;
others must skip unrelated prefix/body effects with the original goto. The initial
protected route comparison already places case3 content after its outer switch,
so it remains a protection/recovery guard rather than claimed positive coverage.

nonCaseTail deliberately shares a failure tail that is NOT a selector case entry.
It records one concrete matches event before selecting 10/20/30 or -2; unknown
key selects -1 without a call. This work does not duplicate or turn arbitrary
non-case sharing into fallthrough. It also retains the earlier completed-tail
regression independently of the now-structured case-entry examples.

parameterJoin keeps the direct case3 seed distinct from either mutated predecessor.
When its delivered selected-entry Phi copy is executable, natural case-entry
scheduling must decline rather than executing that copy on another case's arrival.
If the prepared input fails to establish that obligation, it is effect evidence
only, not a passing copy guard. Cyclic has two genuine selector arrivals into its
cycle; an acyclic case schedule must refuse it while retaining the original code.
The added literalCycle records expectations before its first inspection and
starts from an original constant2, not a pinned parameter. It is intended to keep
selector entry copies empty so the cyclic-case refusal is exercised independently
of parameter copies. That prerequisite must be inspected, not assumed from the
absence of a natural layout. The Java phase local only explains alternating
Smali labels; it is not DEX state and must not be introduced by the engine.

Natural grouping must retain each original key/default exactly once, one shared
body, collision/condition tests, selected-edge copies and all exception/effect
order. A case entry may have several original predecessors; only actual adjacent
placement can become implicit Next. Nonadjacent arrivals keep their original
label/target, without an alternative boundary search or dispatcher.

These expectations precede first baseline/candidate inspection. No behavioral
execution result is claimed.

## Preparation

```sh
mkdir -p .cache/fixtures/case-entry-joins
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/case-entry-joins/classes.dex \
  quality/fixtures/branches/case-entry-joins
cp .cache/fixtures/case-entry-joins/classes.dex \
  quality/fixtures/branches/case-entry-joins/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/branches/case-entry-joins/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and OpenJDK 21.0.12.1. Baseline is the recorded
working tree based on `ebbc003` after completed-tail display recovery and before
acyclic case-entry join scheduling. Both comparisons consume the same prepared
DEX. Choice exercises the new schedule; route already has a following shared body
and remains a protection guard. ParameterJoin establishes an executable direct
case3 copy, while literalCycle establishes empty selector copies and still retains
a cyclic graph. Non-case sharing remains explicit. See review.md for the direct
reading judgment and unrun behavioral execution.
