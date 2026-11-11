# A small recovery core for P6-P9

> Historical proposal against `f6e7893` for the retired numbered-stage pipeline.
> It does not define current architecture, implementation status or refactoring
> authority. See the [documentation entry](README.md) for active references.

## 1. Decision and size objective

Use one recovery grammar, two composition operations and local work queues:

- P6 combines control fragments by their exact entry and completion interfaces.
- P7 combines operations into expressions by exact uses and execution obligations.
- P9 traverses the selected structure without interpreting recovery history.

The primary objective is substantially less maintained production code with
stronger recovery of combinations of existing mechanisms. An elegant API without
net deletion is insufficient. The design makes code smaller by removing repeated
representations, repeated reconstruction of facts and whole-shape handlers.

~~~text
P5 facts: operations, exact uses, storage, copies, snapshots
                         |
P6: normalize and contract control fragments
                         |
     one control grammar with original-operation leaves
                         |
P7: lower leaves once, normalize and cover expressions
                         |
     one selected structure with owned expressions
                         |
P8 names -> P9 traversal, formatting and source spans
~~~

The core does not require a rule registry, optimizer framework, candidate e-graph
or a new solver. Direct Rust functions implement the two composition operations.
A shared helper is justified by actual duplicated mechanics it replaces.

The baseline P6/P7/P9 directories contain 36,635 physical Rust source lines,
including comments and blank lines. The completed replacement MUST be smaller
across its full production scope, including helpers moved elsewhere. Tests and
documentation are measured separately. Moving files, compressing formatting,
deleting tests or removing required semantics does not satisfy this objective.
No reduction percentage or parity claim is established by the proposal.

## 2. Minimal representation

### 2.1 One control grammar

Use arena IDs and named child roles for the following concrete structure:

~~~text
Control = Sequence | If | Loop | Switch | Scope | Try | Synchronized | Graph
Body    = ordered Evaluate | Copy | Control | Exit | Unavailable items
Leaf    = original operation portion in P6; owned evaluation in P7
Arm     = original edge + edge copies + body + completion
~~~

An If names its predicate and true/false arms. A Loop names its original test,
body, repeat/exit transfers and continue target. Loop forms are while, do-while, while-true, traditional for and enhanced-for. A Switch names its original cases, default and shared bodies.
Try and Synchronized retain their original handler/cleanup/lock correspondence.
Graph uses the same bodies, leaves and transfers; it is not another method route.

Each selected body has one static owner. Shared entries reference that body
through explicit transfers. Children with the same destination are not copied.
A selected-edge copy belongs to that edge even when its arm has no statements.
Lexical completion, ordinary transfer and exceptional completion stay distinct.

Condition chains use ordered predicate preparation and exact selected/other
edges. P7 chooses compact spelling only when that program's obligations fit it.
The explicit form uses the same program. P9 does not choose another schedule.

### 2.2 One operation representation

~~~text
Operation
  operator and original reference
  ordered operands: Read(use) | Literal | Nested(operation result) | Unavailable
  result: no result | direct value | existing storage commit | explicit outputs
  original copy/snapshot/exception/initialization obligations
  original operation, member and commit origins
~~~

Standalone statements and nested expressions use the same operation records.
Structural placement determines execution. There are no parallel composed-use,
composed-operation and hidden-statement decisions to reconcile at rendering time.
A source anchor alone never implies execution.

Value identity, execution storage and display name are separate. Reads retain
original ValueId/StateId and their interpretation. Snapshot reads retain their
saved state. Distinct values can reuse one storage; equal storage is not equal
value. Shared results execute once and retain the materialization their observers
need. Explicit multiple outputs remain valid when one expression cannot hold them.

Every copy, snapshot, receive and commit has one selected execution owner. A
reference in an interface or origin table does not execute it again. A phase-owned
copy cannot also be an independently executed body item.

### 2.3 Complete products and narrow construction authority

