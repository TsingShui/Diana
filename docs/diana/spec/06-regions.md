# P6 control, protection and monitor plans

[PIPELINE_DESIGN §5.6](../PIPELINE_DESIGN.md#56-p6-natural-regions-first-a-java-like-control-plan-with-explicit-transfers-preserved)
defines the product obligations. This chapter describes their implementation in
`crates/diana-decomp/src/p6/`. Capability observations belong in the
[engine README](../../../crates/diana-decomp/README.md).

## Boundary and failure behavior

Engine's method task calls `build_regions(task, storage)` once. P6 consumes a
mature `StorageMethod`, privately retains it, and publishes `PlannedMethod` or
`RegionStop`. Only the owning constructor has publication authority.

`PlannedMethod::facts()` supplies `RegionFacts`. `storage_facts()`, `type_facts()`
and `execution()` provide the established facts needed by P7. Borrowed
`storage_evidence()` and `analysis_evidence()` support inspection without extracting
an owned predecessor or exposing another analysis entry. P6 MUST NOT use the
retained SSA inspection view to reconstruct missing storage facts.

The P5 execution graph has known operation boundaries, terminals and targets.
Local storage gaps do not invalidate that graph. P6 records affected accesses and
continues planning independent regions. A `Graph` region denotes known explicit
control, not an invented target for unknown control. P6 MUST NOT replace an
unavailable operand with a default, omit an effect, or re-run an earlier solver.

A region mismatch rejects that composition, not the method. Resource failures and
necessary construction-invariant failures produce a classified `RegionStop` with
the complete P5 product and a separate, unpublished `RegionPartial`. The partial
arena is not `RegionFacts`. A method-private stop does not cancel sibling tasks.

## Planning state and published diagnostics

During construction, P6's private issue accumulator includes `PlanningIssue::Graph`
records for candidate rejection and graph selection. P6 consumes those records to
select irreducible graph reasons and explicit lock-separation reasons. P6 MUST
finish these computations before finalizing the published diagnostic set.

Before publishing `PlannedMethod`, P6 MUST remove all `PlanningIssue::Graph`
records from `RegionFacts::issues()`. Each surviving `RegionKind::Graph` retains
its selected reason and entry; its printed `graph<reason>` header states that
reason once. This includes `Unstructured`, `MultipleEntry`, `Irreducible` and
`ProtectionBoundary` when selected, and any other selected `RecoveryReason`.
Rejected-candidate reasons, including `MultipleEntry`, `Overlap` and `LockBoundary`
at entries retained as non-graph regions, are planning state rather than
reader-facing execution gaps. P6 MUST NOT publish them as diagnostics or header
annotations. The finalization changes no region, node, label or selected reason.

P6 MUST retain `Storage`, `Lock` and `Protection` diagnostics in their original
order and with their original identities and content. A stopped construction keeps
its unchanged unpublished `RegionPartial`, including its planning records; it is
not subject to successful-product diagnostic finalization. P7 consumes the mature
diagnostic set without reconstructing candidate history or matching issue entries
to regions.

## Original blocks and normal structure

Each original P2 block has one `BlockPlan`. Its ordered `Node` portions partition
the original operations and terminal without duplication. Each operation and its
normal result commit owns one whole portion; a terminal owns its own portion.
An empty original block retains its routing portion. Pure body completion,
cleanup argument preparation and effects therefore need no interior cursor or
new block cut to acquire different source owners. This partition is established
once before normal structure, locks, protection and region selection. Node and
Link IDs are derived coordinates, not original DEX member identities. A
`WithinBlock` link is a sequencing relation between those portions; it is not a
new DEX edge, effect, instruction or SSA definition. Original normal edges retain
their `EdgeId` and connect the original block's final portion to the target's
first portion. Reachability includes the original exceptional transfers.

P6 builds private normal incoming/outgoing indexes. Iterative Lengauer–Tarjan
computes normal dominators, with analysis-only roots for the method entry,
handlers and otherwise disconnected components. Reverse LT supplies finite-exit
postdominator candidates. A position with no path to an exit has no such candidate.
P6 MUST retain every original divergent path when checking a candidate; virtual
analysis roots do not become runtime transfers. Iterative SCC walks and
header-dominated backedges establish ordinary natural loops. The same normal DFS
also records original retreating links as candidate headers for the private-handler
entry construction below. Neither an SCC nor a retreating link alone establishes
a natural loop; normal LT relations are not recomputed over exceptional edges.

The region arena contains `Block`, `Sequence`, `If`, `Loop`, original `Switch`,
`LabelledBlock`, `Try`, `Synchronized` and explicit `Graph` nodes. Enclosing
completion interfaces constrain child construction before ownership commitment. External entrances, disjoint child
ownership, protection boundaries and P5 construction-group membership constrain
composition. Crossing candidates remain explicit rather than duplicating their
shared content. Empty branch arms contain no fabricated instruction.

Private `CompletionInterface` records retain exact node coverage, original normal entrance
and outlet LinkIds, executable exceptional entrance EdgeIds, and original abrupt
terminal NodeIds. An outlet references its actual target and unchanged P5 copies;
a terminal references its original operands, completion, phases and site through
its node. Return/throw categories do not compare values or merge terminals.
Storage availability is not a topology predicate: original accesses and CopyPlans
retain their local gaps, and dependent P7 composition still requires availability.

Private parts carry entry, continuation and membership constraints. Their common
boundary consists of actual outlet targets plus their declared normal follow.
If, Condition, Loop and Switch proposals use this boundary without classifying
an outlet as break or continue. Parent paths supply only selected scope context
when P6 later chooses transfers. All graph walks,
ownership composition and context walks use explicit stacks or finite queues.
Candidate frontiers are derived from the input; a mismatch does not restart method
analysis or launch an unbounded template search. Scratch and result allocation are
fallible and checked, without a configured numeric quota.

P6 MUST classify an original link as implicit `Next` only when selected lexical
completion reaches its actual destination. During the ancestor walk, the first
pending Sequence sibling blocks fallthrough to any outer sibling or follow unless
that pending sibling's entry is the link destination. A link bypassing that work
retains an explicit transfer. The walk still resolves enclosing break, continue
and lock exits, which can legitimately bypass the pending sibling. P7 consumes
this distinction without rechecking the graph; an unowned conditional with
separate destinations MUST NOT lose a route merely because both destinations
are reachable through enclosing completions.

## Ordinary branch boundaries

An ordinary branch first proposes its normal postdominator follow when it lies
in the enclosing part, otherwise its unique first shared original target within
that part, otherwise the part's established follow. A local shared body tail is
ordinary join topology, not a source-level update role. Both original
arm reach sets stop at the same enclosing completion boundary. Before comparing
ordinary branch arms, P6 closes each set over established `TryPlan` membership:
when the whole protected body is inside an arm and no member is its follow or
boundary, that arm also owns the plan's original handler content. The monotone
closure stops when membership no longer changes. It neither follows arbitrary
exception edges nor identifies a new cleanup pattern. Overlapping arms, foreign
entrances, split construction groups and scope crossings retain their existing
admission checks. This lets a containing branch retain an abrupt catch beside
its protected body instead of excluding the try before child selection.
The primary boundary is admissible when neither arm reaches the predicate, the arms
are disjoint, and their union with the predicate satisfies
`protection::closed_entry`. P6 MUST enqueue only that primary proposal when it is
boundary-admissible. A later owner/barrier rejection MUST NOT trigger another
boundary search or reopen an accepted owner.

Otherwise, distinct original successors supply the only alternative boundaries.
For boundary F and other successor S, A is all normal content reachable from S
without visiting F; the candidate owns only `{predicate} union A` and follows F.
The arm targeting F is empty, with no entry and follow F. The other arm enters S
and follows F. Header/true/false child positions and original link identities do
not change. P6 MUST retain the selected edge's copies even for an empty arm.
An arm empty because its original target is an enclosing completion boundary
keeps that actual target as its own follow. P6 MUST NOT replace it with the
containing If's different follow; the selected enclosing scope can then deliver
its original break/continue, or retain an explicit goto.

Before committing a successor-boundary candidate, P6 MUST establish:

- The predicate is outside A and F is outside the candidate. After contracting
  wholly contained, already-selected natural Loop regions, the remaining normal
  arm graph is acyclic. Only Loop owners are contracted; their explicit
  descendants remain in the product. An arbitrary Graph owner is not a cycle
  abstraction.
- The unchanged `closed_entry` predicate accepts all normal and executable
  exceptional arrivals. If the candidate contains method entry node 0, its
  predicate is node 0.
- Every original normal link leaving a successor-boundary candidate targets F. Original
  return/throw terminals and contained divergent loops retain their behavior;
  exceptional outlets retain their site-specific facts. Reachability is not
  clipped to a guessed enclosing scope or abrupt outlet.
- Existing lock boundaries, P5 construction atomicity, exact/disjoint parts and
  whole-current-owner containment all hold. A frontier cutting an owner rejects
  only that composition; duplicate, foreign or missing content remains a
  classified construction-invariant failure.

Before committing either a primary or successor-boundary branch candidate U,
P6 MUST require laminar membership with every established LoopPlan's lexical
candidate membership: the branch is disjoint, wholly contained, or contains that
membership in full. P6 MUST reject a partial branch that owns the loop's continue
target. A contained branch can own that loop's latch source when it owns a proved
private exit corridor and its boundary is the loop's original continue target.
It can also own an enclosing loop's particular latch source when a proper nested
loop supplies the complete private corridor containing that source: the candidate
owns its original entrance and every corridor node, follows the nested loop's
source continuation, and the corridor ends at the enclosing loop's actual
continue target. P6 retains this corridor membership/target from its single
lexical-exit construction as private ownership evidence; no public upstream view
or graph reconstruction supplies it later. Every included enclosing latch needs
that exact evidence; an unrelated private exit licenses nothing. Its arms then
complete the selected loop-body execution without acquiring an evaluation entry.
Other partial latch ownership is rejected. These constraints apply before Loop region selection, alongside unchanged
entrance, arm-reachability, whole-owner and construction checks. Rejection retains
explicit control without reopening an accepted owner.

P6 MUST preserve predicate evaluation, original arm roles, effect order and all
selected-edge copy, exception, lock and construction obligations. F has one outer
owner, not an execution copy in either arm.

One explicit-stack construction selects disjoint enclosing proposals by decreasing
membership size, then Loop/Synchronized/Try/Condition/ordinary priority, full catch
frontier, entry and original successor-boundary identity. It constructs each
selected proposal's exact parts within that interface before committing the
parent once. Every syntax-owned predicate is excluded from its descendant
proposals. A child cannot acquire an incompatible follow before its enclosing
interface is selected. Whole construction groups, contained loop plans and
natural lock scopes cannot be split across parts; the same whole-current-owner
check applies when an owner already exists.

Successor-arm acyclicity is decided once after child selection, contracting only
wholly contained selected Loop owners. A refusal assembles those same original
parts explicitly, without another boundary, solver or reconstruction attempt.
The ordinary part assembler remains the sole Sequence/Graph constructor. There
is no Graph-first retry, alternative method chain or owner reopening. Reverse LT
uses each original exit vertex; void returns do not acquire a canonical runtime
return node.

## Ordered shared-result conditions

A conditional terminal can own one `Condition` region when its two original
arms, within their common completion boundary, are nested rather than disjoint:
one arm is a proper subset of the other, and that shared subgraph has a single
entry `S` reached by one original link of the first predicate. P6 MUST retain
both original links of every test. Each later selected link reaches the same
`S` directly or through a P6-owned path of original empty goto blocks; the other
link continues into the next test's original block run or reaches the remaining
result arm. P6 MUST require each routing block to contain no operations and one
goto terminal, with one original outgoing link and no edge-copy bundle. P6 MUST
refuse a cycle, an enclosing boundary on that path, or a normal entrance other
than the condition's selected edges and internal routing links. Executable
exceptional arrivals remain subject to the child-interface check below.

Those ordered tests select between two result arms. The arm their selected
original links reach is the shared arm the region owns. The arm a link from the
tests' own side also merges inside is the second result arm, and only that
merge is the region's normal exit: when such a merge exists P6 MUST derive it
from the predicate's original links inside the enclosing completion boundary and
no other node, so the ordered tests keep one exit instead of leaving at an
unrelated established follow. When the tests' other links only leave at the
established follow, or at the last test's other original link, the region keeps
that exit and owns the arm it reaches as before.

The established follow can itself be the selected result when it begins one
straight block run ending in an actual return or throw. In that case the
condition owns that abrupt run as its shared arm and has no normal follow; the
selected links, result terminal and original block positions remain unchanged.
This covers conjunctions whose first false test jumps directly to a common
`return false` or `throw` block.

The region owns the test runs, the shared arm, the optional other arm and the
optional routing child in that source-role order. The routing child owns every
original path node and link once, including paths shared by several tests. The
shared result retains one entry; routing does not merge or replace original
links. A test run is the original block's parts from the run entry to its
terminal predicate; P6 MUST NOT move an operation out of its block or reorder
tests. A run that does not end in another conditional starts the other arm. When
the last other link leaves the region directly, the region owns no other arm and
MUST keep that original link and its copies. A matched cleanup's established
continuation can start the first test run inside an original block: its preceding
normal duplicate remains with that existing cleanup product. Without such a
supplied source entry, the original block prefix stays in the first run. This
choice occurs before owner admission, not as a retry after rejection.
P6 separately records `other_exit`
when this no-body link reaches an enclosing completion boundary different from
the common follow. Absence of an owned body MUST NOT collapse that boundary
transfer into ordinary condition fallthrough. Its classified original target and
break/continue/goto role remain available to P7.

The candidate enters the shared enclosing-interface construction and MUST satisfy the
same original normal/exceptional entrance, disjoint/exact part, whole-current-owner, construction,
lock and loop-laminarity checks. The established loop checks apply to a
`Condition` candidate exactly as to an `If`, using established lexical membership
and the narrow proved-private-exit/continue-boundary latch rule from
[ordinary branch boundaries](#ordinary-branch-boundaries). Partial overlap and
partial ownership of the continue target reject the composition. P6 MUST require each result
arm to be a closed interface: the shared arm is reachable only from its entry
inside the shared set and every link entering it is one of the tests' selected
original normal links or a supplied routing-path link, every normal link leaving it reaches the common follow or an enclosing actual outlet,
the other arm is entered normally only by the last test's other original link
and leaves normally only at that same completion boundary, and no test run is entered in its
middle. P6 MUST also inspect executable exceptional arrivals for each child,
including arrivals from another child of the same candidate. An exceptional
arrival from outside a child rejects composition unless it enters the first run
at the whole candidate's entry from outside the candidate. Exceptions internal
to one child retain its ownership; outward exceptional propagation is not a
normal follow and remains unchanged. Only the existing `ImpossibleRelease`
proof excludes an exceptional edge from this check. A shared body therefore
has one owner and no execution copy, and the other result arm cannot jump into
the natural shared arm. Rejection retains the previous owners and the explicit
Graph schedule; P6 MUST NOT retry another boundary or build a Graph first.
An original abrupt result arm needs no common concrete global postdominator.
Condition predicates are syntax-owned by their region. Predicate nodes, links,
copy placements and source origins are delivered unchanged.

After establishing region ownership and transfer plans, P6 finalizes an explicit
alternative for every accepted Condition in the same construction chain.
`ConditionSchedule` references the selected test-run children in order, then the
optional other arm, the optional routing child and the shared arm. It retains
every selected-path link as an explicit goto in that alternative. Each child retains its entry, internal
placement, operations, terminal phases, copies and abrupt-exit scopes. P6 supplies
one `ConditionCompletion` for every child. A source `Parent` completion denotes
the source parent's follow, not the next printed sibling: a nonfinal child gets
a planned goto to that established follow. A `Next` completion retains implicit
delivery only when the selected next child's entry is its original target;
otherwise it gets a planned goto. Existing `Goto` and `None` completions retain
their meanings. The final child's `Parent` completion uses the condition's own
retained continuation.

P6 constructs child membership from the selected region products, without
flattening nested syntax. The alternative names both original test links and
original `Next` links crossing a child boundary as explicit gotos unless the
child's established follow and selected implicit normal completion already
deliver that destination. Internal links retain their established delivery. Break and Continue retain their target
scopes because the alternative changes neither child containment nor internal
placement; normal completion after an internal scope exit passes through the
child's selected completion. Return, throw, exceptional delivery and lock outlets
retain their original ownership and phases. Planned continuations execute no
second edge copy and invent no cleanup. These rules cover implicit and empty-arm
completion as well as explicit links. Neither the schedule nor its selection
changes source child roles or continuation evidence. Existing closed-interface,
whole-owner and protection checks reject unrepresentable compositions locally
before commitment. Resource and construction-invariant failures retain their
classified errors, not a missing-plan success.

P7 MUST consume the delivered plan: it composes each test's scheduled prelude
only at its original evaluation position and prints one short-circuit condition,
or keeps the established explicit if/goto schedule from the same region. When a
prelude contains a source-ordered producer chain, P7 may compose those producers
into the predicate's call arguments or receiver only when every intervening
producer also stays in that operand order. A display-only check-cast remains
owned by the existing cast carrier and does not count as a separate operand.
When a prelude cannot keep operand evaluation order, copy facts or the
established branch role, P7 MUST choose that complete explicit schedule for the
conditions that depend on the missing prerequisite instead of publishing an
optimistic compound layout. The explicit schedule restores each test's terminal
at its delivered predicate portion, after all preceding portions, and keeps every
original branch edge and label. P7 MUST NOT infer shared reachability, split
owners, re-derive the tests or discover the explicit alternative's order by
walking descendant controls. P7 lowers the selected order into `EmissionPlan`;
the retained control IDs, children and Condition roles remain unchanged.

## Integer decision alternatives

After the selected hierarchy and transfers are complete, P6 can attach a
`DecisionPlan` to an existing If region. It describes Kotlin `when` over a
chain of equality/inequality tests of the same available integral
local/value/state. Tests have distinct integer keys, empty operation headers,
no terminal exception or copy phase, and a common original follow. Every later
header has only the preceding failed-test entrance and no executable exceptional
arrival. Intermediate failed-test links have no copy bundles.

The plan references each original test region, header, predicate, operand
position, selected/remaining link and body, plus the final default body. It
neither changes the source region tree nor constructs another execution graph.
Arms retain their abrupt exits and selected-edge copies; a nested normal
completion can be omitted only when it already completes the same source follow.
Rejection keeps the existing If layout. Resource errors retain RegionError.

P7 lowers this complete alternative without comparing selector text or
discovering cases. P9 prints its cases and default; both true and false original
link identities, test terminals, operands, controls and labels remain mapped.
The rule is not evidence that a method is a coroutine or permission to remove
state stores, suspended-marker returns or resume-error paths.

## Loops, switches and continuations

`LoopForm` is `While(LoopCondition)`, `DoWhile(LoopCondition)` or `Infinite`.
`LoopCondition` identifies the original predicate node, repeating link and exiting
link. Its polarity is preserved by those original edge roles. The body and the
syntax-owned predicate have separate execution ownership.

`LoopPlan::evaluation()` records the ordered original predicate evaluation nodes.
For a head test, P6 follows only the natural header's private `WithinBlock` prefix
to the same original block's conditional terminal. Every interior portion has
exactly its original preceding normal link and no executable exceptional arrival.
The prefix stays in the backedge core and contains no monitor operation. P6 does
not cross an original branch, rotate a tail test or hoist initialization. Original
operations, terminals, protection sites and P5 positions remain unchanged; candidate
partitioning still rejects split construction groups, scopes and existing owners.

A head test whose private prefix already contains every loop member leaves no
body before the test, so those operations run before the predicate on every
arrival, including the first one. The same predicate node is then the tail test:
P6 records `DoWhile` for that loop and keeps only the condition node as its
evaluation, instead of accepting a `While` whose body would be empty. This is
classification from the existing prefix, not rotation: the predicate node, its
original links and its evaluation operations keep their identities, and only the
form changes. Both shapes evaluate those operations once per iteration, so the
recovered behavior is identical.

The head evaluation and repeated body are distinct children. A head-loop continue
returns to the original evaluation entry, including its operations, not merely to
the terminal portion. If the head candidate fails, P6 retains the existing tail
candidate or Infinite form. A genuine tail test keeps its original predicate and
continue target at the tail. Traditional for, enhanced for and for-each are not
region forms. An evaluation prelude that cannot compose into a compact expression
keeps the existing explicit predicate-phase notation.

Private parts distinguish `Ordinary` completion from `LoopBody(LoopId)`.
Only an accepted Loop candidate supplies `LoopBody` to its body part before
composition; its predicate and all other parts remain Ordinary. P6 MUST keep the
body follow equal to that LoopPlan's continue target. During body composition,
original links to that target complete the lexical body execution and are omitted only
from the private sibling-successor relation, even when the target is inside the
body. Original links, edge/link plans and copies remain unchanged; subsequent
transfer planning retains Continue/LoopCondition selection. A link leaving a selected
loop's lexical content cannot become Next merely because an outer Sequence or
branch follows its target. It retains an explicit goto unless an enclosing
break/continue scope establishes that original transfer.

P6 MUST NOT project another cycle-closing link, use an unselected loop as context,
or infer loop completion from `follow == entry` in an Ordinary part. Exported
entries, one successor per child, no repeated remaining child, complete root
coverage, whole-owner membership and Graph selection retain their existing checks.
A selected loop child contributes every exit outside its lexical membership to
its parent's sibling relation. Proved private corridor content inside that membership
is not an independent sibling successor.

### Private handler rejoins

A normal analysis root at a catch can make an interior rejoin appear external to
a loop, including a nested loop. For each original normal-DFS retreating header
not already proved by normal dominance, P6 closes original latch predecessors
backward while leaving that header boundary open. It rejects a pulled-in method
entry, an added non-handler analysis root, and a handler root with a normal
entrance. Every interior normal or executable exceptional arrival must originate
inside the candidate, and every member must be reachable from the header using
original normal or exceptional transfers. Dead, disconnected and foreign catch
corridors are not private. These are owning-stage membership guarantees, not an
independent output verifier or an exceptional-edge dominance graph.

Both header nominations feed one LoopPlan construction before composition. The
accepted closed core keeps original node/link identities and its internal normal
arrivals at the header as latches. Header predicates, exit conditions, exact
parts, protection, lock and construction constraints still decide region
composition. Nested retreating headers are considered individually; a maximal
SCC cannot substitute for its contained loops. A shared handler receiving an
exception before the loop remains explicit when that arrival bypasses the
header. Ordinary normal-dominance loops retain their existing construction.

### Loop forms and original continuations

P6 supplies only head-tested, tail-tested and unbounded loops, displayed as
`while`, `do-while` and `while (true)`. The
[approved loop contract](../PIPELINE_DESIGN.md#563-loop-expression-while-and-do-while)
excludes traditional/enhanced for reconstruction. No initializer/update product,
extra update child or header-owned assignment is constructed.

Original updates remain ordinary body operations, with their original reads,
effects, copies and protection. A head-tested continue targets the established
head; a do-while continue targets its actual tail predicate. A transfer into a
shared body update is not such a continue: it keeps its original destination and
can remain an explicit goto. Break, return and throw skip exactly the original
work; no update is duplicated to manufacture a natural spelling.

Release outlets retain their original Follow/Loop roles and targets. Selection
checks that an original loop exit has its actual selected enclosing loop; it
does not turn a Follow(update) into a new loop Continue. Original release/link,
routing and before/after copies remain unchanged. P7/P9 consume those products
without identifying an updater or hiding a body operation in a header.

A conditional loop's normal continuation is the actual destination of its exiting
predicate link. It is not a common postdominator beyond exit-side work. Other loop
exits retain their own targets. A non-terminating region does not acquire a
synthetic exit to make postdomination convenient. `LoopPlan::members()` retains
the established backedge core, including proved private handler rejoin corridors
when needed. Latches and exits retain original links. `lexical_members()` separately records
the proposed lexical content, including closed synchronized scopes, their
release/break corridors and private established try handlers. Before child
selection, P6 closes lexical membership over complete established try bodies
using the same monotone membership rule as ordinary branch arms. A try touching
the loop header, predicate evaluation or normal follow remains outside this
closure. This later lexical closure changes neither established core membership
nor follow; ordinary
entry, partition and whole-owner checks still decide region admission.
`private_exits()` identifies original exits with admitted
private corridors. The predicate's actual exiting link and destination remain
the loop follow; lexical ownership does not skip exit-side work.

Before candidate enumeration, P6 can add a private corridor from another
original loop exit. Admission requires a finite single-normal-successor path ending
at its original return/throw, at the predicate's actual normal follow, or at an
enclosing loop's original follow or continue target. Strict containment of the
whole backedge core establishes an enclosing loop; header proximity or overlap
alone does not. Follow and continue destinations remain outside the corridor.
The corridor has that exit as its only normal entrance and no executable exceptional
entrance. It can lie inside an enclosing core, but cannot reenter its own core,
intersect any non-enclosing loop core or synchronized plan, or contain a monitor.
An established try permits the corridor only in its body, with the entire loop
core in that same body and its selected handlers proved normally closed and
abrupt by the protection producer. Handler content, cleanup alternatives and
partial/recovering try intersections do not qualify. A local catch target also
requires that established enclosing owner; absent ownership keeps the exit
explicit. Construction groups stay whole in the enlarged
loop; candidate partitioning also keeps them whole in each child. Shared corridor work, cycles, branching corridors and unknown completion retain
explicit control. The selected body interface retains private operations and the
original outgoing copies before its selected break; predicate-exhaustion copies
remain on their distinct original edge. A candidate loop supplies no break scope
until that Loop region is selected. Stopping at an enclosing boundary does not
supply an exit scope: transfer planning resolves break/continue only through the
selected ancestors and otherwise retains the original explicit transfer.
P6 does not compare returned values, merge terminals or manufacture an edge for a
throw. The enlarged candidate still passes normal/exceptional entry closure,
whole-current-owner, scope and partition checks before commitment. Rejection affects
only that composition; resource or construction-invariant failure keeps its existing
classified RegionStop behavior.

Original switch case keys, default, roles and duplicate-target edges remain distinct.
An original selector without a selected Switch interface keeps explicit original
gotos. A Sequence sibling is not a case-label successor, so that adjacency cannot
supply Next, break or continue to a raw selector arm. Selected switches retain
their existing arm/group and completion products.
A switch with no concrete postdominator can use one unique first shared actual
normal target as its external continuation. Reach counts start at distinct case
targets: duplicate selector edges do not manufacture an external follow.
They identify sharing of original targets, not equality of returned values. Multiple first shared targets
remain unresolved and explicit.

When that first shared target is not a selector entry, P6 gives its whole normal
suffix one outer owner, stopping at the enclosing completion boundary. The suffix
cannot reenter the selector. The primary switch body stops at every node in this
supplied suffix, not only its first node: a case that bypasses an earlier suffix
operation retains an outward link to its actual downstream entry. This suffix
precedes the ordinary postdominator nomination; a selector-entry join retains the
existing postdominator/shared-target choice. One primary proposal enters owner
admission; rejection does not search another boundary or reconstruct a tail.

Before child construction, P6 can group original selector links by their exact
entry and give each group one private child instead of one multi-entry body.
`RegionKind::Switch::cases` records those groups in selected lexical order;
`None` retains the explicit shared-body layout. Original keys, default roles,
link identities and selector order remain in the original arm list.

Normal membership stops at other distinct case entries and the supplied body
interface, including external shared suffix entries. Bodies are disjoint, cover
the original body exactly, have no executable exceptional entrance, and retain
whole construction groups/scopes. Whole established try bodies close over their
private handlers using the ordinary membership rule. A private case body can
contain its original loops or explicit cyclic graphs; case partitioning does not
classify those cycles, construct another loop, or supply a synthetic exit. Normal entrances come only from selection or another case body
at that body's entry. A body can name at most one other distinct case-entry
target, but that entry may have several original predecessors. P6 orders this
case-entry DAG once: ready roots follow original selector order, predecessor
counts retain all incoming relationships, and a next entry is visited adjacent
only after its last predecessor has been placed. Other predecessors retain their
explicit original gotos to that entry's one printed label. Only actual adjacency
can become implicit Next. Case-entry cycles, conflicting outgoing targets and
unresolved sharing retain explicit layout; no owner or boundary is retried.

A shared-label header or an entry reached from another case, adjacent or by an
explicit goto, requires complete zero-step selector copies already established
by P5. Otherwise its entry-only
assignment would also run on fallthrough, so P6 declines the layout instead of
moving or hiding that copy. Other selected copies remain on their original edge.
A direct target of the external follow has an empty final child, retaining its
selected copies and switch completion. A selector targeting a different external
suffix entry or enclosing outlet has an empty child whose follow is that actual
target, not the switch's other follow. Its selected assignment executes before
that completion. Other cases reach the external original label without entering
this selector-only assignment. Only internal case entries participate in the
case-entry DAG/copy-free checks. Shared suffix work has one external owner.

Adjacent case completion can become Next only for the exact supplied next entry.
Normal completion to the external follow becomes Break only against the selected
Switch; original Return/Throw and enclosing exits retain their identities. A
rejected composition keeps the original single body and explicit graph. No tail
is duplicated, selector repeated, storage invented or dispatcher generated.

`Continuation` records whether a completed region finishes its source parent,
reaches a confirmed next position, or needs an explicit goto/break/continue.
Only selected ancestors resolve the latter scope completions. A region with no
normal completion toward its follow receives no fabricated onward continuation;
an original break/continue that already leaves that region does not execute a
second planned jump. Completion emerging from a contained scope still proceeds
through the enclosing child's completion, including the explicit Condition
alternative. P7 MUST consume this
choice rather than infer fallthrough from source layout. Public follow content has
one owner, not a separate copy in each arm.

## Local continuation suspension protocols

After decision layout, P6 can publish a `SuspensionPlan` over an existing
`invokeSuspend(Object): Object` member whose original superclass is
`SuspendLambda`. This identity is only a candidate filter. P6 MUST also establish
the label-field read, matching state store and continuation operand, original
call result commit, marker comparison and private marker-return arm. The other
original edge and the matching resume arm each copy their established result into
the same existing local at the same original follow.

The supported interface has initial and resume integer cases, a resume body
containing its original `throwOnFailure` call, and one original assignment on each
result edge. P6 MUST require exact storage states and occurrences, no other
consumer of the call-result local, no intervening call-to-predicate operation,
no local exception handler or copy/snapshot phase on the call or resume check,
and no foreign normal or executable exceptional entrance into the hidden protocol
portions. A target's complete original method reference must end in Continuation
and return Object. That signature and the observed protocol infer suspension at
this call site; they do not establish an external callee implementation.

The owning lock-flow pass retains private node-entry emptiness from its existing
fixed point. An explicit no-monitor result is distinct from an unestablished
result. P6 MUST require established method-local lock emptiness at the call,
predicate, marker return and resume ingress. It MUST NOT rerun lock propagation.
Unknown or held locks leave the original protocol explicit.

The plan names the original dispatch, call, state write, result local, continuation
argument, marker, predicate, return, normal/resume links and copy plans. P6 keeps
the dispatch, state stores, resume check, default/error arm and shared continuation
as original code. It adds no state, dispatcher, SSA definition, tail copy or
runtime helper. A mismatch leaves that local protocol explicit; resource and
construction failures retain their classified behavior.

### Closed source-body alternative

A local plan alone does not authorize a source-body replacement. P6 additionally
publishes `CoroutineBodyPlan` when the selected root Sequence consists of an ABI
entry prefix, that dispatch and the original common-tail controls. The alternative
contains the original initial body followed by those same tail controls; it adds
no execution copy. The initial failure check remains an ordinary instruction.

P6 MUST account for the prefix's marker/label reads and literal definitions, state
publication, original resume body and closed invalid-state exception construction.
Their exact operations, effects and exceptions remain protocol attachments; P6
MUST NOT declare these calls pure or infer their implementations. An original
instruction-use issue on a protocol operation rejects the source-body alternative. P6 MUST establish
that their private locals do not escape into readable code. The initial body's
only normal entrance is its selected dispatch edge, and its only normal exit is
the original suspension-result edge. Executable exceptional arrivals into hidden
protocol content reject this alternative. Lock emptiness comes from the existing
owning computation, not a second propagation.

The alternative retains the initial ABI argument and every containing member.
The prefix and resume/error bodies remain readable ABI-only attachments, with
their original controls, instructions, references, sites and copies. The call
still refers to its precise continuation operand and original callee binding.
A missing prerequisite leaves the complete original layout and local facts
available. Resource and construction errors retain the ordinary classified stop.

### Ordinary-method suspend loop alternative

P6 publishes `CoroutineLoopPlan` for an ordinary Object-returning method whose
last declared parameter is Continuation and whose original dispatch has state
zero, one positive resume state, and a closed invalid-state exception arm.
The existing SCC product nominates the first cyclic node reached by the initial
corridor. P6 MUST establish a head predicate, one closed normal-run cycle, its
straight exit, and one private resume corridor entering the original result
merge. Other branches, multiple suspension sites, executable handlers, unknown
locks, unavailable operands, and additional entrances retain the original layout.

P6 MUST match the original call binding, continuation storage, state publication,
reference-identity marker comparison, and private marker return. Private direct
calls are eligible under the same Object/Continuation binding contract as other
ordinary invocations. Prefix allocation/cast/constructor references must identify
the continuation owner. The marker, entry result, and continuation storage cannot
be reassigned to another value. Prefix and invalid-state effects remain original
ABI attachments; they are not declared pure.

The owning correspondence requires each resumed merge copy to carry the same
available value/state as the normal route, or an unchanged local saved in the
same original field. Move normalization follows existing P5 definition/use facts;
it reconstructs no register state or SSA. Every normal merge assignment needs
its resumed counterpart. Changed captures, foreign fields, resumed-only effects,
copy gaps, and unmatched result delivery reject this alternative locally.

The plan partitions every original node into initial work, repeated work, exit
work, the head predicate, or ABI attachments. It delivers original normal links,
protocol operations, spill pairs, and ABI targets within the source body.
P7 consumes this order without planning another loop; P9 renders it without
analysis. The existing node/link/control arenas remain source evidence. Resource
and construction failures retain the owning P6 stop and its partial facts.
Normal requests run no independent output verifier or repeated SCC/dominance pass.

## Transfers, labels and P5 obligations

### Multi-site continuation body

`CoroutineFlowPlan` consumes the same P5 accesses, original P6 graph, shared SCC
facts and selected region hierarchy. Its site records associate each positive
dispatch state with one original call/marker pair, private resume corridor and
the original normal/resume copy merge. The owning P6 construction MUST establish
that saved fields carry the same available values as normal completion and that
private protocol values do not escape. Narrow field conversions and missing copy
facts keep their dependents explicit.

The plan selects existing source controls and prunes only established protocol
guards. It does not invoke composition, rebuild SSA or run dominance again.
Source control order, original targets and transfer copies remain delivered
facts. P7 lowers protocol terminal comments and the normal selected transfer at
the original guard position; P9 consumes that schedule without analysis.
Unmatched fields/effects, executable handlers or incomplete control ownership
retain the original readable layout. A cyclic source body uses the closed
alternative below; cycle shapes it cannot establish also retain the original
layout. These alternatives are not whole-method output qualification or
independent runtime verification.

The complete alternative also supplies selected guard-control replacements,
source completions after retiring the ABI dispatch scope, and explicit ABI
link spellings. P6 MUST match a guard's original normal child and preserve all
normal copies. A source completion naming a retired scope needs its established
source follow; it cannot become an unqualified break outside the old switch.
P7 consumes these products together. It MUST NOT keep both the original marker
If and the projected guard, or borrow case-body completion from the ABI layout.

#### Closed cyclic source body

When selected source content contains a cycle, the same `CoroutineFlowPlan`
carries a `CoroutineCyclePlan` only if the shared SCC facts and selected region
hierarchy establish one closed head-tested loop. P6 MUST require one cyclic
component among the selected source nodes, a unique initial corridor from the
original case-zero entry to that component, and a header that is the component's
only terminal branch with no operations. The corridor's controls each have one
normal successor, and the header has exactly one repeat edge and one straight
exit. Every retreating link from the component targets that header, and exactly
one source link enters it from the repeated body.

P6 MUST own the whole selected body: initial controls, repeated controls and
exit controls partition every selected node, with the header as the only shared
control. The latch is the unique source link from the last repeated control to
the header, that source node has one normal successor, and no other repeated
link targets the header. P6 delivers the original head `LoopCondition`, the
implicit latch, the initial-header link and the three control lists. The
repeat/exit edges, exit copies, interior resume corridors, spill barriers and
protocol operations remain original facts.

Nested cycles, do-while shapes, multiple latches, additional cyclic components,
header operations, branches on the initial corridor and foreign entries reject
this alternative locally. The plan then falls back to the original readable
layout with its ABI dispatch and resume facts. P7 and P9 consume the delivered
partition without selecting a latch, retargeting a transfer or moving an
interior resume corridor.

Every original edge has an `EdgePlan`. Each normal `LinkPlan` chooses a concrete
`Goto`, `Next`, arm, loop predicate, `Break`, `Continue` or synchronized-boundary
action. Return and throw retain their original terminal identity; P6 does not
invent normal edges for them.

`LabelId` binds a node entrance, original `BlockId`, entry kind and leaf ownership.
Original block entries, handler entries and within-block continuation labels remain
distinct. An original jump still targets the original block entrance. Continuation
labels only describe the planned sequencing of portions of that same block.
A handler label does not turn an exception arrival into a normal goto.

P6 MUST select the original branch before executing that edge's P5 copies. Normal
link copies refer to the original `NormalEdge` site. `CopyPlacement` retains every
other P5 location, including initial entry, normal commit, before-operation,
before-terminal, before-throw and handler-only receiving positions. Assignment gaps
and unavailable copy components remain P5 facts; a plan ID is not evidence that
all of its assignments are available.

`CopyPlacement::owner` selects original emission, a P5 construction group or a
synchronized outlet. Outlet-owned plans retain their P5 locations and appear in
that outlet's before/after-release references, not also in ordinary link emission.
P7 MUST consume that ownership and phase order without reordering assignments.
`SynchronizedReleased` identifies the particular original release and its link;
the outlet separately records any syntax-owned routing to the final continuation.

`OperationOwner` and `TerminalOwner` define one emission partition. A natural
predicate, catch capture, synchronized operation or P5 construction member MUST
NOT also execute through an ordinary block lowering. P6 retains all original
facts even when natural syntax represents an event. Selected P5 construction
groups stay atomic across implicit control/protection scopes. Their deferred
publication, operation/copy emission and snapshot prelude remain P5 decisions.

## Protection and catch ownership

Every original throwing operation and throw terminal has a `ThrowSiteId`, its
original completion, ordered handler chain, original edges and before-throw snapshot
references. Throw and selection PCs remain the P2 facts. Handler entries retain the
original exception capture and P5 receives after that capture. Missing catch type
content is not a reason to remove its original identity or priority.

A natural `TryPlan` records a clause range in an original chain and disjoint body
and handler content. `handler_clauses()` associates each unique handler body with
its contiguous original clause range. Adjacent typed clauses with the same target
can share one body; every original type, priority, clause and edge remains in the
chain. Nonadjacent aliases, aliases across the selected range boundary, and a
mixed typed/catch-all group remain explicit. No hierarchy or assignability fact
is inferred. Normal or sibling-catch entrances into a natural catch body reject
that wrapping.

Before enumerating protection candidates, P6 computes each original handler's
normal and executable exceptional reach. A node with a normal or executable
exceptional arrival from outside that reach is a shared frontier, not private
catch content. This includes an outer cleanup handler that also receives
exceptions from the normal protected body.
The capture entry itself is never cut this way. Including nested exceptional
paths keeps their private joins in the enclosing handler.

Handler membership stops at these frontiers as well as its selected normal
follow. The original outward links retain their targets, edge copies and
explicit transfers; the shared tail has one owner outside the catch. The
protected body's normal follow is not replaced by the handler's separate
continuation. Catch admission still rejects foreign entrances into the remaining
private content, and construction groups cannot be split. This can recover a
natural catch with an explicit outward goto without duplicating its cleanup or
claiming that it is a finally. Frontier walks use fallible scratch and iterative
queues in the same owning protection pass.

Proved enclosing synchronization supplies a bounded protection context containing
the actual held graph and outlet anchors. Cleanup handlers remain ordinary
protected content: P6 does not remove their clauses, captures, operations or P5
receives in favor of an implicit cleanup template. The unlocked cleanup alternative
below retains these facts separately. Ordered prefix and suffix clauses remain unchanged. Candidate composition prefers the full catch frontier
when equal memberships would otherwise let a suffix obscure it.

Protection patterns retain original ordered intervals. In addition to whole-chain
suffixes, an observed proper suffix chain establishes an inherited outer layer;
its longest match supplies the immediate outer boundary, allowing the preceding
interval to become an inner try. P6 does not search arbitrary partitions. Matching
at a throw site requires one unambiguous identical interval. Selected deliveries
must preserve clause order through equal or successively enclosing try scopes;
a mixed explicit/natural route or reversed lexical priority withdraws that
composition without changing original edges.

A protected body's base retains every guarded seed and its original forward
content. Optional backward prefixes require all normal successors already inside
and no executable throw. One fallible predecessor queue grows that closure once,
stopping at the follow, excluded handlers and throwing portions. During that
construction P6 tracks original normal and executable-exceptional arrivals from
outside, plus method entry, and records complete single-entry checkpoints. It
selects the latest such checkpoint before handler/guard/owner admission: a later
optional predecessor must not destroy an already established guarded entry.
Temporary multi-entry growth is allowed when a later common prefix closes it.
Choosing an earlier checkpoint removes only the optional predecessor suffix;
every guarded seed and forward path remains. No branch path is clipped, no
normal dominance is recomputed, and a later rejection does not retry another
prefix or owner. If no single-entry checkpoint exists, this local scope remains
explicit. Ordinary guarded-site, handler, construction, scope and combined-entry
checks still apply.

Original later clauses remain outer boundaries for both body and handler walks.
A loop backedge can make the outer cleanup reachable from a catch; this reach
alone does not authorize absorbing the outer capture into that catch. Earlier
clauses can remain internal protected content for an established outer layer.

Follow candidates start after guarded nodes. A successor that is itself one of
the guarded seeds is internal protected content, not a follow constraint. P6
omits those successors when deriving the common normal follow, while retaining
every seed and original edge for membership and entrance checks. Within a synchronized context,
handlers that complete through abrupt outlets do not constrain a shared normal
follow; guarded nodes' own normal successors still constrain it. When no follow
is common to the guarded successors and handlers, their normal-successor follow
can be used as a candidate boundary. The same body/handler membership, protected
site and entrance checks must still accept the entire candidate; no handler or
abrupt exit is removed by this choice. Exception-closed
body/handler membership and entrance checks retain nested cleanup protection and
secondary exceptions. A proved `ImpossibleRelease` supplies no executable
exceptional entrance, but its original completion, chain, edges and snapshots
remain in the product.

A site does not execute both an explicit and a natural capture plan. If selected
candidates cannot account for its original handler choices in one model, P6 expands
the affected natural protection nodes into explicit graphs before publication.
This changes plans, not original execution facts, and preserves independent regions.
P7 receives either natural ownership or the complete site-specific chain; it does
not infer a handler relation from printing position. Catch-all is not automatically
finally.

### Completed abrupt catch scopes over loops

Before try admission, an unbounded single catch-all can use an established
LoopPlan's actual predicate follow as its primary normal boundary when the
ordinary common follow is absent or inside that core. P6 constructs the header's
original normal/executable-exceptional footprint up to that follow, excluding
selected/outer captures. Every core node and guarded seed must be present, every
throwing site must retain the exact selected interval or dispatch entirely
inside the body, and original normal/executable-exceptional arrivals must close
at the header. Private abrupt branches can lie outside the backedge core. An
unguarded call, even another cleanup call, or a foreign arrival refuses this
nomination.

The existing normal dominance establishes each original handler's closed-abrupt
fact once: dominated content has no normal outlet beyond that subtree, and its
normal leaves are original return/throw terminals. A shared recovery join or an
unestablished capture root does not qualify. The largest eligible core supplies
one footprint, header and follow before ordinary handler/construction/owner
admission. That footprint is reused, not reconstructed; optional backward prefixes
do not acquire initialization before its already proved header entrance. A later
admission rejection does not search another loop or boundary. Bounded monitor
contexts do not widen.

Ordinary handler construction also checks the complete private arms, including
nested executable exceptional paths: normal outlets must remain inside each
arm, and normal leaves must be original return/throw terminals. A recovery via
such a nested path invalidates a completed-loop nomination without retrying a
boundary. The TryPlan retains this private closed-abrupt handler fact for lexical-loop
corridor construction. Only a whole enclosing body can license a protected
private exit; consumers do not rediscover abrupt handlers or infer an owner from
printed nesting. Original core, links, copies and priority stay unchanged.

This preserves one catch owner while exposing the real normal loop completion
instead of an intermediate iteration join. It does not by itself claim finally:
that requires the owning-stage cleanup correspondence below, not a renderer
recognizing cleanup syntax.

### Matched unlocked cleanup

The cleanup cursor first supplies its original first event after capture. P6 can
complete the normal body's whole pure portions from its established sole outlet
until one complete single-frontier entry names that first cleanup event. The
frontier requires matching original operands and initialization, not only the
opcode kind. Pure body preparation with different reads remains before cleanup.
Every new portion has only body predecessors, no executable exceptional entrance, and
available ordinary scalar reads; it crosses no method entry, construction group
or held monitor scope. Temporary multiple frontiers can close at a later common
join. Checked counters and one fallible queue construct the closure; no graph or
dominance relation is recomputed. The first admitted event is the only candidate:
a later correspondence failure does not search another start. Literal argument
preparation belonging to cleanup stays at that start, rather than being swallowed
as body work. Only a successful complete cleanup match commits the supplied
preparation as protected body; otherwise the original TryPlan remains available.

Before composition, P6 can attach a `FinallyPlan` to a `TryPlan` whose selected
original clause interval is one catch-all, even when its source chain has an
inner typed prefix. Exceptional body transfers may stay in nested body handlers
or enter this exact cleanup, but may not escape to another handler or uncaught
completion. The full original chain is not substituted for the selected interval.
Its handler is a finite acyclic cleanup graph whose leaves are explicit cleanup
returns/throws or pending rethrows of the exact capture storage state with a
reference view. An always-abrupt cleanup needs no pending rethrow. Every hidden
pending rethrow must have an established unhandled original route.
A site-specific outside handler cannot be erased by assuming implicit lexical
propagation; until natural enclosing delivery is established, this alternative
stays explicit. The captured local has no other read or copy dependency. Moves, numeric/string
constants, unary/binary arithmetic, comparisons, field accesses and
static/virtual/interface calls can match. Allocation and direct construction
calls match only as members of already selected P5 natural construction groups.
Each group requires complete member/role correspondence to one equally complete
peer group; preludes and snapshot members keep construction explicit.

A paired cursor traverses handler and normal cleanup events. Block cuts and
copy-free gotos are transparent. Conditions require the same operator and operand
correspondence, with original true/false edges paired by role. Bidirectional
event maps require consistent joins; an active pair revisit rejects a cycle.
Completed pairs are reused, not expanded into every possible path. Explicit
cleanup return pairs require identical widths and corresponding P5 reads; throw
pairs require corresponding reads and established unhandled routes on both sides.
Pending rethrows instead end at the same actual normal continuation, at a
whole-node boundary. All-abrupt cleanup has no fabricated continuation.
Final empty routing is included in the normal duplicate before choosing
that continuation, so one arm's goto to a shared end does not become a different
continuation. No unpaired instruction is swallowed to align block cuts. Scratch allocation is fallible and cursor arithmetic is checked.

An operation pair requires identical original kind/target, effects and normal
commit rules, plus equal exceptional frame/heap/lock behavior and uncaught routes.
Inputs are exact P5 reads, equal literal bits/views, or results of already matched
definitions. Two IntegralView declarations may have different method-local name
identities: their published range proof makes both value preserving, not runtime
conversions. They correspond only when the underlying state or previously paired
definition already corresponds; equal literal bits are still required. Unknown,
reference and incompatible machine interpretations are not equated by this rule. Result pairs require compatible storage types and complete reads
confined to their respective cleanup graphs, with no copy dependency or escaping
result. Original Phi incoming dependencies also prevent folding a private result,
including a literal-folded incoming with no executable copy assignment. This correspondence does not coalesce storage or make a Phi input equal
to its result. Initialization facts must match for each use.

Both paths require established empty local lock entries and no P5 copy or snapshot
execution inside the folded cleanup. Original selected-edge copies on body outlets
remain at those outlets before finally; touching their target is not execution
inside cleanup. Interior and cleanup-exit copies still decline folding. Cleanup
nodes have no foreign normal or executable exceptional entrance. Body returns,
bypass outlets, escaping protection, cross-handler shared tails and unmatched value
joins reject this alternative.

The plan pairs original normal and handler operations, predicates and explicit
cleanup terminals, retains the original capture and all pending rethrows, and
adds the normal duplicate as a separate source child.
The protected body owns its original nodes plus the established pure normal
preparation; normal completion enters the matched cleanup, then continues at its
original next node. A body outlet targeting the normal duplicate is implicit only
when lexical completion reaches it without bypassing pending work or a loop.
Otherwise P6 supplies a Break of this selected Try, preserving the original target
and copies while executing finally once. Derived child completions obey that same
rule; they do not print a goto to the hidden duplicate's label. Catch-all
completion executes that same cleanup and implicitly rethrows the established
capture unless cleanup itself returns or throws. An abrupt cleanup exit replaces
the pending body exit; a normally completing cleanup preserves it.
No store, binding or copy is manufactured
or moved. Ordinary region admission still selects the whole interface; an
unselected or refused alternative leaves its instructions explicit. Allocation
and necessary construction failures retain the ordinary classified P6 stop.

## Catch binding surface

After region ownership is selected, P6 publishes optional header captures for
selected Try handlers. A capture requires one original ExceptionCapture definition,
only exact-state reads within that handler body, and no Phi, copy or capture-phase
obligation. These facts permit P7 to bind the same exception at the catch header;
unavailable dependencies and escaping/shared storage retain the original assignment.
The original type clauses, handler priority and receiving copies remain unchanged.
P6 consumes published P5 storage and selected regions without another analysis chain.

## Monitor capture and scope proof

Lock analysis describes method-local acquisition obligations under P2's completion
contract, not a claim about all locks held by the calling thread. An untracked
acquire or an uncertain join remains a concrete `LockGap`, not proof of malformed
monitor use. Normal acquisition, normal release and exceptional release outcomes
remain distinct.

A persistent, method-private stack and work queue propagate those obligations.
Repeating one acquisition site before its release becomes uncertainty rather than
an unbounded stack. Captured-reference stability consumes P4 identities and P5
storage reads/writes. Before/after positions distinguish a change before the exit
operand read from one after release. A shared static root or an allocation-source
set alone does not establish the same dynamic object: relevant writes or repeated
births in the capture window defeat that proof. This is not another register or
alias environment.

Under the P2 lock-completion model, a successful acquire supplies a non-null,
current-thread-owned capture. A matching stable release with established ownership
has a local `ImpossibleRelease` exception disposition. P6 retains its raw P2
completion and all original exception edges; the disposition is not a general
callee-behavior or VM-admission proof. An unresolved lock relationship retains
explicit monitor operations and the original exceptional outcome.

`MonitorPlan` records `Natural(SyncId)` or `Explicit(LockSeparation)` for each
acquisition. Scope construction follows all executable normal and exceptional
held paths. Each path stays held, diverges, or reaches its own stable matching
release. A return or uncaught exception while still held rejects naturalization.
A closed divergent body needs neither an invented release nor an invented exit.
Acquisition failure does not enter the held body. Nested acquisitions remain
distinct obligations even when they capture the same object.

`SyncPlan::held_nodes()` records original held execution. `body()` additionally
contains syntax-owned outlet anchors, not unlocked operations moved under the
lock. Each `SyncExit` records its original release/link/target, routing, P5 plans
before and after release, and continuation. Multiple releases are supported.
A scope has at most one shared normal follow. Distinct closed unlocked targets
instead leave that follow absent and retain an explicit original goto for each
outlet, or a selected enclosing-loop break/continue. No implicit continuation is
invented at the closing scope boundary. Routing remains syntax-owned original
content; target operations, including value returns and unlocked calls, stay
outside the held body. Operand-free returns and rethrows of existing exception
values retain their established outlet forms. Loop outlets require the selected enclosing loop, not merely
a loop analysis candidate. Context finalization may refine only a retained
Follow matching a supplied iteration target of an actual selected enclosing Loop.
It does not reinterpret an incompatible existing Loop intent, reopen a declined
scope or change early shared-follow/copy availability. A missing selected context
keeps Follow; an already classified Loop without its required ancestor retains
LoopContext separation. This is completion of source roles from existing products,
not another loop or lock analysis. A rethrow retains the exact original P5 read. Its
capture identifies provenance, not permission to substitute the current handler
exception or the capture local.

Branched cleanup operations, nested handlers, handler-only receives and secondary
exceptions remain at their original positions in the held graph. Only the matched
monitor event is represented by the synchronized release. Every original terminal
and operation retains one emission owner; cleanup effects are not silently
absorbed into an implicit finally. A failing cleanup that actually escapes before
release keeps the monitor scope explicit rather than acquiring an invented release.

Outlet classification does not authorize copy motion. One normal outlet can
execute its own post-release plans after the scope closes. Multiple normal outlets
or abrupt outlets require their post-release plans to be complete and have no
execution steps; P6 does not hoist path-specific assignments into the locked body
or generate a dispatcher. Distinct unlocked continuations that cannot be expressed
by implicit fallthrough retain their explicit original targets instead; they
need no dispatcher or invented shared follow. This does not relax closed-entry,
capture, construction or phase admission. Unsatisfied post-release plans retain
`PostReleaseCopy`; in particular, multiple normal outlets still cannot move an
executable selected-edge copy under the lock. Value-returning target code remains
at its original unlocked location. Scope/region rejection
retains the original plans and a concrete reason, not a method failure.

The synchronized entry and every release outlet retain their exact P5 obligations.
Successful body completion executes the selected release before its after-release
plans and original continuation. Synchronized syntax and ordinary monitor emission
do not both own the same event. An explicit goto does not add release, finally,
heap rollback or initialization. Independent proven nested regions remain available.

### Implicit monitor surface

P6 can publish an `ImplicitMonitor` on a selected SyncPlan after ownership and
transfer finalization. The surface consumes the existing closed held graph and
stable matched releases. It requires one original normal Follow outlet without
routing, no executable release-phase copies, and only private exact capture
rethrows on the other outlets. Every hidden cleanup must be the last catch-all
handler of a selected enclosing Try inside that monitor scope. Its body contains
only capture, the matched release, transparent routing and the exact rethrow.
Every executable entrance already requires natural delivery to that same Try.
A pending rethrow must propagate uncaught or into an existing enclosing natural
catch; an unplaced outside handler cannot become implicit propagation.

The surface identifies the Try and handler index, captures and rethrow terminals.
Earlier typed handlers remain executable. Extra cleanup, replaced exceptions,
different locks, multiple normal outlets, missing delivery and changing/copy
obligations retain the explicit surface. Independent inner scopes can stay
explicit while a qualified outer scope becomes implicit. No node, edge, capture,
copy or original monitor fact is deleted, and no independent lock/graph verifier
runs on ordinary requests.

## Correspondence and evidence

Regions retain original block/operation/terminal ownership, normal edge targets,
P5 copies, source-site completions, handler priority and snapshot positions.
Synchronized outlet phases retain original release and continuation locations.
These obligations are not established merely by constructing a region.

The previous core test was removed by operator decision. Ordinary requests do not
run independent symbolic replay or a verification switch. Corpus observations do
not prove arbitrary graphs, callers, callees, monitor patterns or resource paths.
