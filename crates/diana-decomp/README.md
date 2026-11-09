# diana-decomp — native engine library

Diana's independent native Rust engine borrows caller-owned DEX bytes. Files,
ZIPs, network access and output ownership belong to the application. Production
forbids unsafe code and external runtime/process dependencies; declarations come
only from the analyzed input.

The active method chain is P0 class facts → method reader → P1 disassembly → P2
execution model → P3 SSA → P4 value/use interpretation → P5 storage/transfer
plans → P6 control/protection/monitor plans → P7 owned method expressions,
followed by P8 class assembly/type-name planning and P9 text/JSON with UTF-8 source spans. `Engine::analyze_method` and class requests use one method task. The
[CLI](../../apps/diana/README.md) delivers the same owned P9 publication, without
another renderer or analysis route.
The approved architecture is [PIPELINE_DESIGN](../../docs/diana/PIPELINE_DESIGN.md).

P1 retains complete instruction/payload facts, original references, entry layout
and structural associations. P2 owns ordered operations, exact slots, explicit
normal/exception frame/heap/lock completion rules, transfers, exits and coverage.
It does not expose its complete P1 predecessor. Read-only `MethodEvidence` supports
original inspection and checks without product extraction or construction access.
P2–P7 constructors are internal and require Engine's method-task authority.
P3 privately retains P2 and delivers exact-slot states, definitions, ordered use
bindings, necessary Phis, per-transfer incoming states and original StatePoint
mappings. Borrowed `ExecutionFacts` exposes normal execution queries, not a
predecessor product. [P3's contract](../../docs/diana/spec/03-ssa.md) describes
exception-aware LT dominators, live-demand IDF placement and test-only correspondence checks.
Undefined/invalidated sources remain explicit availability facts, not default values.

A P2 stop retains the complete P1 input, classified error and available unvalidated
construction facts. A P3 stop retains complete P2 input and a partial SSA
snapshot; neither stop can be promoted to a successful product. P4 construction
stops likewise retain SSA and a distinct unpublished `TypePartial`. P5 stops retain
P4 and an unpublished `StoragePartial`. P6 stops retain P5 and an unpublished
`RegionPartial`. P7 stops retain P6 and an unpublished `ExpressionPartial`;
these snapshots are not mature successors.
Class requests
reserve result storage before starting method work and retain all selected known
member outcomes. A method-task resource/invariant error no longer stops siblings;
`ClassAnalysis.terminated` and its kind-based truncation path are removed.
Range/capacity checks and fallible allocation remain mandatory. The CLI/host owns process termination; Engine has no request
cancellation callback/token or checkpoint parameter. There are no numeric quotas.

The [P1–P9 local-result contract](../../docs/diana/PIPELINE_DESIGN.md#92-p1p9-uniformly-consume-local-results)
requires each stage to preserve independent results within the same method.
P1–P3 now continue through local reference-read/use, register-span, invoke-grouping,
result-use and return-kind issues. Target declarations, grouping and return category
keep separate states; invoke binding reuses P1's resolved prototype rather than
reading declarations again. Unresolved inputs are not zero arguments, and unknown
results create no ValueId. SSA preserves exact-slot scope, exceptional old state,
unknown Phi incoming and recovery after an established overwrite.

This does not establish the entire target: P1 still requires reliable instruction
boundaries, entry layout and control/protection associations. An unreliable graph
stops with original facts instead of producing SSA by dropping edges.

P4 consumes existing SSA/execution facts and retains P3 inside `TypedMethod`.
It delivers value families/type sources, per-use literal/reference/integral views,
Phi incoming adaptations and position-related constructor states. P0's class
declaration now travels inside the method products as one shared immutable
`OwnerFacts` handle, so `this` carries its declared owner type and constructor
calls on `this` distinguish `This`, `Super`, proven same-DEX `Ancestor` and
`OtherOwner`; unresolved owner relationships retain their concrete declaration gap. Typed-catch descriptor
results now travel from the method reader through execution facts, including local
read/use issues. Array fills consume checked payload width/count/bits and select
an element interpretation from existing SSA array sources. Ordinary reference
inputs now use `Direct(Reference)` without proving a class hierarchy. Original
source/use declarations and invocation prototypes stay separate; invocation
identity is not selected from argument types. Machine reads are established before
optional value-preserving declared views. Insufficient range evidence retains the
original read rather than making it unavailable; use declarations do not add
conversions or establish boolean naturalization. Runtime Phi representation now
consumes available literal-input constraints before joining nonliteral groups;
source literals keep their independent views. Initialization follows original SSA
reference bindings and execution positions, including edge-correlated paired
observations; Multiple identity is not an initialization failure by itself. No
general input-context query is implemented. A restricted same-DEX superclass
query now follows original declaration facts without exposing an index or stage
constructor to P4. Successful check-cast bounds now refine array
component knowledge at original execution positions without an SSA write or global
source-declaration rewrite. Actual object-relation uncertainty and unsupported
interpretations remain separate from missing input facts.
The [P4 chapter](../../docs/diana/spec/04-types.md) records its algorithm and guarantees.

P5 privately retains P4 in `StorageMethod`; P6 privately retains that product in
`PlannedMethod`. P7 consumes this mature plan in the same method task.
It initially maps each SSA value to a parameter, independent local, exact literal
or explicit reference carrier, preserving P4 nonliteral numeric bit storage.
Numeric constants with consistent delivered views can materialize at their
original definitions when normal-copy liveness permits sharing canonical primitive
storage; independent or mixed-view constants keep their literal form. No P4 type
fact or original execution position changes. Before copy scheduling, compatible Move/normal-copy locals can
share storage when exception-aware liveness establishes no interference. Original
value/state identities and Local representative mappings remain inspectable;
parameters, snapshots and construction-related carriers remain independent.
Definitions keep their original normal commit/materialization sites. Every Phi
incoming retains its original transfer and adaptation: normal copies execute only
on the selected transfer; exceptional incoming uses before-throw saves and
handler-only receives after the original exception capture. Dependency-indexed
copy scheduling preserves simultaneous-copy requirements and uses matching
temporaries to break cycles. Local gaps follow affected storage dependencies;
independent producers and plan components remain available. The
[P5 chapter](../../docs/diana/spec/05-variables.md) defines its active interface.
Construction plans now select natural groups for established allocation/constructor
correspondence, including private alias publication and compatible snapshot
preludes; other relationships retain explicit operations and concrete reasons.
P6 delivers Block/Sequence/If/Loop/original Switch/LabelledBlock/Try/Synchronized
and explicit Graph regions through the same method chain. Every original
operation/commit and terminal has its own whole portion before planning; original
blocks, edges, PCs and member identities remain unchanged. Synchronized plans
separate held execution from individual release outlets and retain cleanup control,
handler receives and before/after-release P5 obligations. Closed scopes with
distinct unlocked targets keep those explicit outlet gotos and no fabricated
shared follow. Executable path-specific post-release copies still retain explicit
monitors rather than moving assignments under the lock. Normal LT dominators,
finite-exit postdominator candidates and iterative SCC analysis drive composition.
Private handler rejoins can establish a closed loop entry without replacing
normal dominance with an exceptional-edge relation. Loop recovery uses only
while, do-while and while(true), as required by the
[approved contract](../../docs/diana/PIPELINE_DESIGN.md#563-loop-expression-while-and-do-while).
There is no for/enhanced-for reconstruction or dedicated update product.
Initialization and updates remain at their original body positions. A shared
update is an ordinary target, not a new continue destination: copies, cleanup
and release precede it only on the original paths, and update-skipping exits
remain distinct. A whole-loop catch-all can use the actual predicate follow
before try admission, including protected private return/throw corridors. Closed
abrupt handler ownership permits those corridors without acquiring recovering
handlers, shared entrances or unguarded cleanup. Predicate work stays at its
original position even when it cannot fit a compact header.
Labels and selected edge copies remain concrete. Original switch links can share
one case body, with acyclic private case-entry joins ordered once. Several
predecessors can share one case entry: only the adjacent one falls through;
others retain original gotos to its label. Entry copies that still require
selected-only execution, case-entry cycles and unresolved sharing retain
explicit layout. A unique shared non-case suffix has one outer owner, including
its downstream bypass entries. Private case bodies can retain whole loops and
established handlers; direct dispatch to an external suffix still executes only
that selector's original copies before its actual completion. Protected prefix completion keeps every guarded seed and the
latest established single-entry frontier, so an optional predecessor cannot erase
an available inner catch. A genuine multi-entry core stays explicit. Ordered
throw-site chains,
original captures, P5 construction ownership and explicit monitor separation reasons
survive failed natural composition. A printed catch or cleanup structure also covers
its original clause set for display, so nested typed catches, several ordered
catches, one union clause over adjacent same-handler types and the remaining
method-level propagation no longer repeat that shape as a chain line and inline
markers. Reordered clauses, a printed position naming fewer types than its
original clauses, use issues and unplaced routes keep their markers. One private completion interface supplies
normal and executable exceptional entrances/outlets, including uncaught exits,
with original copy/snapshot references. A guarded core with only throw completion
has no normal follow into its catch; independent normal outlets keep their roles. The [P6 chapter](../../docs/diana/spec/06-regions.md)
defines the boundary and scope rules. A single unlocked catch-all cleanup can
print as `finally` when its cleanup graph matches the normal exit. Matching covers
conditional branches, arithmetic, field accesses and calls, including private
result pairs established from P5 definitions and reads. Explicit cleanup returns
and throws replace pending exits; always-abrupt cleanup has no artificial follow.
Complete selected natural construction groups can match without redoing P5
analysis, while unequal returns/arguments and construction snapshots stay explicit. Both paths retain their original
effect order and require no copy execution inside cleanup; original body-outlet
copies remain before it. Pure normal preparation can complete the original body
up to the cleanup cursor's first admitted event; only a complete match commits
that preparation. Literal cleanup arguments stay in cleanup. A switch bypass can
break its selected Try and execute finally once rather than jump to a hidden
normal duplicate. Foreign entrances and failed correspondence keep explicit code,
without searching another start. Shared handler continuations retain one external owner; their private entries
can print as natural catches with original copies and explicit outward transfers.
Cross-handler finally folding, unmatched Phi values and different cleanup exits
retain the original catch or explicit handlers.

A selected monitor can use natural implicit release when P6 establishes one
normal outlet and private exact capture/rethrow cleanup with equal natural
delivery and no release-phase work. Typed recovery remains inside the monitor;
extra cleanup, different locks, replaced exceptions and multiple normal outlets
keep explicit events. Private captures can bind in the catch header while
original operations, sites and receiving phases remain inspectable.

P7 produces an independently owned `MethodResult`, delivered by
`MethodOutcome::Expressed(ExpressedMethod)`. `inspection()` borrows earlier facts
from that same execution; `into_result()` releases retained analysis. No-code
members use `Declared(MethodResult)`. Unified operation lowering retains original
calls, conversions, checks, copies, exception sites and initialization positions.
P5 natural constructions become `New` expressions with complete ordered
argument trees, including calls, arithmetic and field reads under equal handler
protection. Different argument/effect order, shared producers, differing handlers
and changing exception snapshots retain explicit allocation/initialization.
Protected constructions can compose into an adjacent throw with that same
handler chain and spelling. P6 controls/outlets have
explicit emission ownership. P7 also attaches local declarations to their first
assignment when later reads and writes remain in scope; joins, copy destinations
and unknown placements retain outer declarations. The P7 compose passes select only adjacent
expressions and compact control forms admitted by their delivered facts; original
operation and control origins remain in the final source spans. A shared private
adjacency view of selected sibling Blocks preserves expressions, check carriers
and exact array literals across operation portions. Checks cannot move across a
call, early selection, copy phase or protection change merely because a later
read names the same value. An initializer carrier keeps the check's own PC and
operation, not its preceding call's identity. Planned actions
after an already printed unconditional exit can be omitted through linear
Sequence tails, without crossing the exit's scope or an opaque protected/loop
container. Original transfers and inspection continuations remain available.
A complete linear return/throw arm can print as a guard; either original edge
keeps its exact condition polarity. The other edge's copies/body follow once,
in their enclosing declaration scope. Recovering or opaque scopes do not supply
that exit fact, and a guard plus later work is not mistaken for one else-if.
Ordered conditions also retain P6-owned empty selected goto routes. Copy-free,
effect-free paths can print as one short-circuit condition; copied, cyclic or
foreign-entry paths remain explicit. A noncomposable test prelude retains the
delivered complete explicit schedule and original routing origins.
Metadata and
bootstrap declarations travel from their owning producers; P7 does not reopen
input. See the [P7 chapter](../../docs/diana/spec/07-java.md).
Known StringConcatFactory call sites can print an explicitly inferred `+`
presentation while retaining the original Custom invocation and complete
bootstrap facts. P4 argument views, captured effects and conversion positions
remain; unsupported references, recipes or constant conversions keep their
original call with a local presentation-unavailable reason. Recipe tags and
static String/integral/boolean/char constants are interpreted from owned facts.
The presentation is not proof of runtime linkage or library behavior.

A closed single-suspension SuspendLambda entry can now select a Kotlin-style
suspend body: its initial work, suspended result and original common tail form
one sequence. The declaration retains its original format and adds only a
`suspend` modifier. Original ABI entry/reentry, resume checks/copies, state publication
and invalid-state errors remain readable attachments with source mappings. The
callee binding and continuation identity stay explicit, and suspension is marked
as an inference. ABI parameters and wrapper members remain; original source-lambda
signatures, multi-suspension spills and source exception recovery are not established
by this view. Missing prerequisites retain the original method layout.

Ordinary suspend methods with one closed head-tested loop and one resume state
also have a Kotlin-style body view. It restores `while`, suppresses the matched
continuation argument at the call, and forwards proven loop-local copies. The
Java-style signature gains `suspend`; boxing, casts, original names, ABI
entry/reentry, state publication and error paths remain explicit. The
[captured-effect challenge](../../quality/fixtures/kotlin/suspend-loop-captured-effects/README.md)
exercises another mutable value and effects around suspension. Mismatched spills,
changed captures, and resumed-only effects retain their original explicit code.
Ordinary methods with matched multiple suspension sites now select a source body
over existing P6 controls, including early returns, saved scalars and one closed
head-tested cycle. A cyclic flow keeps the original head predicate, exit copies
and interior resume corridors; it restores `while` only when one component, one
latch and one initial-header edge are established.
Original fetch and the
[three-step challenge](../../quality/fixtures/kotlin/suspend-three-step-effects/README.md)
exercise private/shared marker returns and distinct result/capture merges.
Normal-only effects, missing spills, nested or multi-latch cycles, do-while
shapes and protected coroutine source reconstruction remain explicit rather
than using this view.
The [multi-site branch](../../quality/fixtures/kotlin/suspend-two-step-branch/README.md)
and [two-site loop](../../quality/fixtures/kotlin/suspend-loop-two-sites/README.md)
inputs keep original call/capture origins, state publication and resume copies.
The [two-site checksum](../../quality/fixtures/kotlin/suspend-loop-two-site-checksum/README.md)
challenge adds a mutable checksum with pre- and post-update effects across both
sites. The
[protected two-site](../../quality/fixtures/kotlin/suspend-finally-two-sites/README.md)
input retains explicit state dispatch and original protection facts.
An [effectful-default challenge](../../quality/fixtures/kotlin/suspend-default-shared-capture/README.md)
also preserves a shared seed across both sites. Their ordinary Engine checks
cover available members, original call/capture origins and dispatch spelling;
native helper effects and scheduling remain modeled, not executed.

P8 produces owned `ClassResult`/`TypeNamePlan` from the original directory and
joined method deliveries. Success, no-code and stopped members remain distinct.
The same execution retains inspection while sharing immutable owned P7 results.
Static preparation and annotation expressions preserve original event order and
reference identities. Names handle imports, conflicting short names, lexical
shadowing, array ranks and original `$` spelling; standalone methods remain fully
qualified. Erasure-matched signatures deliver class/method type parameters,
single bounds, scoped variable uses and superclass/interface arguments. Unknown
package scope and uninterpreted signatures retain explicit naming reasons.
See the [P8 chapter](../../docs/diana/spec/08-class.md).
P9 consumes that frozen display through Engine's private `RenderTask`, publishing
owned `Rendered` text, UTF-8 spans, local diagnostics and the same source product.
Text and JSON use that one publication. P7 fixes final child/phase placement,
including loop-condition edge copies and original-node synchronized outlets;
P9 does not reconstruct control. Explicit storage/check/lock notation preserves
semantics without requiring legal Java. Supplied lossless byte/short/char use
views now remain visible on word carriers as `(value via Type)`; exact char
literals keep their escaped UTF-16 unit. Other int uses, storage/copy phases and
unproved views are not rewritten. See the [P9 chapter](../../docs/diana/spec/09-rendering.md).

Independent translation checking compiles only under `cfg(test)`; ordinary
Engine requests do not run symbolic replay, output audits or duplicate dominator
verification. There is no runtime or feature switch. Parsing and construction
still enforce input safety, checked arithmetic and allocation failures.

`Engine::with_outline()` optionally retains line ranges, lexical depths, header
identities and original call facts during the same P9 traversal. `Rendered` and
`RenderStop` expose this owned presentation through `outline()`. Normal text/JSON
retrieval keeps its existing publication without allocating outline facts. The
CLI's `outline-class` and `outline-method` consume these facts for finite call
navigation; P9 does not start callee analysis or recover structure from text.
The development shell accepts `"outline":true` on a `p9` request to inspect the
same product. See the [P9 outline contract](../../docs/diana/spec/09-rendering.md#outline-presentation).

Integration checks live in `tests/` and consume prepared inputs from
`quality/fixtures/` through the ordinary Engine entry. Run
`cargo nextest run --locked -p diana-decomp`. Thirteen
[scalar-use and preservation checks](tests/scalar_use_views.rs) cover
[carrier/literal views](../../quality/fixtures/atoms/scalar-use-views/README.md),
[protected joins/composition](../../quality/fixtures/exceptions/scalar-views-protected-joins/README.md),
[captureless retry](../../quality/fixtures/exceptions/handler-retry-rejoin/README.md)
and [response/resource cleanup](../../quality/fixtures/exceptions/response-close-snapshots/README.md).
Those preservation checks are complemented by nine
[exception-rejoin checks](tests/exception_rejoins.rs). They cover natural
captureless retries, nested/latch/foreign arrivals,
[dual cleanup outlets](../../quality/fixtures/exceptions/cleanup-outlet-frontiers/README.md)
and [shared cleanup contexts](../../quality/fixtures/exceptions/shared-cleanup-contexts/README.md).
P6 owns the recovery and retains snapshots, original catches and close/suppression
order. Four [exception-only completion checks](tests/exception_only_completion.rs)
cover protected construction, typed/unmatched exits,
[nested cleanup and a foreign entrance](../../quality/fixtures/exceptions/exception-only-completion/README.md).
Five [natural catch display checks](tests/natural_exception_display.rs) cover
[loop/rethrow/foreign/multi shapes](../../quality/fixtures/exceptions/natural-typed-catch-display/README.md)
and the [nested/union challenge](../../quality/fixtures/exceptions/nested-catch-display/README.md):
redundant markers disappear while sites, chains and explicit protection remain.
These product checks do not execute fixture behavior.
Six structural checks cover
[nested private loop exits](../../quality/fixtures/loops/nested-loop-exits/README.md).
Four [resource-cleanup checks](../../quality/fixtures/exceptions/resource-cleanup/README.md)
cover loop-private handlers, nested protection, propagation-only sites and shared
handler separation. Three [catch-group checks](../../quality/fixtures/exceptions/typed-catch-loop-exits/README.md)
cover adjacent typed aliases, inherited cleanup ownership and a nonadjacent-alias
priority guard. Three [loop-entry checks](../../quality/fixtures/exceptions/nested-catch-rejoins/README.md)
cover private catch rejoins, nested loops under a monitor and a genuine pre-loop
exceptional arrival. Five [finally-scope checks](../../quality/fixtures/exceptions/branching-finally-suffix/README.md)
cover suffix cleanup, original body copies, loop completion, unequal arguments
and preservation of a pending rethrow's outside dispatch. Five
[iteration-tail checks](../../quality/fixtures/loops/iteration-tail-cleanup/README.md)
cover original update anchors, nested continuations, pre-test evaluation, an
update-skipping backedge and specialized monitor routing. Four
[monitor-continuation checks](../../quality/fixtures/sync/monitor-iteration-exits/README.md)
cover held cleanup, direct release delivery, mixed exits and a released
update-skipping backedge. Four
[split-outlet checks](../../quality/fixtures/sync/monitor-split-tails/README.md)
cover nested captures with separate unlocked tails, a live-parameter Phi copy
barrier, mixed loop exits and preservation of the direct header continuation.
Four [completed-tail checks](../../quality/fixtures/branches/abrupt-continuation-tails/README.md)
cover original case breaks, opaque switch/protection work and a retained release
continuation when no original jump prints. Nine [switch checks](../../quality/fixtures/branches/switch-fallthrough/README.md)
cover grouped entries, fallthrough, original constant placement, conditional
breaks, wide values and explicit-layout guards. Four
[case-entry checks](../../quality/fixtures/branches/case-entry-joins/README.md)
cover three predecessors of one body, nonadjacent original gotos, executable
selector-copy separation, case cycles and non-case sharing. Four
[protected-prefix checks](../../quality/fixtures/exceptions/protected-prefix-frontiers/README.md)
cover a retained guarded entry, a branched common prefix, foreign throwing
predecessors, real multi-entry refusal and complete outer coverage.
Four [variable-update checks](../../quality/fixtures/loops/mutable-iteration-step/README.md)
cover an original parameter read, body-mutated step through cleanup, no-body
nonlocal condition exits and throwing-update refusal.
Three [quiet-copy checks](../../quality/fixtures/branches/quiet-condition-copies/README.md)
distinguish origin-only condition copies from executable call-result/parameter Phi
assignments, preserving one original anchor without an empty else.
Four [pure-tail checks](../../quality/fixtures/exceptions/pure-tail-finally/README.md)
cover branched preparation, actual parameter Phi copies, foreign entrances,
switch bypass completion, literal cleanup prefixes and reachable second-cleanup
faults. Six [abrupt-finally checks](../../quality/fixtures/exceptions/abrupt-finally-completion/README.md)
cover return/throw replacement, saved body-return state, always-abrupt and bare
void cleanup, two original rethrows, exact parameter throws and unequal/foreign
entry refusals. These are product/origin regressions, not full-method behavioral
execution. Six [constructor-argument checks](../../quality/fixtures/atoms/constructor-argument-chains/README.md)
and their [effect challenge](../../quality/fixtures/atoms/constructor-argument-effects/README.md)
cover two independent field/call chains, protected exception construction,
branching argument trees, fault short-circuiting, original anchors and
order/sharing/handler/snapshot refusal. Their independent numeric evaluator
executes final argument trees only, not whole methods. Six [monitor-surface checks](../../quality/fixtures/sync/implicit-monitor-cleanup/README.md)
and their [boundary challenge](../../quality/fixtures/sync/monitor-surface-boundaries/README.md)
cover implicit release, header capture, nested/reentrant locks, typed recovery,
extra cleanup, changed errors, split returns and escaping capture storage.
Their test-only final-plan model executes the documented fixture subset, not a
general JVM. Four [carrier checks](../../quality/fixtures/atoms/refined-carriers/README.md)
retain exact check provenance and prevent delaying a check past a call or fusing
an array fill across another reference check.
Four [while-continuation checks](../../quality/fixtures/loops/while-tail-continuations/README.md)
cover three-level private outer exits, actual shared entrances, cleanup before
visible updates and do-while first-body placement. Dedicated for/update products
are absent; existing cases now inspect original body positions and targets.
Nine [terminal-guard checks](../../quality/fixtures/branches/terminal-guard-arms/README.md)
cover original predicate direction, actual other-edge Phi assignments after the
guard, parent declaration scope, recovering-Try refusal, cleanup, construction
and early void return. Missing natural protection still retains explicit code.
Six [shared-tail checks](../../quality/fixtures/branches/switch-shared-tails/README.md)
cover nested case loops/recovery, two-level suffix bypasses, actual selector
assignments, foreign case arrivals and original ABI marker/default exits.
An empty conditional child with executable edge copies retains its else phase;
child emptiness alone cannot qualify a copy-only branch as silent.
Five [protected-loop checks](../../quality/fixtures/exceptions/protected-loop-exits/README.md)
cover nested protected throw corridors, real returns that skip cleanup, foreign
normal entrances, recovering handlers and explicit raw-selector destinations.
A raw selector's next statement is not its next case label; its supplied gotos
survive display simplification.
Eight [selected-route checks](../../quality/fixtures/branches/short-circuit-shared-routes/README.md)
cover boolean disjunctions, ordered effectful tests, original route anchors,
executable copies, effect barriers, foreign entries, cycles and complete explicit
preludes, including floating NaN comparisons.
These checks cover ownership,
transfer spelling and source anchors, not independent execution of the final plan.
Six [concat checks](../../quality/fixtures/composite/concat-capture-recipes/README.md)
cover original bindings and anchors, independent parameter/constant content
scenarios, char/int and null handling, captured helper order, loop/catch and
explicit unavailable cases. The small content evaluator covers projections,
not complete methods, arbitrary callees or VM linkage.
The retained independent verifier code is not enabled on ordinary requests. Real-input corpus runs have not encountered custom/polymorphic
invokes or method-handle/type constants.

A [Verus trial](proofs/README.md) proves the wide-copy patch predicate used by
test replay, using the same Rust function body. It is absent from production. The adapter and remaining
P2 test checks are not machine-proved; this is not a proof of whole-method correctness.
Simulated P2 capacity/invariant and P3 capacity failures have been observed through
class requests with an executable sibling continuing. These are typed-failure
observations, not actual allocator-failure injection; the latter remains unperformed.

Use the single engine inspection entry from the repository root:

```sh
cargo run -p diana-decomp --release --example dbg_shell --locked -- \
  --dex <classes.dex> --exec '{"cmd":"p2","class":0,"method":0,"details":true}'
```

`class` is the `class_def` index of that bare DEX image; `method` is the
engine-wide `MethodId`, not an index local to the class. A class product lists its
own members as `MethodId(n)`; a class-local ordinal selects a different method and
can report a misleading malformed-DEX or foreign-method-identity error.

`p3` (`details:true`) observes state positions, bindings, definitions and Phi
incoming records. `p3-all` reports SSA counts, unavailable uses, upstream failures
and per-method construction failures. Optional `class` selects one original class
through `Engine::analyze`; `analysis_ns` measures that analysis, excluding index
opening, JSON formatting and inactive rendering. `partial_methods`, `local_issues`
and unresolved input counts distinguish ready products from fully established
facts; batch `complete` describes traversal, not semantic verification. P3–P7 are not
formally verified. `p4` (`details:true`) exposes type origins, bits, requirements,
views and initialization paths. `p4-all` adds typed/partial counts, interpretation
problems, array-fill interpretation counts and missing-fact occurrences to the
same traversal. Array-access counts and component gaps remain separate from
unreadable use views. Detailed P4 output includes projected component sources and
gaps, catch facts and array-fill interpretations with original payload bits. These are diagnostic
counts, not semantic verification badges. `p5` (`details:true`) exposes bindings,
original reads/definitions, copy assignments/steps, snapshot routes, constructor
receiver accesses and local gaps. `p5-all` aggregates the same facts, including
unavailable plan components. `p6` (`details:true`) exposes the region arena,
continuations, labels, original links and copy delivery, handler/site ownership,
lock events, release outlets, copy emission ownership and selected operation roles.
`p6-all` aggregates those plans and
construction stops. `p7` (alias `expressions`) and `p7-all` expose owned
expression counts, detailed products and classified expression stops. The timed
Engine traversal includes P9, even when a command
inspects a retained earlier product. Each shell command creates a
fresh Engine. `Engine::with_method_jobs(n)` enables a bounded Rayon pool for
multi-member class requests when `n > 1`; the default is one. Single-method calls
keep the same sequential chain. The shell exposes this setting as
`--method-jobs <workers>`. Separate DEX-process concurrency and method-worker
concurrency need joint configuration rather than multiplying both blindly.

`Engine::open` numbers images by vector position. A caller that already resolved a
class can lend only the defining image through `Engine::focus(image, dex)`, which
keeps the original `DexId`. Published class keys, image facts and locators then
still name the image that defined the class, and an application that resolved a
name across a multi-DEX input does not need to hand every image to the engine.
Both constructors store one image per id and serve the same `Request` chain.
`p5-all` with `details:true` also exposes each class member's retained P3/P4/P5
products and the P6 plan from that one class request. `p7-all` with
`details:true` exposes each member's owned method result. `p8` (aliases `assembly`
and `names`) and `p8-all` expose class/member delivery, metadata expressions and
name plans; adding `method` selects standalone naming. `p9` (alias `source`) and
`p9-all` expose final text, byte spans and rendering issues; `details:true` also
includes the frozen source product. `--format text` prints the existing P9 text
rather than a differently reconstructed body.

Earlier engine format/check/Clippy and release builds passed for unified component
projection, scoped check bounds and same-DEX ancestor relationships; those changes
added, modified and ran no tests.
The focused observation covered 1,112 methods associated with component gaps or
constructor diagnostics. All 2,224 retained P2/P3 products matched. P4 value/use,
Phi, type-name, array-access, fill and initialization-use products matched.
Constructor relationships and diagnostics changed in exactly the 11 affected
methods; no other selected product changed. Component precision now
comes from original declarations and successful checks at the actual access point,
not a fake cast output or consumer-desired type.

Machine reads remain independent of optional numeric views. Absent int/long ranges
and full-int ranges at boolean-declared uses are not new range evidence; a Z
declaration alone does not authorize boolean naturalization. Literal views retain
their original bits. No runtime conversion or new arithmetic rule was added.

Full traversal of the two APKs in the working directory covered all 55 DEX entries
(41 and 14) and delivered 2,644,578 TypedMethod products. Construction/upstream/class
stops, ordinary unknown reads and representation conflicts were zero. Ten
same-DEX constructor-owner relationships are now Ancestor. The one remaining
recovery diagnostic is Missing(InitializationOwnerRelation), with a concrete
UnknownOwner(MissingDefinition) constructor result. The 3,142 formerly incomplete
component positions now have established bounds; all 523,479 observed array
accesses have complete component knowledge. This includes Copy/Phi and nested
array dependencies, not just direct checked receivers. Null-only alternatives
supply no element type and do not import a different checked array's component
into a non-null path. Complete bounds do not prove non-nullness, an exact runtime
component class or permission to remove the original checks.

The same-DEX constructor-owner query reuses the current declaration and follows
only necessary local ancestor headers. It preserves the original calls to Object
rather than replacing them with an intermediate parent's constructor. A target
reference can establish ancestry without a target definition. The remaining case
requires a parent definition from another DEX; local absence stays Unknown, not a
mismatch, and the query does not search other images. This is declaration-based
initialization evidence, not ART execution or admission evidence. The
[pinned ART source](../../docs/diana/SOURCE_MAP.md#84-additional-mature-projects-pinned-commits-specific-symbols-and-what-not-to-borrow)
records the runtime-source review's separate scope.

Same-DEX ancestor-query timing compared with ae25282 through the same shell:
three warmups and 12 alternating before/after processes for each of six selected
classes. Each process issued 13 class requests with warmed input files. The shell
recreates Engine for every command, so every request had fresh Engine/declaration
caches. First-command median ratios were 1.011–1.053, or approximately 0.5–10.1
microseconds higher; last-ten-command median ratios were 0.904–1.072. Those later
commands measure a warm process, not declaration-cache hits. These observations cover
three ordinary classes, two local-ancestor classes and one missing-local-parent
class. Median process peak RSS did not increase in these samples. They establish
neither universal speed/memory bounds nor a general performance improvement.

The focused single-DEX runs retained all 279 fill interpretations and checked
1,444,571 byte-roundtrip items. The full APK traversal produced 10,212 fill views
without unknown element interpretations, including 28 Short and 1,334 Double
views. Full byte roundtrip checked 48,102,163 items with zero mismatches, skipped
items or method/class errors. These finite observations are not correctness rates
or ART admission evidence. Aggregate counters do not establish branch-specific
coverage of source-unavailable, representation-conflict, nonzero-reference-literal
or out-of-domain literal-view rejection.

Post-Phi construction with multi-origin receivers, opposite allocation selections,
constructor failure/repetition uses, retained older loop allocation instances and
unreadable-direct-call events also lack execution evidence from these runs.
Malformed catch declarations and conflicting/width-mismatched fills retain the
same evidence limitation. Unavailable array sources, ungrounded
component cycles, numeric wide-bit Phi storage and mixed numeric/reference
representation rejection also have static review rather than new execution
evidence from these inputs. The local missing-definition ancestor case was
observed; malformed/ambiguous/cyclic local chains and declaration-lookup allocation
failures were not exercised by this corpus.

Signature-qualified Java-like calls (`Api.register<Listener>(h)`, with a complete
original descriptor when needed) are specified under
[the P7 contract](../../docs/diana/PIPELINE_DESIGN.md#5731-original-target-signature-qualification).
They introduce no operand conversion. P7 now selects the display and P9 prints
its complete raw-signature binding form when selected.

`p2-all` observes retained execution facts from the same Engine chain and reports
classified outcomes and encountered opcode variants. `roundtrip-all` checks independent byte re-encoding over retained
P1 evidence. These inspection commands do not run the test-only correspondence checks.
No phase-specific debug binary is required.

## Value identity and storage normalization

P5 definitions deliver the original optional `ValueId` independently of their
storage representative. P7 builds one private value-use index over delivered
reads, definitions and copy obligations. Expression, array-payload and check
composition match the original value and state of the exact consuming read, so a
representative that stored several values no longer refuses composition.
Composition records whether a result retains storage, and declaration placement
selects storage visibility once from the final materializations, including
grouped construction arguments and the reads of composed identity Moves.
No second SSA, alias, dominance or availability analysis is added, and source
only attachments remain distinct from conditional identity Moves.

The [value fixture](../../quality/fixtures/atoms/value-occurrences/README.md) and
[effect challenge](../../quality/fixtures/atoms/value-occurrence-effects/README.md)
record independent expectations and LLM path reviews. The `value_occurrences`
integration target passes all 11 checks; on f83f2fa the same file passes 5 and
fails 6. That includes a test-only linear integer executor covering result and
order effects for specified scenarios, plus an extra-effect negative example. It
does not execute arrays, casts, catches, arbitrary callees or a JVM.

On rustc 1.98.0 the default and all-feature package nextest runs each execute
195 tests with 195 passing. Engine Clippy with all targets and features,
formatting and the release engine-example and CLI builds pass.

Before/after inspection covers 121 prepared fixtures, 189 classes and 917
methods through ordinary Engine requests. Original P2–P4 products, original P6
coordinates and copy/exception facts agree; the newly delivered P5 definition
value field is an intentional addition. Two fixtures change text: the new value
fixtures. Traversal reports no construction or rendering failures, which does not
establish whole-method behavioral correctness.

## Shared control delivery

P7 now normalizes established redundant checks before covering expressions.
Required casts participate as readonly operation roots in the common cover and
retain their value/state and original exception site. They are no longer skipped
as transparent condition preparation. The cast-preparation-order input detects
an earlier exception-order error and checks both source orders plus a produced
reference. Two new cases execute 29 paths; the scoped thirteen-test run and
engine Clippy pass. Full historical regression remains deferred.

P7 cast queries and loop-read collection now share one private expression walk.
The safe-read query and occurrence count consume the same syntax grammar and
preserve nested carrier boundaries. New cast-carrier-order/contexts inputs recover
the right comparison operand, a later call argument and a first short-circuit
test without moving checks past selections, helper effects or catch boundaries.
Six new cases execute 41 reference scenarios; the joint-expression checks reuse
the same independent test executor. The scoped eleven-test run and engine Clippy
pass. Historical fixtures remain deferred during the structural rewrite.

P7 stores selected nested Evaluation edges in the expression arena and each
original operation names its unique nested_at occurrence. P9 obtains statement,
PC and result-commit origins from that original operation. The separate composed
use/result tables are removed. Condition candidates use an immutable sparse
proposal view, so refused candidates require neither arena copies nor rollback.
The new joint-expression-cover input has four passing ownership/origin checks;
historical fixture adaptation and regression are deferred during the structural
rewrite at the operator's request. These checks do not execute helper behavior.

Ordinary expression consumers and short-circuit tests now share an ordered
suffix-cover matcher. Several argument roots enter one forest, which retains
nested preparation and exact value/state reads. An unmatched effect or reversed
operand stops the suffix; skipped test runs require complete prelude coverage.
The new tree/barrier challenges have five passing nextest cases, including
scalar selected-plan results and every reachable modeled helper-fault prefix.
The expression module has 133 fewer production lines than `c2e1ac5`, including
the sparse pending-ownership index and active-root suffix. Full historical
regression remains deferred during the rewrite.

P6 now publishes the common `ControlLayout` with named roles and original
predicate/transfer coordinates. P7 consumes that structure directly and keeps
one `Control` arena for selected execution, presentation and retained source
facts. P8/P9 traverse `MethodCode::controls()`; the separate `EmissionControl`
arena and P7 loop/layout reconstruction are removed. Immutable role lists share
owned storage, with fallible element allocation when a selected list changes.
The `dbg_shell` detailed P6 response exposes these delivered controls.

The default and all-feature engine nextest runs each pass 188 tests. Engine
Clippy with all targets/features and the release `dbg_shell` build pass. A fresh
ordinary-Engine comparison against `f6e7893` covers 119 prepared fixtures,
187 classes and 907 methods, retaining text, reported issues and output counts
without construction or rendering failures. This establishes compatibility of
the control-representation change, not new recovery coverage or whole-method
semantic equivalence. General fragment reduction and joint expression coverage
remain subsequent work in the unified recovery design.

## P5 observations

The current authorized-input traversal covered 55 DEX files and delivered
2,644,578 StorageMethod products. It retained 29,138,117 value bindings,
44,074,425 use accesses, 3,085,278 Phi incoming records and 12,407 handler/Phi
snapshot locals. Reported storage gaps, partial copy plans, stage stops and class
errors were zero. The original single P4 initialization-owner uncertainty remained;
it was not converted into a storage failure. The 1,112-method focused comparison
retained identical P2, P3 and P4 products, for 3,336 matching observations.

Copy-cycle saves were not encountered. Source-unavailable and representation-gap
storage propagation also lacks execution evidence from this traversal. The
installed scheduling/dependency algorithms and finite counters do not prove these
branches or whole-method behavioral correspondence. Existing byte-roundtrip
evidence was reused because input and P1 were unchanged; no second full chain was
run merely to repeat that check. These are authorized private-input observations,
not repository-reproducible fixtures or correctness rates.

### Natural construction observations

P4 now publishes original allocation-source sets and position-related receiver
comparisons. P5 consumes these facts to select construction groups without
rebuilding reference provenance. Original sites and commits remain facts; selected
deferred publications happen only after normal initialization.

A 55-DEX P5 traversal delivered the same 2,644,578 code methods with no new stage
stops or storage gaps. It selected 720,360 natural groups and retained 459,253
explicit allocation carriers. Uncertainty in the original initialization-owner
relation stayed unchanged. These observations are not whole-method equivalence or
ART admission evidence.

### Region planning observations

A 55-DEX traversal delivered 2,644,578 PlannedMethod products without P6 stops or
upstream/class stops. The pre-existing P4 issue remained and storage gaps stayed
zero. It selected 28,695 synchronized regions with 64,777 release outlets, including
28,469 existing-exception rethrow outlets. Natural and explicit plans coexist;
incompatible unlocked continuations and unsatisfied post-release copy obligations
retain complete explicit plans. Recovery diagnostics are not execution-fact
failures or whole-method verification badges. All 4,448 retained P2/P3/P4/P5
products for 1,112 focused methods matched. Serial/four-worker products including
P6 matched for 19 classes and 4,585 members. Input/P1 were unchanged, so prior
byte-roundtrip evidence was retained rather than rerun.

### Method expression observations

A fresh fixed-binary traversal of all 55 DEX entries delivered 2,644,578 expressed
methods and 263,414 declaration-only results, with no P7, upstream or class stops.
Storage issues stayed zero and the one existing type issue remained. The result
contains 720,360 natural constructions and 459,253 explicit allocation operations.
Retained P2–P6 products matched for 1,112 focused methods; serial/four-worker P7
results matched for 19 classes and 4,585 members. These counts describe observed
coverage, not universal semantic acceptance. All 48,102,163 instruction roundtrips
were rerun across those 55 DEX entries with zero mismatch, skip or method/class
error. Earlier synthetic DEX 038/039 observations covered the changed dynamic facts;
the removed core test is no longer present. The real-input corpus does not supply
those opcodes. Engine fmt/check/Clippy and release library/shell builds passed.

### Class assembly and naming observations

A fresh fixed-binary 55-DEX traversal produced 544,430 class results, preserving
1,472,798 fields and 2,907,992 methods, including 263,414 no-code declarations.
P8/upstream/class stops and retention errors were zero; the existing one type
issue remained. It retained 369,299 static preparations and 1,299,169 annotations.
All 5,560 P2–P6 products for 1,112 focused methods matched. P7 code and headers
excluding intentionally expanded metadata also matched for those methods.
Serial/four-worker P8 products matched across 19 classes and 4,585 members.
All 48,102,163 instruction roundtrips were rerun with zero mismatch, skip or
method/class error. Actual allocator exhaustion and every stopped-stage prefix
remain outside the exercised execution evidence.

### Rendering observations

After composition and the source-control span change, a fixed release binary
freshly processed one authorized APK's 14 DEX entries with eight DEX processes
and one method worker each. It rendered 118,925 classes and 748,075 methods,
including 90,371 no-code declarations. The traversal reported no rendering issues,
render stops, upstream stops or class errors; the existing one type issue remained.
It emitted 1,439,037,024 UTF-8 bytes and 81,743,055 spans. One inspected method's
four composed operations each retained an operation and statement span, with
their source controls enclosing them; its text matched JSON and all 110 span
endpoints lay on UTF-8 boundaries. These observations are not a proof for other
methods or runtime behavior. No tests were written or run.

Before composition, a fixed release binary freshly processed all 55 DEX files without reused results:
544,430 rendered classes, 1,472,798 fields and 2,907,992 methods, including 263,414
no-code declarations. Rendering failures/issues, upstream stops, class errors and
retention errors were zero; the existing one type issue remained. The traversal
produced 7,488,484,265 UTF-8 bytes and 313,026,198 source spans. These counts are not
per-instruction semantic proofs. The unchanged readers reuse the P8 evidence of
48,102,163 instruction roundtrips with no mismatch, skip or method/class error.

All 5,560 retained P2–P6 products for 1,112 focused methods matched. Existing P7
facts also matched after accounting for the new emission, type-source,
parameter-name and exact UTF-16 serialization fields; no call-qualification
change occurred in that selection. Serial/four-worker P9 products matched for
19 classes and 4,585 members. Engine format/check/strict Clippy and release
library/shell builds passed. Workspace strict Clippy and release builds also
passed after CLI integration. Actual allocator exhaustion, all stopped-stage
prefixes and arbitrary VM/heap/callee behavior remain outside this evidence.

## Method scheduling observations

Rayon workers share immutable input/P0 facts and run the same MethodTask, with
private phase products and scratch. The scheduler reserves selected-member result
storage before dispatch, retains classified member failures and joins tasks before
returning. Pool startup errors retain their typed source and are not cached as an
absence. The [scheduling contract](../../docs/diana/phases.md#current-method-scheduling)
defines startup and local-failure scopes.

A serial/four-worker comparison covered 19 existing focused classes and 4,585
members. Detailed retained P3/P4/P5 products matched. Full 55-DEX traversals under
8-process/1-worker, 4-process/2-worker and 2-process/4-worker configurations retained
identical P5 counters and 2,644,578 StorageMethod products, without storage gaps,
stage stops or class errors. Existing P4 uncertainty remained unchanged.

For six selected large classes, three warmup pairs preceded 12 alternating
serial/four-worker pairs. Analysis-time median ratios were 0.341–0.939; process
wall-time ratios were 0.372–0.940. Every command used a fresh Engine/pool and warm
input files. These finite measurements are not universal speedup bounds.
Whole-batch observed wall times were approximately 32.0, 41.2 and 71.0 seconds for
the respective configurations above, without reused P5 results. Thus this batch
retains the faster eight-process/single-method-worker configuration; internal
method parallelism is available for class requests, not forced on every workload.
These were individual batch observations, not a repeated throughput benchmark.
Actual worker-startup failure and allocation-failure execution remain unobserved.

Applicable engine gates:

```sh
cargo fmt --all -- --check
cargo clippy -p diana-decomp --all-targets --all-features --locked -- -D warnings
cargo build -p diana-decomp --release --locked
```

Dependency, reference-isolation and unit-test policies are owned by root/package
AGENTS.md and [CONTRIBUTING](../../docs/diana/CONTRIBUTING.md). The crate remains
unpublished (`publish=false`, version `0.0.0`).