P6 and P7 instantiate this grammar with different complete leaf domains. P6
publishes a control plan with valid original-operation references, not empty
expression slots. P7 maps leaves and owned references once into its own product.
P7 preserves P6 roles and targets; it does not re-identify regions or handlers.

The common schema supplies role traversal and reference mapping once. Owning-stage
constructors remain sealed; sharing records does not expose a general mutable
arena or predecessor getter. P6 does not build Java expressions, P7 does not
reschedule P5 copies, and P9 does not perform analysis.

The owned P7 result has no DEX or PlannedMethod lifetime. Original facts remain
available for inspection and local uncertainty, separately from selected execution.
No-code members retain the declaration path. A source-only attachment is distinct
from an executable protocol event and is traversed in a separate origin mode.

## 3. P6: contract fragments instead of recognizing complete shapes

### 3.1 Working state

P6 keeps one original graph and its existing analysis facts. The mutable working
view contains active fragments, an original-node-to-current-owner map and boundary
references into the original incoming/outgoing edge indexes.

~~~text
Fragment
  selected body/root
  original entry positions
  normal entrances and outlets
  exceptional entrances and outlets
  abrupt completion and protection/lock/initialization obligations
~~~

A fragment's members are represented by ownership and child relationships.
The implementation SHOULD avoid copying every descendant's complete membership
list into every ancestor. Current-owner lookup can use a contraction forest;
index layout is chosen with the actual graph operations, without another library.

The existing CompletionInterface is the starting point. It becomes the interface
for composition, not a second summary beside competing per-family boundaries.
Original edge identities retain their copies, exception priority and saved states.

### 3.2 The composition operation

A structural rule nominates children and their roles. The common operation then:

1. Resolves the current owners of the nominated original content.
2. Collects the candidate boundary from child interfaces and original edge indexes.
3. Classifies matching internal transfers and retains all unmatched outlets.
4. Checks outside entrances, child overlap and the rule's semantic conditions.
5. Reserves the new record and ownership updates, then installs one compound.
6. Enqueues changed neighbors and containing candidates.

Outside entrances to an interior node remain visible after contraction. A
candidate cannot become private merely because its children hide those entries.
Unknown boundary coverage does not mean an empty boundary.

Sequence, If, Loop, Switch, Try and monitor rules share this operation. Their
specific facts differ: a loop needs its established header and continuation;
a try needs ordered protection; a monitor needs matched release ownership.
The rules do not each implement another membership walk and completion merger.

A candidate can nominate a compound before every part has a natural spelling.
Candidates respect enclosing boundary constraints before commitment. Disjoint
and nested candidates compose; crossing ownership retains shared content once
with explicit transfers. No recovery rule invents a join, clones a body or creates
a dispatcher to force a natural form.

### 3.3 Generalization example

A switch arm containing a loop with a typed catch and a shared tail should use:

~~~text
try rule: protection and handler facts
loop rule: iteration and continue facts
switch rule: keys, entries and fallthrough facts
common composition: ownership, boundary and completion
~~~

There is no additional rule for the complete combination. If this combination
fails, fix the missing common relation or the owning semantic rule. Do not add
another method-shape recognizer. The existing
[shared-tail fixture](../../quality/fixtures/branches/switch-shared-tails/README.md)
is a compatibility starting point; a new complexity challenge must demonstrate
any claimed extension.

### 3.4 Protocols retain facts, not separate control languages

Coroutine body, loop and multi-site forms use ordinary control records. Protocol
facts describe call/state/resume associations, spills, marker exits and source/ABI
correspondence. Adding a site adds facts and ordinary composition, rather than
another executable layout type and another renderer traversal.

Recognition remains P6-owned. Names alone do not establish a protocol. Original
state-publication events remain ordering barriers even if their established
surface is an ABI annotation. A source-only attachment cannot replace an event.

Cleanup correspondence similarly identifies original occurrences and their pending
completions. Normal cleanup resumes the applicable completion; abrupt cleanup
overrides it. One static cleanup node can represent multiple dynamic executions
only with that delivered correspondence. Shared nodes alone do not prove equality
of cleanup behavior or caller state.

