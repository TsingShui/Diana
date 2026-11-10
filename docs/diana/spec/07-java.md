# P7 owned method expressions

[PIPELINE_DESIGN §5.7](../PIPELINE_DESIGN.md#57-p7-methodresult) defines the
method-expression contract. The implementation is `crates/diana-decomp/src/p7/`.
Capability and execution observations belong in the
[engine README](../../../crates/diana-decomp/README.md).

## Boundary and ownership

Engine's sole method task calls `build_method(task, planned)` after P6.
P7 consumes mature execution, type, storage and control plans. It MUST NOT decode
instructions, resolve missing declarations, solve types, identify regions or
schedule copies again. `build_declaration` consumes the reader's established
no-code state and P0 declaration without manufacturing an executable graph.

`MethodResult` owns its header and optional `MethodCode`. The header retains the
original MethodKey, owner descriptor, flags, exact declaration, method kind and
method-scoped annotation records. Metadata sharing retains owned immutable P0
records, not a DEX lease. Names retain UTF-16 units and an identifier/original-name
form; constructors and class initializers have distinct kinds.

The original flags distinguish `ACC_SYNCHRONIZED` (0x20, VM locking on native
methods) from `ACC_DECLARED_SYNCHRONIZED` (0x20000, the declared Java modifier,
with no execution effect beyond reflection), as specified by the
[DEX access-flags table](https://source.android.com/docs/core/runtime/dex-format#access-flags)
and pinned ART [modifier definitions](https://android.googlesource.com/platform/art/+/bebbc3cc49f2d9d5420197df0a336fbc3fcbea40/libdexfile/dex/modifiers.h).
P7 MUST NOT synthesize or suppress body monitor operations from the declared
modifier. P9's [declaration spelling](09-rendering.md#declarations-metadata-and-literal-identity)
uses `synchronized` for either bit without changing P7's execution plan or raw
flags.

`MethodCode` contains flat expression, operation, reference, type-use, local,
block, transfer and control arenas. It also owns original coverage, operation
provenance, copy schedules, construction publications, exception chains/sites,
handlers, array payloads/component gaps and upstream issues. Original stage IDs
are typed provenance coordinates, not permission to query a live predecessor.
Type uses retain descriptors and original sources; P8 owns class-wide display
names and imports.

Each operation record owns its optional statement body. StatementId identifies
that body's original operation coordinate, not a second arena position.
Construction initialization references the body held by its original allocation;
prepared arguments retain their own bodies. The delivered construction record
names the sole execution operation. Body lookup and execution lookup consume
these established coordinates without an index or a method scan. P7 constructs
original operation records before lowering bodies; unpublished partial records
remain partial, and a missing required body is a local construction failure.
Original operation/member/commit identities and execution order are unchanged.
P9 consumes the owned body and its original execution reference; it does not
infer placement from the body-owning coordinate.
Selected operation phases are the sole executable before/snapshot/after lists.
Original copy bundles retain their source locations and states. Body records do
not copy those lists. P7's construction scratch supplies pre-publication phase
needs, including synchronized outlets; the owning phase constructor installs a
construction prelude once. Neither representation suppresses source obligations.

Each block retains its original P6 operation range. P7 constructs block actions
from that range and reads body data by original operation coordinate, without
per-node operation vectors. The range indexes only this method's owned operation arena; it grants
no access to a predecessor. P9 consumes it separately for source-only origins.
Rematerialized literal operations contribute zero-width operation/statement
anchors at their original portion without another execution or storage commit.
An invalid range remains an owning construction/rendering error, not an empty
completed portion. Tests retain original PC/identity expectations independently.

A selected expression owns its nested `Evaluation`, which references the same
original operation used by standalone statements. The operation's `nested_at`
names that exact expression occurrence. Its PC, statement, result commit and
exception sites remain in the original operation and statement records. The
expression's original read and value/state remain evidence. P7 MUST establish
both ownership references together and reject a second execution owner. P9 reads
this structure; there are no separate composed-use or composed-result tables.

Condition cover preparation uses a private list of proposed evaluations over
the immutable current product. Subsequent proposals read established and proposed
ownership through the same view. P7 installs the complete list only after every
test admits its schedule. A refusal discards that list without mutating or
copying the method's expression and operation arenas.

The private cover view indexes proposed operand and operation ownership in one
sparse map. Ordinary consumers keep an active suffix of unowned roots; covered
roots leave that suffix once. Installation orders origin associations by original
operation identity and selects each portion's last member for its closing anchor.
That installation order does not change the selected expression's evaluation order.
Scratch allocation remains fallible; these indexes supply no new analysis facts.

P7 derives site annotations and established redundant-check records before
constructing its use index and condition covers. Redundant-check reads do not
become executable value observers. A redundant check leaves the ordinary root
sequence before it can consume any producer. Its zero-width source anchor cannot
own an evaluation or replace the alias/result commit needed by a later consumer.
Remaining producers retain their original suffix positions until a real consumer
admits their joint cover. Required casts are ordinary readonly
producers: their returned operand keeps the original value/state, and their
operation still performs its original check without writing a register. The
common cover requires an available reference binding, established initialization,
empty phases and the original unannotated exception site without snapshots.
It preserves the check's position among all other operation roots. A check is
transparent only when the owning normalization record establishes its omission.
Later carrier placement MUST NOT claim an operation already owned by a nested
Evaluation. A value producer feeding a check retains its original store when
the existing observer facts require it.

P7's private expression walk consumes owned syntax children and selected
Evaluation edges. It publishes entry, completion and nested-carrier events to
P7 consumers, not a new analysis product or upstream fact view. Carrier queries
stop at nested ownership; selected-read queries follow the already selected
producer. Checked child references and fallible work storage retain owning-stage
construction failures. Cast placement consumes the actual statement/terminal
operand order and selects a matching available read before any preceding
observable evaluation. Ordinary local reads and literals add no such event.
Allocation, unresolved expressions and nested carrier boundaries remain barriers.

The single `Control` arena contains each control's selected `ControlLayout`,
execution children, continuation and presentation, alongside its retained source
facts. P7 consumes the common P6 grammar directly rather than translating another
loop/branch/switch layout. ControlId and TransferId reuse P6's original structural
coordinates. Immutable role lists can be shared with the P6 delivery; modifying
a selected list preserves the predecessor and uses fallible element allocation.
`source_children` retains original topology for scope facts and origins;
`children()` is the selected execution order. P9 MUST NOT use source topology as
an alternative execution schedule. There is no separate EmissionControl arena.

`ExpressedMethod` pairs the result with privately retained P6 analysis for the
existing inspection consumers. `inspection()` lends restricted read-only facts
from that same execution. It exposes no owned predecessor or solver.
`into_result()` releases retained analysis and returns Arc<MethodResult>. P8 and
the same-execution inspection response share this immutable owned expression,
without duplicating arenas or borrowing DEX. Partial expressions likewise retain
a shared unpublished prefix; its type-use and local-declaration views do not
promote it to a completed MethodResult. The production method chain has no parallel P6 success path.

P5 Local representatives select execution storage, not new source identities.
P7 retains alias Local records and hides only their redundant declarations.
One private composition index records existing reads, definitions and copy
obligations by original ValueId, separately from aggregate storage involvement.
Expressions match the original output value/state against the exact consuming
read. Reusing a representative does not by itself prevent composition.
An exact array fill likewise names its allocation's value/state, not merely
the reused local. The index also resolves a carrier's pure Move definition
without scanning every statement for each check. It infers no new alias,
availability or dominance relation. P4-established redundant checks remain a
separate omission decision.

A phase-free, site-free Move with a direct read and store to the same reused
representative contributes origin-only text while its operand remains a plain
read and the Move is not itself composed. A composed producer in that operand
requires executable evaluation; it MUST NOT disappear with the former self-move.
A composed Move's remaining read executes at its selected use and participates
in declaration placement there. Source-only attachments remain distinct from
these conditional identity Moves. P7 consumes P5's representative selection;
it does not reestablish storage equivalence.

## Unified operation lowering

P7 uses exhaustive P2 operation/terminal matches for both natural and explicit
control. Lowering starts from established locals, snapshots and exact literal bits.
The final layout either retains P5's original materialization or composes a
definition at an admitted adjacent use. P7 MUST NOT inline an effect merely
because its result has one use. Unused calls, loads, checks and arithmetic effects
retain their original execution and exception sites.

An allocation and its exact whole payload fill can share an array literal across
whole-operation portions using the supplied linear adjacency described below. Every existing type/count/element/storage/copy
requirement still holds. An intervening check or other event prevents this
placement even if another pass removes its separate action; empty effectful
portions are not transparent.

The operation rules cover moves, constants, unary conversions, binary/compare
operations, field/array reads and writes, array length/fill/allocation,
instance-of, check-cast, invocation, object allocation/initialization, monitors,
exception capture and unresolved invocation/result layouts. Reverse subtraction
retains operand orientation. Literal interpretation is P4's view, not a guessed
Java type. `check-cast` is a check with no invented SSA write or replacement
binding.

One private adjacency module indexes the already selected Sequence/Graph sibling
schedule once. Expressions, checks and array literals consume it. Crossing a
portion requires direct sibling Blocks in the same original block, one original
within-block fallthrough, no link copies or required target label, and no control
continuation. It crosses no protected, loop or synchronized container. Each caller
still enforces its operand, phase and effect requirements; this is not another
control or storage analysis.

Required checks first participate in the ordinary ordered expression cover.
A remaining standalone check may use its adjacent declaration or the first
executed step of its original unconditional transfer copy bundle. Other checks
consume one forward query over the supplied run: statement and terminal reads
share operand ordering, occurrence counting and exact binding admission. The
query stops at an effect, copy phase, selection or protection change. Plain reads
and literals before the checked read are transparent. Traversed portions and
actions bound the cursor; a cycle is an owning-stage construction error.
An admitted operand uses the common check producer and Evaluation installer.
Declaration/copy locations retain their InlineCheck records; one check MUST NOT
have both execution owners. Target before-operation/terminal phases and
suspension-owned operands remain barriers. Searching the method for the same
value is not an ordering guarantee.
The carrier names that exact statement and read-expression occurrence, so a second
read of the same storage in the printed form never carries the check. A carrier
read whose state a pure reference copy of the checked value published also carries
it: the copy's input read is the checked `ValueId`/`StateId` itself, so no other
event sits between the check and the copied value. The check's
own statement, operation, exception site, reference, resolved type name and
original copy steps MUST stay in the product with their source origins; composition
removes one display action, not the check event, and P7 MUST refuse the composition
when a phase, route, later operand, second transfer, selected branch or earlier copy
step would move the check past an event it precedes.
Cross-portion terminal carriers must also retain the check's enclosing natural
Try body context at every traversed link. A catch clause is outside its own
Try body's protection and inherits only enclosing protection. Textually adjacent
portions on opposite sides of that boundary cannot share the check's display.
A following statement already composed into another carrier must retain that
same protection context through its selected composition chain.

A check that reads exactly the value a store produced carries its own declaration
instead when four facts hold: that store is the local's only store, the check's
read state is that store's published state, the check action directly follows the
store with no event between them, and the check's reference type replaces the
erased `java.lang.Object` declaration. P7 then declares the local with the check's
type and prints one cast on the store's expression, `T v = (T) <producer>;`.
Both the standalone definition and check keep empty phase obligations and the
same protection context. The inline record names the check's own operation/PC,
not the preceding definition or invocation. Later
reads print the declared narrow type; a read carrier MUST NOT print the same check
again. The store, its expression, the check's operation, pc, reference, route and
origins all stay at their original positions, and a local with another store or a
reused representative keeps the read carrier or the standalone statement.

P7 omits a check-cast's own text only from P4's published `Redundant` disposition,
which is a must-fact over every reaching transfer. The omitted statement keeps its
statement, operation, pc, reference, resolved type name, original route and a
zero-length source anchor at its original position (`OmittedCheck`), and the check
never composes: the surviving carrier prints its original text, so no redundant
`((T) v)` is invented.
A check that carries a before/snapshot/after copy phase, a printed route or an
output definition keeps its statement even when P4 established redundancy, because
its removal could not keep those obligations. `Retained` and `Unknown`
dispositions always keep the statement. P7 performs no alias, reachability or
dominance reasoning of its own.

`Arguments::Unavailable` retains the original register words and storage reason;
it is not an empty parameter list. An unavailable expression retains its concrete
gap and provenance. P7 MUST NOT substitute zero, null, NOP or a fabricated local.
Reference-read errors and opcode-use errors remain distinct.

Boolean display uses P4's range-proved `IntegralView` of `Z`, never a declaration
alone. At such a use, exact zero/one literals print as false/true; an integral
local can print a value test. A local produced by one boolean call, boolean field
read or instance-of expression can be declared boolean only with an established
[0,1] range, complete boolean type
sources, exclusively boolean reads and no copy/snapshot involvement. Other
storage keeps its machine spelling. Copy expressions retain their original
storage contract, and composed numeric expressions keep their existing form.

A constructor or delegated `this`/`super` call also uses the natural surface: its
name is never a Java identifier and the receiver or created type already names the
target, so only an unresolved competing delivered overload qualifies it.

Ordinary calls use a natural surface unless the delivered references leave the
binding ambiguous: an original-name form, a missing signature or argument layout,
dynamic invocation, or another delivered reference with the same owner, name and
arity but a different parameter list or return type. An argument whose displayed
type only widens into the formal parameter is not by itself such an ambiguity;
P7 inserts no conversion for it. Unknown external declarations do not block
ordinary calls.

For competing overloads only, P7 omits the binding disclosure when the recovered
types of the printed arguments select exactly the original target by equal
original type units.
Every argument MUST be a leaf value or literal read with available original read
provenance and exactly one recovered type whose units are known. Snapshot reads,
missing or conflicting types and unavailable type spellings keep the disclosure.
P7 compares only equal original type units at each parameter position among
references with the same owner, name and arity, deduplicated by original reference
identity. Exactly one distinct candidate MUST match, and it MUST be the original
invocation target. A unique match to another target keeps the disclosure.
Custom/polymorphic calls and original-name forms other than constructors retain
their existing qualification rules. This comparison is not Java overload
resolution, assignability, inheritance, widening or a claim about declarations
outside the input.

The signature qualification discloses the original reference; it is not a Java-legality
device. Both forms retain the complete original reference, dispatch kind,
receiver, argument order and invocation prototype. P7 MUST NOT insert casts,
typed aliases, boxing or helpers to influence binding. `LocalSpelling` uses a
nominal name only for complete, equal delivered reference type sources; other
locals retain their machine storage representation. Header parameter names are
fixed by P7, including no-code declarations.

P7 selects `Parameters` when the parameter list distinguishes the original
reference from its competing delivered overloads. A competing reference with the
same parameters and a different return type selects `FullSignature`; other
mandatory disclosures also retain that full form. The selection scans all
competing references, so their delivery order cannot hide a return-type collision.
Both qualified forms carry an owner-disclosure flag. For an instance call, P7
omits that disclosure only when the final receiver remains a local read whose
nominal `LocalSpelling` has exactly the original owner's type units, with no
composed producer or inline check at that read. Owner selection runs after
expression and check composition. This is display
selection, not hierarchy resolution; inheritance or multiple implementations
alone do not select qualification. Static calls already print the owner.
An otherwise natural instance call or field access whose displayed receiver is
only `java.lang.Object` can disclose its different original owner without adding
a signature. A check already printed at that exact receiver occurrence needs no
such disclosure. This names the original reference, not a new runtime cast or
a replacement SSA value.
P9 prints the selected signature and owner according to
[the call notation](09-rendering.md#expression-and-control-notation), without
recomputing these decisions. Unread references keep their concrete unavailable
form rather than a guessed signature.

P1 supplies checked call-site IDs, bytes/range, argument count and flat bootstrap
events. A separate flat leaf-declaration pool retains referenced strings, types,
fields, methods, prototypes and handles. P7 copies these facts into owned reference
records, preserving container order and literal bits. It does not reopen encoded
values or replace custom/polymorphic calls with ordinary calls.

### StringConcatFactory presentation

P7 can attach an inferred concatenation presentation to an original Custom call.
The original invocation, argument list, reference, bootstrap events and effects
remain authoritative. The presentation assumes the named standard bootstrap
contract; it does not prove runtime linkage or replace the call with ordinary
calls. P7 recognizes only the exact StringConcatFactory owner, static handle and
the complete `makeConcat` or `makeConcatWithConstants` bootstrap signature. The
generated call name is arbitrary. Other bootstraps retain their original form.

The call-site return is exactly String and every runtime operand has an available
original read and a compatible P4 String/primitive interpretation. P7 consumes
P4's proved narrow integral views, including the long view; declarations alone
do not establish ranges. String reads retain established initialization and
complete String sources. Numeric-bit storage, unknown interpretations, objects,
arrays and nonfinite bit literals retain the explicit call. Primitive char/int
spelling follows the original ABI view and actual parameter/local display type,
without changing values, local storage or binding. Parameter header types remain
distinct from the early machine-storage spelling.

The recipe is read from owned flat bootstrap String facts, never reparsed from
input bytes. Dynamic tags consume each original operand once in order; constant
tags consume each static constant once in order. Missing or extra tags keep the
original explicit call. Literal recipe portions reference their original UTF-16
ranges. Static String, signed integral, boolean and char constants have a retained
one-time standard String-conversion presentation; other constants stay explicit.
A tag inside a static String remains literal content. An empty-prefix flag is
String-conversion notation for numeric starts and single-operand null handling,
not an extra original operand or a fabricated SSA value.

The selected view consumes already-captured operands. P7 keeps calls, field
reads, casts and arithmetic argument producers at their existing positions.
The existing resolved-String composition may apply only before the first printed
join and with its original adjacency, phase and exact-use guarantees. Copy,
snapshot, exception, initialization and result-commit phases do not move.
Plan allocation failure records a classified unavailable presentation on that
call, preserving its original form and independent calls; it does not stop the
method or erase previously recovered presentations. P9 consumes this owned
decision without discovering protocols or solving argument conversions.

## Materialization and ordered phases

A statement retains ordered operand reads, P5 definitions and distinct
before-operation, before-throw snapshot and after-normal-commit copy references.
Normal transfer copies execute only after selection of that original transfer.
Handler receives execute only on exception arrival, after the original capture
when present. `HandlerCapture::Body` and `Catch` distinguish the unique capture
owner; a handler label MUST NOT bind it a second time.

Each copy bundle retains P5's requirements separately from executable Save/Assign
steps. P7 maps storage reads and destinations without rebuilding a parallel-copy
schedule. Unavailable assignments remain gaps; a plan ID alone does not authorize
an assignment. Pure literal definitions can omit an emitted statement only when
all publication plans are literal and no before/snapshot/after copy is attached.

A store prints as an ordinary assignment unless its array, index or receiver slot
is an inlined expression or an unavailable operand. Those slots are never
composed, so reading them before the value cannot be observed and the store's
null/bounds check still follows the value. A narrow element store keeps its
original width marker.

A void `return` that is the last printed action of a method body is the same as
reaching the closing brace, so P7 marks it display-omitted when nothing is attached
to it: no copies, phases, site or unavailable definition, and no statement after it.

A break or continue whose target is the innermost enclosing scope needs no label,
and a continue that ends the loop body is the same as reaching the body end, so the
final plan prints neither. Its attached copies remain ordered statements.

A tail-test repetition-edge copy set may move into the body end when the delivered
facts prove both schedules equal: the predicate has one normal arrival from inside
the body; the condition reads no destination the copies write; the exiting-edge
copies neither read nor write those destinations; and no read outside the loop
subtree, handler receive or construction observes them. Copies are plain
assignments, so moving them introduces no effect or exception site. A head-test
predicate is reached on entry with different state, so those copies stay on their
edge.

A loop predicate becomes an inline condition only when its delivered evaluation
subtree consists of label-free, continuation-free Sequences and Blocks with no
remaining statement or terminal phase. Suppressed composed producers and copy-free
original fallthrough links retain their origins in that subtree. Edge copies may
remain: they are printed inside the condition by their original repeating or exiting role, so a break still
does not execute exiting-edge copies.

A control's planned continuation contributes no second action after a printed
unconditional goto, return, throw or selected break/continue. P7 consumes the
completed block-end spelling and follows only a last-child Sequence chain with
no intervening continuation. A scoped exit stops before its own target; opaque
Try, Loop, Switch, Synchronized and other containers are not transparent to this
rule. A throw may recover through such a container, and breaking that container
may require its own onward completion. Conditional and origin-only terminal ends
do not establish an unconditional printed exit. Original transfers, terminal
phases, copies, source continuations and origins remain in their arenas; only
the unreachable second display action is omitted. This is layout selection from
supplied products, not output replay or another graph/dominance computation.

A jump, including a planned continuation, whose target prints next is ordinary
sequencing, so P7 MUST omit its displayed `goto`. The search follows the final
emission layout through Sequence, Labelled and Graph containers and the last
printed If branch. It stops at a parent's continuation and at Loop, Try, Switch
and Synchronized boundaries, whose trailing text or actions prevent fallthrough.
For the successor, P7 follows the established entry through a Sequence's first
child or an If, Condition or Switch header only when every traversed container
has the target as its entry. It does not enter a loop, protected scope or graph to find
a target. This same entry rule applies to transfers and planned continuations.
Selector transfers print inside their label table rather than after their owning
Block. P7 MUST NOT collapse their supplied gotos using that Block's statement
successor; only an already selected case-layout interface supplies case
fallthrough. Otherwise an earlier default/case would execute the next label's
work instead of its original destination.

P7 MUST omit an `else` emptied by this collapse only when its false-edge copies
are empty and its control is an empty Block or a label-free Sequence of empty
children. An empty owned child alone does not establish NoElse: every late
display decision also requires its selected-edge copies to print nothing. A
printing assignment stays inside its else phase, never between the if predicate
and opening brace. Each such control has no required label, continuation or deferred
control; each block prints no text and has no displayed label, which means every
action is an origin-only `nop` and its end prints nothing. An empty block end is
None, a list of Fallthrough transfers whose copy bundles have no executable steps
or unavailable assignments, or a Goto terminal with no terminal phases and such a
transfer. Other terminals retain the branch. For an explicit If terminal, P7 MUST
omit the false arm only when its transfer is such a Fallthrough and all terminal
phases are empty. P9 consumes this one display choice per branch terminal without
recomputing emptiness: each terminal selects exactly one of both arms, no false
arm, an inverted single arm, or no text at all, so two display decisions can
never contradict each other.

When an If's true arm is empty under the same rules and its false arm prints, P7
selects an inverted single-arm display using `PredicateEmission`'s exact negation.
This covers both a syntax-owned If control, whose true arm is its child control,
and an explicit If terminal, whose true arm is its Fallthrough transfer. The
predicate is still evaluated once at the original position, so neither form adds
or removes an evaluation. Original true/false edge identities and child ownership stay
unchanged; the false-edge copies begin the displayed body. A true-edge copy
bundle with no executable steps or unavailable assignments is nonprinting; P9
retains its zero-length copy and transfer origins without moving any step.
The omitted true child retains zero-length control, label, terminal and silent
transfer origins at the displayed arm; P9 emits those anchors without evaluating
or printing the omitted child. A displayed predicate exception site prevents
inversion. If both arms are empty, P7 suppresses the test only when it has no
terminal phases, displayed site or
composed operand evaluation, and its control has no continuation or deferred
control; its header still prints at the original position. This omission requires
both arms' copy bundles to have no executable steps or unavailable assignments.
An explicit If terminal
with two such Fallthrough arms uses a separate empty-If display flag under those
same predicate and control restrictions. P9 retains both edges' zero-length copy
and transfer origins.

P7 selects `else if` only for a copy-free false edge whose body is one printable
If, possibly inside label-free Sequences of otherwise empty children. Those
containers and the inner If have no required label, continuation or deferred
control; the inner header is empty and has no before-terminal copies. P9 prints
the selected chain without another scope or duplicate child. These display
choices preserve the original blocks, controls, labels, transfers, sites, copies
and exception routes.

A syntax-owned If can select `IfShape::Guard { inverted }` when its selected
exit arm is an entire linear schedule of plain Sequences/Blocks ending in an
actual printed return or throw. P7 MUST reject required labels, continuations,
opaque controls anywhere in that arm, non-fallthrough prefixes and origin-only
or omitted terminals as evidence of that exit. A return inside a recovering
Try is not sufficient. Deferred origin spans alone are not control transfers;
already composed statements still execute at their established uses.

The guard can use either original edge. P7 supplies its exact predicate polarity;
the other edge's original copies and complete body follow the guard in the
enclosing scope. This removes an else wrapper, not its work. No updater,
dispatcher, body clone, new target or graph analysis is introduced. P7 MUST NOT
classify a Guard as a single nested If for else-if composition: it also prints
the remaining body. Declaration placement visits the remaining body in its
actual enclosing scope, while the terminal arm remains nested. Original calls,
copy phases, definition/commit positions and exception routes do not move.


Each exception site also records whether its route still needs an inline marker:
a natural single route targeting an enclosing printed catch does not, because the
chain line and the catch scope already state it. Other sites keep a compact chain
reference.

P5-selected construction groups become `New` expressions. P7 places each
P5-selected producer at its original consuming read within the ordered argument
trees, so `new`
evaluates allocation, preparations and initialization in their original order.
Root arguments retain their original read origins. The allocation receiver is
protocol storage, rather than an extra unevaluated expression read that could
prevent a scoped declaration or inflate its use count. P5 retains an explicit
plan for other intervening operations. The group retains
allocation and initialization operations, member/edge provenance, arguments,
normal publication point and all throwing sites. Its selected common snapshot
prelude executes once, before the entire New evaluation on its initializing
execution owner. It does not remain on the source-only allocation portion or
run after constructor arguments. P5 establishes the saved sources at the
allocation anchor and the closed preparation corridor; P7 consumes that proof
without relocating unrelated copies or re-identifying construction. Original
snapshot bundle identities and source locations remain facts. Alias publications
occur only after successful initialization; failed construction does not invent external assignment or undo
callee effects. Unselected construction relations retain separate allocation and
initialization statements. `this`/`super` syntax requires the existing pending
initialization fact and an original first operation without preceding copy/save
obligations; otherwise P7 retains explicit initialization at its original position.

## Control, protection and synchronized outlets

P7 retains P6's selected Block/Sequence/If/Loop/Switch/LabelledBlock/Try/Synchronized/
Graph kinds, child ownership and original continuation evidence. Labels have fixed
method-local display names. Block completion distinguishes ordinary terminals from
syntax-owned predicates/exits; retained evidence is not a second emission.
All original switch roles, duplicate-target links and selected-edge copies survive.
P7 consumes P6's optional case groups without finding regions again. Its switch
emission lists the supplied bodies in lexical fallthrough order. Each group
prints its original case/default labels once, then its single body, without a
dispatch goto. A supplied acyclic order can also retain nonadjacent gotos from
several cases into one original case entry; that entry's label stays available,
and the body is not copied. P6 has established zero-step selector copies for
shared labels and entries reached from other cases; P7 does not repeat that
control or storage analysis or decide which predecessor falls through. An empty
default body still executes its original selected-edge copies and delivered
switch completion. Case-body normal completion can be a
selected Break; a successful original Return/Throw bypasses it. P7 lowers P6's
scope completion without discovering another target, and chooses an unlabelled
break/continue only for the supplied innermost selected scope. The selector
terminal and arm transfer identities remain owned.
Other switches retain the single-body dispatch layout. P9 follows these placements;
it does not split graphs or infer private membership from printed text.
Explicit graph transfers use the same statements and storage as natural regions.

P7 MUST copy only P6's published reader-facing diagnostics into `Issue::Control`.
Under the [P6 publication contract](06-regions.md#planning-state-and-published-diagnostics),
these are `Storage`, `Lock` and `Protection` records, not `Graph` planning records.
P7 retains the selected graph reason in its control expression; P9 prints it once
in `graph<reason>`, not as a duplicate method-body `diagnostic` statement.
Rejected-candidate reasons at non-graph entries are not method statements or graph
annotations. P7/P9 MUST NOT re-derive or display P6's private candidate history.
The retained non-graph diagnostics keep their existing `diagnostic` spelling,
content, order and method-head placement; no other text states those issues.

P7 copies each P6 handler's contiguous original clause range alongside the
unchanged chain and full interval. One handler child owns the shared capture and
receives once. P9 prints the supplied adjacent type alternatives on that single
catch header; neither stage groups targets, moves clauses or infers hierarchy.
A delivered header binding removes only the capture's body action and hides its
method-level declaration. P9 prints the original local in that header and anchors
the original capture operation/statement there. Receiving copies still follow
capture. A binding with missing producer facts is a classified construction error;
unqualified captures retain their original statements.

Exception sites retain original completion, ordered chain, edge delivery and
snapshot provenance. Natural catch delivery, explicit protection, propagation and
P6's method-local ImpossibleRelease finding stay distinct. P7 MUST NOT discard an
original exceptional edge or invent a handler from syntax adjacency.

A selected P6 `FinallyPlan` lowers to the existing Try control with a delivered
finally layout. Its source children retain the protected body, handler cleanup
and original normal duplicate. The executable child order contains the body and
one cleanup spelling; the duplicate is an origin-only attachment. The original
capture and all pending rethrows likewise contribute origins without text, and
their private capture local needs no declaration. Original operation, predicate
and explicit cleanup-terminal pairs remain in the owned control product. Normal
duplicate predicates and cleanup terminals retain origin-only placement; the
canonical cleanup returns/throws remain executable. Private duplicate result
locals and natural construction publication destinations need no declaration.
P7 does not infer equal values or establish cleanup correspondence.

Origin-only occurrences cannot carry a composed check-cast or invalidate an
otherwise scoped local declaration merely because they have no executable
display location. Statements, expressions, sites and transfers remain in the
source arenas. P9 consumes these placements through its existing event stack.

Each synchronized outlet identifies its release statement, entry label,
original transfer and exact resume intent. Before-operand copies precede the
release read; snapshots follow that read. `completion` orders after-commit copies,
selected transfers with their copies, and routing-terminal phases after release.
The retained `after_release` list describes obligations, not a second schedule.
`Resume` describes the established destination/abrupt intent, not a second terminal
event. Rethrow retains its exact original expression; capture is provenance only.
P7 MUST NOT move unlocked work into the held body or infer outlets from a common
follow. These outlet records retain source ownership; they are not another
execution route alongside the final emission plan.

A ReleasedSync transfer consumes P6's supplied release-block continuation rather
than initializing an additional ordinary goto. Its emission spelling is
Fallthrough; the source control completion delivers the original next, break,
continue or goto once. Original copies and routing nodes still execute in their
original phases. A direct release-to-body-update link keeps that original
ordinary target; its supplied completion must not be replaced by a continue
that skips the update.
LeaveSync still names the original release entrance; it is not another release.
A scope without a shared follow consumes its separately supplied original outlet
completions. P7 MUST NOT fabricate a common destination, execute another release
or interpret its closing brace as an implicit route into one unlocked tail.

A `new-instance` statement that publishes its own storage keeps the original
allocation point: P7 marks it so P9 prints the allocated type and local where the
allocation runs, and the header omits that local's declaration. The constructor
call stays at its own statement, so neither the allocation nor its publication
moves.

P7 selects a local's `local_initializer` after expression, cast and control
composition. The selection attaches its declaration to an existing single-output
move, evaluation or construction statement. The earliest candidate assignment
must precede every other identified write to that storage. A
construction qualifies only when its sole publication is its own deferred
carrier. The initializer's expression, exception sites and commit stay in place.
P9 uses the selected statement ID and existing `LocalSpelling`; it does not
choose scope or infer an initializer.

The declaration pass consumes selected control roles, original read origins,
composition links, copy locations and construction arguments/publications.
It follows composition links to each access's final display location. P7 MUST
retain an outer declaration unless every identified read and write follows the
selected assignment and remains in its lexical scope or a nested scope. Branch arms, loop bodies and predicates, catch bodies and switch
cases have separate scope boundaries. Cross-branch merges, copy destinations,
snapshots, parallel temporaries and unavailable local storage
retain their outer declarations. A read without an established display location
keeps that local's outer declaration. Explicit graphs and predicate evaluation
blocks retain their existing declaration form. These choices do not change the
original storage identities, copies or recovery gaps, and do not rebuild CFG,
SSA or dominance. Plan scratch failures retain the existing P7 error classification
and partial-product behavior.

Ordinary declaration placement selects materialized storage visibility after composition.
Composed results distinguish value-only evaluation from retained Store
materialization. Removing one definition does not hide a shared local still
needed by another retained write. Declaration scope checks consume these
selected writes and the final locations of composed reads, rather than treating
an eliminated original store as an executable assignment. Original definitions,
commits, copies and state identities remain inspectable.

Closed loop and multi-site protocol finalization share one source/ABI storage
observer query over the owned P7 product. It consumes original operation ranges,
body input read origins and Store definitions, construction argument reads,
non-guard terminal inputs and selected edge-copy steps. The matched continuation
occurrence alone is implicit. Literal/unavailable reads establish no invented
local, and an unavailable item does not erase independent observers. The query
does not reopen P5 uses, infer aliases or move an operation. It handles Assign
and Save from their delivered source/destination facts. Existing protocol-copy
forwarding still determines selected storage spelling separately; original read
origins retain their original ValueId/StateId/storage correspondence.

`EmissionPlan` is P9's sole execution layout. It binds the supplied P5 copy
locations to operation, terminal and selected-edge phases without rescheduling
steps. It fixes child roles, condition polarity, method-local control names and
scope continuations. Control completion distinguishes goto, break and continue
without inventing another edge copy. Syntax-owned header/predicate continuations are deferred to
their owner, so a header cannot jump past its own test or monitor acquisition.
Loop-condition edge copies stay inside that test's selected-edge clauses; they
are not appended after a loop. `LoopPlacement` distinguishes infinite, head-test,
tail-test forms, and records when an empty predicate can use a compact form.
P7 MUST NOT select for/enhanced-for syntax or a header-owned update. Original
initialization and updates remain ordinary statements. A body-update target
retains its explicit transfer when it is not the actual while head or do-while
predicate; no compatibility schedule or duplicated update replaces it.

A noncompact head test retains its original evaluation and explicit exiting
test at the body's start under `while (true)`. An assignment before the first
test still runs there. Tail tests stay after the first body, including when
copies or effects require an explicit predicate phase.

A synchronized layout states whether P6 supplied an implicit monitor surface.
The explicit layout prints releases and routing at their original nodes, with
original lexical try/catch ownership; its closing brace adds no release.
The implicit layout owns one release on each selected completion. Original
release actions become origin-only, and the qualified compiler catch becomes a
source-only attachment rather than an execution child. Earlier typed catches
stay executable and their original clause indices remain. After-release work
and value returns stay outside the held body. Entry phases remain in place. A selected compound
construction prints at the original constructor call; correlated members that
only alias the object or define literals remain suppressed. These are
placements of existing plans, not region identification or a new dispatcher.

P6's optional integer `DecisionPlan` lowers into owned `Decision` records and
`EmissionLayout::When`. The original If controls remain the source ancestry.
The alternative's bodies define printed succession; a case is not the next case's
fallthrough. P7 adds no selector evaluation, state local or dispatch edge.
Each selected-edge copy list executes only at its case or default entry.

## Composition before publication

`p7/compose` consumes the lowered P7 operations, P5 materializations and the
completed P6-derived emission layout. It does not decode input, rebuild SSA,
identify regions or reschedule copies. Composition modifies the selected control and expression structure and its
emission facts before `MethodResult` is published. P9 prints those products.

Ordinary If, ordered Condition and Loop predicates share one producer,
exact-use matcher, branch-role gate and composition installer. Generic layout
consumers use selected emission children and continuations; retained source
children continue to describe lexical ancestry, not printed succession.

The expression pass composes a move, a resolved string, an integral/floating unary
or binary definition, or a numeric/reference field or call definition only when its
original output has one established value identity, has one compatible value
use, and carries no copy/snapshot phase. The definition's P5 ValueId is
independent of its Local's kind or number of stored values. A move's source must be a
delivered direct value read with a compatible view. A reference source and use
must each have an established initialized state; allocation carriers and other
position-related initialization states remain separate.

Ordinary operations and ordered Condition tests share one ordered suffix cover.
It matches exact consumer operands against source roots from the end, retaining
strict operand order. Earlier preparation already owned by a root belongs to
that same forest. An unmatched operation stops an ordinary suffix; a skipped
Condition run requires its entire executable prelude. Source runs cross only the
existing copy-free direct sibling adjacency in one original block. No
before-operation copy is crossed. Arithmetic, receiver and call operands use
one leaf-or-owned-evaluation admission rule.

Resolved strings retain their original reference, UTF-16 units and
propagation-only exception site. Concat admission still requires its delivered
string-composition permission. Construction arguments consume P5-selected
preparation. Stores admit only the existing pure producer or resolved-string
case, require plain earlier operands, and forbid nested producer preparation
because their selected spelling can print the receiver before the value. A return or predicate use must be the next original
node in the same block, with one incoming fallthrough, no intervening work or
terminal phases, and adjacent selected Sequence or Graph children. An effectful
producer must have one original exception site that propagates without a snapshot.
A predicate uses the producer as its first captured operand. When both original
branch transfers have the same destination and no copies, the producer remains a
statement instead of being moved into an otherwise empty If. All operands remain
in original order. Locals participating in copies or construction publication are
not hidden. An unavailable copy dependency prevents this optional composition
for that method.

P7 may compose a selected construction into an immediately following `throw`
when its single published local has one initialized read at that terminal, no
copy involvement, and the intervening fallthrough and printed controls carry no
other work. Construction and throw sites MUST retain the same delivered handler chain and
exception spelling, without snapshots. Propagating, natural and explicit
protection retain their original delivery; an ImpossibleRelease cannot qualify. The
construction, its members and their sites remain in the delivered product and
the composed expression retains their origins. Prepared operand expressions
own their operation/result anchors; the construction owns the complete set of
throw-site anchors, without duplicating either.

For later test runs of an ordered Condition, P7 also supports a store-preserving
composition when the result has additional reads. `EvaluationResult::Store` names
the original local; P9 prints `(local = expression)` at the exact matched
terminal read, and the local's declaration and other reads remain visible.
The original value/state/use identity, initialization, commit, exception and
phase requirements still apply. A local read by original normal-edge copies
requires store retention, even if its expression arena has only one read.
Copy destinations, snapshots, non-edge copy phases, relocated copies and
construction publications pin the definition to its separate statement.
The first test run keeps a multi-read definition
in its printed prefix; ordinary operation, If and Loop composition still require
one read. P7 MUST NOT substitute a second evaluation for the retained store.

A single-use `cmp-long` can compose with an integral-zero branch when both source
operands have established long views. P7 delivers `EvaluationPredicate::LongComparison`
with those original operands and the branch operator; P9 prints that relation,
not a subtraction that could overflow. The comparison's original operation,
expression and consumed read retain origins at the predicate. Pair comparisons,
other uses of the numeric comparison result, store-retaining compositions and
floating `cmpl`/`cmpg` remain outside this rule. The shared adjacency, phase and
exact-use requirements still apply.

An original `instance-of` can compose with an immediately following equality or
inequality branch only when its checked type reference is available, its original
exception propagates without a local handler or snapshot, and the same adjacency
and storage requirements hold. Its check and possible exception still occur once
before branch selection. No cast or new SSA definition is invented.

A call can compose at an adjacent use when its receiver and arguments are leaf
reads or literals and its exception route propagates without snapshots. P7 gives
the call boolean shorthand only when its delivered storage family is integral and
its complete, nonempty recovered type sources all name primitive boolean (`Z`).
An integral call whose complete, nonempty sources uniformly name primitive int
(`I`) keeps its numeric comparison and uses the ordered Condition producer path;
mixed or unknown sources refuse that shorthand. Other call results keep their
numeric or reference interpretation. A call with boolean shorthand composes only
into a terminal, not an operation. For a terminal, the call is the last original
operation before the empty, single-entry header; no copies, phases or intervening
evaluation move across it. The same eligibility and installation path accepts a
P6-owned loop predicate when its producer and terminal are adjacent children of
the delivered evaluation Sequence. P7 does not find another loop or exit corridor.
The producer still executes initially and on every original head continue, or
stays at its original tail position. Call results retain their `AfterResult`
commit and move-result PC origin; other candidates still require `AtOperation`.
Calls with printed exception routes remain statements. Additional reads require
the store-preserving Condition form described above; value substitution retains
the single-read requirement.

A delivered ordered condition composes a test's prelude only as an ordered
suffix of that test run's statements. Each composed operation must name a
distinct terminal operand and the matched operand positions must strictly
increase, so P9's left-to-right operand evaluation reproduces the original field
read, call, exception and commit order. Earlier statements stay printed in the
header when that run prints; a later run must be composed completely because it
prints no header of its own. A boolean definition composes only into the same
`Eqz`/`Nez` integral-zero branch role as the ordinary boolean path; a pair or a
different comparison keeps its numeric materialization. When any run cannot meet
these order, copy, phase or branch-role conditions, the condition keeps the
complete explicit schedule from its delivered P6 plan, including when local copy
access is unavailable. P7 restores terminals at the delivered predicate portions
and lowers the delivered child order, goto links and per-child normal completions
into `EmissionPlan` before selecting labels or display omissions. P7 MUST NOT
collect descendant blocks to discover that order or infer completion from a
printed successor. It MUST retain the Condition's tests/shared/other source roles
and original continuation evidence. Each referenced child keeps its existing
operations, terminal phases, copies, handlers and abrupt-exit scopes; its emission
continuation comes from the selected P6 alternative.
The ordinary display pass can omit a goto to the next printed target, as for
other completed layouts; that does not reconstruct a condition schedule.
The tests' selected edges and all intermediate other edges require no copy
bundles. P7 also consumes P6's supplied empty-goto routing links and child. Only
after every test admits compaction does P7 give those links silent spelling and
clear the routing child's and original block owners' printed continuations.
Original destinations, terminal identities and lexical owners remain intact.
The routing child retains zero-width origins in the compact layout; the explicit
alternative retains its original links and complete child schedule. P7 MUST NOT
discover routing paths from the graph or make a copied or effectful path silent.
The final other edge can retain copies at an explicit else position:
every bundle must retain original emission, name that same original normal edge,
and have no unavailable assignment. With no other-arm body, P7 selects
`ConditionElse::Copies` only when those bundles have executable steps or an
unavailable assignment. Complete zero-step bundles keep `ConditionElse::None`
and their Copy/Transfer anchors at the final test; a nonempty list of plan IDs
alone does not create an empty else. The existing branch display predicate supplies
this same no-words distinction without solving storage. With a delivered body,
it retains
`ConditionElse::Body`. Copies remain on the final other edge, never in the common
continuation or on an earlier selected edge. P7 adds no region, local or copy
schedule to represent this else. When P6 supplies `other_exit` for a no-body
boundary arm, P7 instead selects `ConditionElse::Transfer`. Compaction retains
that final original edge's spelling and copies rather than clearing them with
the intermediate test routing. Its else executes the supplied boundary completion
once; the condition's common continuation is not a substitute. P7 does not infer
an outlet or search another boundary.

P7 installs compositions only after every test run admits its schedule; a
refused later run leaves no hidden local, relocated store or copy-only else. A resolved
statement's operations, statements and result PC remain owned and print at the
composed operand; no call is deduplicated.

With `trace-passes`, the condition pass reports one final layout decision per
Condition, identified by method and control. An explicit decision reports the
first blocking test and, when identified, statement and admission reason.
Installed store compositions additionally report their operation, statement,
operand and storage identities. A copy-only else reports its original transfer,
bundle and executable-step count. These are decisions, not counts of removable
gotos or independent semantic verification. An ordinary build emits no such
diagnostics.

The expression pass selects terminal roots, then prepares ordinary consumers
in source order within each established straight run. Each consumer matches one
joint suffix from the end. Previously selected roots retain their owned
preparation; the pass introduces no second expression-analysis route. A store
remains a nesting boundary. P9 renders nested evaluations with a
fallibly allocated work stack rather than host recursion. Original operations,
PCs, statements, transfer records and gaps remain owned; composed operations map
to the expression range, and a cross-node source control spans through its
composed use.

The layout pass keeps labels referenced by explicit transfers, graph entries,
exception handlers and construction continuations. Unreferenced labels retain
zero-length source anchors. An unreferenced Sequence or If wrapper can be
transparent. A graph scope keeps its braces, graph header and children. A
labeled scope prints its control name only when a printed transfer names that
control. P7 marks a labeled scope transparent when it prints no name, has no
continuation or deferred composition, and has one unnamed printed child; P9
then prints that child without the scope's own braces. The original control,
labels and transfers remain in the product, including a bare `break` whose
stored target is the transparent scope. A loop condition becomes ordinary
`while`/`do-while` when its delivered evaluation subtree has no remaining prelude
work after shared adjacent-expression composition. Terminal phases still constrain
compactness; selected-edge copies retain their original repeat/exit clauses.
Otherwise the existing explicit test retains all prelude and edge obligations.
Pure propagation chains can be implicit only when no displayed site or handler requires the chain.

Integral literals encoded in integer arithmetic instructions retain their original
bits and an integral view. Other bit-pattern literals keep their supplied views.

## Suspension surfaces

P7 consumes P6's complete common control records and established protocol
associations. It keeps one `Control` arena and one `Protocol` metadata record,
with calls, implicit continuation occurrences, ABI entries and selected copy
forwarding. SuspendLambda and ordinary continuation nominations remain private to P6;
P7 publishes no separate executable body/loop/flow layout.

Original controls retain source topology and semantic metadata. Selected children
follow P6's common grammar; appended compounds have no second source parent.
Sequencing/label consumers construct their parent index from the selected root,
not by treating every retained source container as an executable owner. Unselected
original containers retain individual origin associations without executing their
children. Allocation is fallible and traversal is iterative, bounded by the
published static structure. A missing required coordinate retains the existing
method-private construction stop, not a reconstruction retry.

A `Scope` names code, ABI or origin presentation. An annotated block completion
retains its original terminal and marker edge and selects the supplied normal
transfer. Ordinary Loop records retain their original predicate, continuation
target and repeated/exit roles. P7 MUST NOT choose a latch, rematch a marker or
retarget a resume corridor. An unqualified local suspension leaves the original
header, call, marker branch and dispatch available without a second published
site product. Accepted call/state/resume correspondence comes from `ProtocolPlan`.

The original call remains at its statement with the same receiver, visible
arguments, target, site and result-commit identity. Only its matched continuation
occurrence becomes implicit. `Operation::publication` names the existing successful
result storage, copy, transfer and copy-source expression. Original output
definitions remain provenance. `CopyEmission::Publication(call)` excludes that
bundle from independently executed edge/body phases; the resume attachment keeps
its original failure check and copy to the same storage.

Protocol state/spill events become ordered `BodyAction::AbiOperation` barriers.
P7 MUST preserve their position when covering expressions or cast carriers. A
producer before state publication cannot move into a call after it. A suspension
cannot become another producer's nested evaluation, and a preceding cast cannot
acquire its implicit continuation as a carrier. Code and ABI visibility consume
the same owned observer query; construction arguments and selected copies retain
their actual storage observers.

P7 can forward a loop-local normal copy only when every executable destination
read follows it and precedes a source write in the same iteration. Outside reads,
other destination writes, parallel saves, copy-carried checks and alias cycles
reject forwarding. This existing per-copy proof consumes original P5 accesses
and P6's supplied interval order; it is not register replay. `ProtocolCopy` keeps
original step/storage/read identities. P9 consumes the chosen spelling without
establishing another alias. ABI attachments keep original names and copies.

`kotlin_continuation_view` selects Kotlin body grammar for the closed alternative.
The declaration retains its original signature style with an inferred `suspend`
modifier. Reference predicate equality retains identity semantics; missing operand
interpretation remains Unknown rather than adding `equals` or null checks.

## Failure and evidence

`ExpressionError` preserves concrete request/resource errors and necessary
construction invariants. `ExpressionStop` retains mature P6 facts and an
unpublished `ExpressionPartial`; partial arenas are not a usable MethodResult.
`fragments()` exposes operation-associated statements with closed dependencies.
Unassociated arena entries remain construction evidence and can still reference
pending records. Copy/storage prefixes do not promise a complete control layout.
Local input/type/storage/control uncertainty remains in the owned mature result
and affects only its actual consumers. No error kind automatically truncates
siblings or promotes a construction prefix to success.

The previous core test was removed by operator decision. Ordinary Engine
requests do not run independent replay, correspondence passes or a verification
switch. P7 JSON serializes the inspection product; final text and UTF-8 spans are
owned by [P9](09-rendering.md), without a Java-legality gate. P8/P9 MUST consume
the existing expression and naming products rather than repairing or re-analyzing
execution. Corpus completion is not an arbitrary VM, heap/callee or lock proof.

## Source-only control attachments

Control's `source_only` list retains original controls that have no
executable placement in the selected surface. The execution child list excludes
those controls. P9 visits their existing products with zero-width origins; it
performs no reconstruction or semantic retry. An implicit monitor's qualified
cleanup handler uses this attachment. Ordinary operations, snapshots and monitor
facts remain available in the source arenas; an attachment is not an alternative
execution schedule.

Exception site annotations consume the selected surface. A naturally delivered
site can omit its repeated chain marker when every original clause is represented
by the printed surface: an enclosing visible catch/finally handler, the delivered
implicit monitor cleanup, or the method-level uncaught completion. The omitted
form requires all of the following:

- the route count equals the original clause count and each uncaught clause has
exactly its `Explicit` route;
- every typed clause's route is a `Catch` delivery whose selected control encloses
the site and whose supplied emission layout prints that handler position;
- resolved controls satisfy original priority order, so each later clause's
control is the same as or encloses the previous clause's control;
- within one control, handler positions never move backward, and an equal
position is permitted only when one printed union clause group covers exactly those
clauses and therefore names both types at that single handler.

Otherwise the marker and its chain line stay. P7 MUST NOT infer a handler from
printed adjacency, reorder clauses, or drop a chain whose printed positions state
fewer types or a different order. Original ordered chains and targets remain in
the owned product; P9 does not point readers at a hidden compiler handler through
a redundant chain line.