## 4. P7: cover expressions through one producer/use operation

### 4.1 Three necessary queries

The shared P7 index answers only these categories:

| Query | Source | Purpose |
|---|---|---|
| What does this read mean? | Original P5 value/state, view and availability | Preserve exact binding through storage reuse. |
| Where does this operation execute? | P6 roles/targets plus P7 selected placement | Admit the actual cover without inventing another execution context. |
| Who still observes its result or phase? | Reads, copies, snapshots, initialization and protocol facts | Retain necessary stores and original completion behavior. |

One P7-private walk supplies ordered expression entry, completion and nested
ownership events. Carrier queries stop at another operation's owned evaluation;
selected-read collection follows that established edge. Statement and terminal
roots share the same operand order, and multiplicity uses the same child grammar.
A plain parameter or literal before the checked read does not create an effect
boundary; an acting expression, unavailable input or foreign carrier does.

These queries replace carrier-specific scans. They do not create another source
variable allocator, register interpreter, SSA solver or whole-method flow analyzer.
Exact identity does not prove that relocating a producer is safe.

### 4.2 Joint cover construction

For a consumer, enumerate its exact operands in evaluation order. Match the
preceding preparation from the end while constructing the operand forest. Several
producers can enter one candidate together; intermediate adjacent pairs need not
be independently composable.

A candidate includes every moved operation, its retained result commits, phase
obligations, replaced placements and origin changes. It is admissible only when
all intervening obligations belong to the forest or remain at their original
independent positions. External observers retain their existing storage.

~~~java
// With compatible views and no additional phase or storage observers:
a = left();
b = right();
return combine(a, b);
// Selected cover: return combine(left(), right());

// Reversed operand order requires a retained materialization:
a = left();
b = right();
return combine(b, a);
// Selected cover: a = left(); return combine(right(), a);
~~~

A source-ordered consumer pass builds each root's forest through an ordered
suffix match. A nested root already owns its preparation, so later consumers
reuse that structure without rescanning it as another statement. An unmatched
effect is a suffix boundary. Operand reversal retains the necessary earlier
materialization. The same matcher admits ordinary suffixes and requires complete
coverage of a skipped short-circuit prelude; refusal of that prelude discards its
uninstalled proposal list.

Pending ownership uses sparse indexes into the candidate list. An active root
suffix replaces scans through already owned preparation. Source-anchor selection
visits each covered portion once after ordering original operation identities;
this is construction of origin associations, not a new execution schedule.

The same cover constructor serves calls, return operands, conditions, stores and
cast carriers. Each operation supplies its own admission conditions, such as
check timing or receiver order. It does not supply a separate relocation engine.

Plan a complete cover before mutating execution. Compare conflicting candidates
in a stable original-source order; prefer admissible natural composition with
required sharing preserved. Joint planning avoids committing a locally attractive
choice before its dependent operations have been considered. Global optimality
is not required, and no persistent graph of all possible covers is built.

### 4.3 Execution obligations

Required readonly checks participate as operation roots even though they write
no register. Treating them as transparent preparation can invert their exception
order with another producer. Established redundant checks normalize before
coverage; required checks consume the exact operand value/state through the
same ordered cover. A source forest that cannot state all its operation order
retains the original preparation and explicit control. A forest with compatible
operand order still composes its checks and other producers together.

An operation retains the order:

~~~text
before copies -> operands -> before-throw snapshots -> effect
    exception: original route, no successful result commit
    normal: original result commit -> after-commit copies
~~~

A nested producer cannot cross the consumer's before copies, required captures,
protection changes or protocol barriers. Nonempty phases are obligations, not
an automatic ban on all composition. When the syntax cannot express them in
place, the existing explicit evaluation remains.

If f commits a value before g and g's handler reads it, the cover retains that
commit. If f throws, g's snapshots and effect do not run. If g throws, its normal
commit does not run. Equal ValueId alone cannot establish these properties.

A producer before a branch cannot be moved into one arm merely because its only
ordinary use is there. Cross-context covers need established execution-count,
iteration and completion facts. Missing facts remain a recovery gap; a smaller
implementation obtained by quietly losing this capability is not success.

## 5. A small local fixed-point driver

Use one concrete scheduling pattern in each owning stage:

~~~text
seed active nodes, interfaces and use sites
while work remains:
    take an affected item
    normalize its local representation
    prepare relevant region or expression composition
    if accepted:
        reserve and atomically commit the complete change
        update affected queries
        enqueue actual users, boundary neighbors and parents
    otherwise:
        retain the readable form and its relevant dependency state
~~~

Coalesce duplicate pending work. Reconsider a refusal only after a dependency
changes. P6 reaches its local fixed point and publishes before P7 starts; P7
cannot enqueue upstream analysis. Original graph and value facts remain immutable.

The initial rule set is intentionally shared-mechanism oriented:

| Rule | Required facts | Enables |
|---|---|---|
| Remove an obligation-free identity Move | Exact binding, compatible view, no lost observer | Producer/use coverage. |
| Flatten an unnecessary Sequence or scope wrapper | No lost entry, transfer target or completion role | Parent composition. |
| Normalize predicate role and polarity representation | Original test/edge relation and interpretation | Shared condition/loop handling. |
| Consume established literals or redundant checks | Existing owning-producer decision | Expression coverage and local cleanup. |
| Contract a control fragment | Exact interface plus structural admission | Larger region candidates. |
| Install a joint expression cover | Ordered operands and all execution obligations | Consumer coverage and compact test presentation. |

Normalization is not permission for arbitrary Boolean algebra, dead-path deletion
or opaque-call elimination. Unknown calls retain possible effects and exceptions.
No rule uses fixture names or a complete-method instruction template.

Each rule documents its input, admission, replacement, dependency delta, local
failure behavior, termination direction and removed old implementation. These are
code-level contracts, not runtime registry records.

Accepted changes consume original structural boundaries, independent producer
placements or redundant indirections/wrappers. A rule group must have a
well-founded measure that all its rules respect; rules cannot recreate each
other's input indefinitely. A candidate requiring expansion is planned before
commitment, not implemented by repeatedly unfolding the selected result.

Candidate keys and scratch must have explicit input-derived bounds. Use iterative
traversal, checked arithmetic and fallible allocation. A fixed round count is not
a termination proof. Unproven: the combined measure and complexity bound for all
migrated rule groups; each implementation slice must supply its argument.

## 6. Finalization and local results

After recovery, derive storage declarations and labels from surviving accesses,
commits, copies, snapshots and transfers. Declaration placement does not move an
initializer or invent a default. Unknown placement retains an established outer
scope; P7 does not solve new liveness or dominance to hide that uncertainty.

P9 has one traversal of the selected grammar, with separate leaf formatting and
origin emission. Removed pure operations retain zero-width anchors; combined
expressions retain every required original identity and commit origin. P9 does
not inspect old composition state or infer control from text adjacency.

Text and JSON expose the same readable fragments and local gaps. A missing use,
copy or region fact affects its actual dependents, not every candidate in the
method. Ordinary no-match, unavailable facts and genuine construction/resource
failure remain distinct. Error kind alone does not determine stop scope.

The proposal retained local results. Constructors retain input safety and necessary invariants;
partial arenas are not complete products. P7/P9 do not fabricate values or retry
another method analyzer. Java legality is not an output gate.

## 7. Required deletion map

| Existing machinery | Replacement | Required deletion |
|---|---|---|
| P6 RegionKind -> P7 ControlKind -> EmissionLayout | One control grammar and leaf/reference conversion | Repeated control encodings, positional role recovery and their translation branches. |
| Repeated descendant membership copies and per-family boundary plumbing | Current-owner relationships and common fragment interfaces | Superseded membership storage, walks and completion mergers where semantic domains match. |
| P7 producer/carrier-specific scans | Three shared query categories and one cover constructor | Repeated value/read/position searches in expression, cast, condition and return handling. |
| Composition, hidden execution and deferred-placement compensation | Structural operation ownership and explicit result placement | Competing execution flags and whole-vector rollback. |
| Coroutine body/loop/flow executable variants | Common controls plus protocol facts | Duplicate control conversion and traversal; retain actual protocol recognition. |
| P9 reconstruction of recovery history | Selected-grammar traversal and attached origins | Composition-history interpretation and alternate layout selection. |

A row is complete only when its old path is removed from the ordinary entry.
A facade around all old code is not replacement. Required effect/exception,
initialization, source identity and local-result behavior remain acceptance
conditions, not optional overhead.

No separate implementation of a whole-method semantic flow graph is planned.
If a recovery family requires a fact that the retained relations cannot supply,
design its narrow owning-producer contract and account for the full added cost.
Do not hide the same analyzer behind an index or replace it with case patches.

## 8. Implementation and acceptance

1. Define the common control records and map every current natural, explicit and
   protocol form into them. Preserve sealed P6/P7 construction.
2. Replace P6 delivery, P7 conversion and P9 traversal as one complete slice.
   Existing algorithms can initially produce the new grammar; remove old layouts.
3. Replace P6 membership and boundary mechanics with fragment composition.
   Demonstrate new combinations without adding complete-shape recognizers.
4. Replace P7 placement and carrier scans with shared queries and joint covers.
   Add normalization and dependency-driven local convergence.
5. Consolidate protocol traversal and derive declarations/labels once. Review
   the net implementation after deleting all replaced paths.

Each completed slice names the old code removed, new production code required,
recovery gained and regressions checked. New helpers count even when placed
outside P6/P7/P9. Representation compatibility is distinct from improved recovery.
The final decision considers both production size and the required capability
set; preserving explicit content does not by itself prove comparable natural
recovery.

Required cases include repeated storage and shared values, ordered multi-operand
calls, inner commit followed by outer throw, branch-dependent uses, loop exits,
shared switch tails, captureless handlers, cleanup overriding pending completion,
monitor release, multiple suspension sites and local gaps.

Follow [EVOLUTION](EVOLUTION.md): record independent expectations, repair the
owning general mechanism, author a harder same-mechanism challenge, perform LLM
path-by-path review and preserve the required prior effects. Inputs remain under
`quality/fixtures/<theme>/<slug>/`; Rust checks use ordinary Engine products from
`crates/diana-decomp/tests/` and run with cargo nextest.

Independent replay/correspondence checks compile only under cfg(test). They do
not call the optimizer as an oracle or construct private stage products. Negative
cases must detect duplicated/reordered effects, wrong edge copies, missing stores
and incorrect exceptional commits. Unsupported semantics are unchecked, not
passing. Renderer fidelity is checked separately from plan behavior.

Implementation status, measured source size and scoped check observations belong
in the [engine README](../../crates/diana-decomp/README.md#unified-recovery-implementation).
The complete replacement must supply the complexity and behavioral evidence
above; preserved contracts and a small schema are not proof of recovery quality.

## 9. Optional optimization policy

Local dynamic programming is justified only by actual competing covers that the
joint selector handles poorly. Its state, phase/store interactions and input-bound
cost must be explicit. It must produce enough recovery benefit to justify all
additional implementation.

Equality saturation, new global value numbering and new path-feasibility analyses
are not core dependencies. They require a separate demonstrated capability need
and an owning-producer design within the existing authority. The current proposal
adds no dependencies, external declarations or runtime quotas.

Reference notes remain in [SOURCE_MAP](SOURCE_MAP.md). The design decision is
based on the operations and deletions above, not on being different from another
decompiler or on adopting a named optimization technique.

The stage-specific chapters referenced by this proposal have been removed and
remain available in Git history. Current contribution and validation rules are in
[CONTRIBUTING](CONTRIBUTING.md).
