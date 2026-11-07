# 01 — Input and P1 original facts

Code: `crates/diana-decomp/src/input/` and `src/p1/`. Application input/ZIP
handling lives in `apps/diana/src/analysis/`. Ownership and stage boundaries are
specified by [PIPELINE_DESIGN §§3–5.1](../PIPELINE_DESIGN.md).

## Shared checked reader

The application owns physical input bytes; `DexContainer` and `ProgramIndex`
borrow them. The engine does not open files, ZIPs, processes or networks.
`DexContainer` recognizes little-endian DEX 035, 037, 038, 039, 040 and 041.
Unrecognized versions and reverse-endian input retain unsupported classification.
Recognizing a header does not establish all member records or ART validity.

DEX 035–040 use a logical header at physical byte zero. DEX 041 can contain
several logical headers; original offsets address the physical container.
Header selection MUST NOT rewrite bytes, add the logical header offset to an
already absolute offset or merge equal numeric indexes from different logical
DEX namespaces.

`Cursor` is the shared bounded read implementation. Readers MUST check offset,
count, stride and end arithmetic before slicing or reserving memory. Fixed table
strides and minimum encoded-record sizes are format facts, not numeric quotas.
LEB values MUST retain their format width/sign constraints; malformed encodings
MUST NOT become zeros. A `string_data_item` is read as its declared unit count plus
the payload up to its terminator; the payload bytes are not decoded or validated on
read. Decoding happens on demand ([`Units`]) and replaces an illegal sequence with
U+FFFD instead of failing the item, so a malformed string stays readable and cannot
make a request that merely names it unavailable. The declared unit count is
advisory: equality and prefix matching compare decoded units, not that count.
Case-insensitive substring matching compares decoded UTF-16 units by lowercase
iterator equality; surrogate units compare as units. An all-ASCII payload and
needle use byte candidates. Other values use the same replacement decoder,
without allocating a UTF-16 vector or changing to whole-string case folding.
Declaration syntax checks (name and descriptor shape) are separate from this and
still reject an unusable declaration.

Original DEX declarations and references are the only symbol facts. Missing target
class definitions do not erase a readable reference's owner/name/prototype.
Class location and P0 caches remain local to the selected logical index;
allocation failures MUST NOT be cached as member absence.

`LocalDeclarations` owns the shared checked table, string/type and class-header
reader while borrowing the original bytes. ProgramIndex and the restricted P0
superclass view use that same implementation; no second descriptor or class-record
parser is maintained. The view keeps only this logical header's namespace, and
its local class-location index is initialized on demand. The declaration view
has no method-analysis or stage-construction interface.

The reader memoizes accepted string spans/counts, type-to-string references,
descriptor shapes, member-name checks and resolved prototype facts within each
logical index. These facts depend only on immutable input bytes. Prototype readers
establish shorty correspondence, parameter descriptors and word counts once;
member-list rows borrow those descriptors without per-row allocation. Cache storage
uses fallible reservation bounded by checked table extents. The prototype slot
array is initialized only on a prototype read; first construction is serialized so
concurrent callers do not allocate duplicate arrays. Field-only scans leave it
unallocated. Failed construction is not cached as absence, and indexes from
different logical members remain separate.

Members are traversed in two forms. The code traversal reads each `code_item` and
delivers the original instructions for a caller that analyses a body. The header
traversal delivers the original index, access flags and declaration without reading
code. Method-only selection reads field record boundaries and identities but does
not resolve unrelated field declarations; a listing that only publishes members uses it, because reading every body to
print its header costs more than the listing itself. Neither form is a second
parser: both walk the same class data with the same per-declaration failure
classification.

A header-only method visitor can select by the original name-string ID and
checked name before acquiring a prototype. Identity/name read failures retain their
classification. Only accepted names require prototype facts; a rejected name does
not depend on prototype availability. Class-data bounds, original membership and
cross-group duplicate handling remain shared with the unfiltered traversal. The
application memoizes completed name matches within one logical index and query;
it does not cache read failures or share numeric IDs across logical members.

## Method source preparation

`ProgramIndex::method_source` consumes a method entry from P0 `ClassFacts`.
It uses the original code offset, delivered declaration and flags without walking
class-data again or resolving the method declaration a second time. It establishes
safe code/header/instruction/try/handler ranges and original register counts.
The reader binds the existing OwnerFacts and the same-DEX declaration view into
one private OwnerContext for downstream relation queries. ClassFacts itself
remains an owned snapshot; method products retain the declaration view's input
lifetime without borrowing the Engine or creating an index/cache ownership cycle.

`MethodSource` separates a code-bearing member from a declaration-only member.
An omitted code offset and native/abstract flags are checked for consistency;
no-code declarations MUST NOT create empty executable graphs. The reader checks
incoming word totals against the declaration, try record ranges/order and handler
encoding. Typed-catch descriptor reads are retained once per original TypeId in
immutable shared facts. A read failure or non-class catch descriptor keeps its
classified issue without erasing safely bounded handler records. Unknown record
extents, malformed encoding or unsafe targets still stop input preparation.
The reader does not know instruction boundaries before P1.

Method/parameter annotation reads are independent of code analysis. Original
annotation facts and local failures also remain available through P0.
P0's owned immutable MethodMetadata handle scopes records to this method and its
parameters. MethodSource and P1 retain it alongside the independently checked
metadata range/error; P7 receives complete available content without parsing again. The code
view retains the original debug-info offset; the former debug-program interpreter
is not an active engine module. P1 does not require source names, generic metadata,
line numbers or debug local names to analyze code. Current range/metadata behavior
must be inspected in the reader rather than inferred from removed debug schemas.

`RawMethodView` can retain its already checked code range as an owned
`RetainedCode` for a stopped-member delivery. P1 keeps this original source view
beside a decoded prefix. Retention copies bytes and established items without
re-decoding, inferring boundaries or promoting the prefix to a complete product.
The shared metadata value reader supplies owned reference-declaration leaves for
P0 preparation/annotation consumers; P8 receives those facts rather than an index.

## P1 disassembly facts

`decode_method_source` uses the method reader product and the selected index's
restricted original reference readers. One decoded instruction/payload directory
is shared by structural checks and subsequent products; P1 MUST NOT recover
boundaries by guessing widths or seeking a later return.

For established positions and facts, `DecodedMethod` retains:

| Facts | Consumer need |
|---|---|
| Identity, declaration and flags | Preserve the selected method and original signature. |
| Entry layout | Exact `this` and parameter slots, type identities and widths. |
| Instruction/payload directory | Original PC, opcode/format/width, register words, literal bits and borrowed payload data. |
| References | Instruction reference slots associated with checked string/type/field/method/prototype/call-site/handle facts. |
| Invoke layout | Receiver presence, ordered parameter widths and result category; wide argument words remain adjacent. |
| Structural associations | Branch/switch targets, payload ownership, handler entries and producer/result adjacency. |
| Protection | Original try boundaries and ordered catch clauses, with typed catches and catch-all distinguished; original typed-catch descriptor results and use issues are retained from the reader. |
| Independent metadata state | Owned method/parameter annotation content, coverage, original ranges and independent recorded read issues. |

P1 checks all decoded instruction register spans and explicit targets, including
unreachable instructions. Structural reachability checks reachable fallthrough;
it does not prove conditional paths feasible. `move-result*` MUST follow its
checked producer and MUST NOT be an explicit branch/handler entry. Handler and
move-exception association are checked independently of normal branches.

`catch_type` exposes the retained descriptor result for an original catch TypeId;
it does not resolve that reference again. Catch ordering and targets remain in
the original protection associations, independent of declaration availability.
`array_payload_at` projects checked element width/count and borrowed bits from
an existing decoded layout. Padding is excluded; no payload header is re-decoded.

Call-site readers retain bootstrap argument events, original IDs, encoded range
and bytes. P1 retains a flat leaf-declaration pool for bootstrap references and
checked handle members; it does not discard trailing encoded values after
validation. Container events preserve order and input-derived nesting without
recursive boxes. Dynamic invocation prototypes remain independent of target names.

P1 checks opcode requirements against original reference facts. It MUST NOT use
Java source legality, normal compiler templates, natural loops or complete external
hierarchies as input gates. Known format damage remains `Malformed`; unsupported
encoding/capability remains `Unsupported`. Instruction/reference association is
not a whole-method legality badge.

## Local availability and publication

P1 MUST follow [the P1–P9 contract](../PIPELINE_DESIGN.md#92-p1p9-统一消费局部结果).
The coverage obligations here define that full target; the current publication
floor and retained representations are specified under Outcomes below.
Its published product MUST describe code coverage as decoded instructions,
payloads, reliably bounded unrecovered ranges and ranges without established
instruction boundaries. A range is not an executable instruction.

P1 MUST retain each established operand, reference identity and structural
association independently. A reference read failure remains associated with its
original slot and reason; it MUST NOT erase known width or other independent
facts. Invoke word order, parameter grouping, result category and adjacency MUST
have explicit states when they cannot all be established; P2 MUST NOT guess them.

Only the reader/decoder that established an extent can authorize continuation at
its end. P1 MUST NOT guess an unknown width, scan for a later return or manufacture
a successor. It MUST preserve later independently established positions when safe.
P2 consumes the coverage and fact states through the same published product,
without retrying P1 or accepting its mutable construction snapshot.

## Outcomes and evidence

Published `DecodedMethod` retains reference-read results separately from opcode-use
issues. `reference_fact` distinguishes no reference site from a failed read;
`reference_use_issue` preserves a bad association without erasing a readable
owner/name/type/prototype. Operand and result issues retain their original PC.

Invoke binding consumes the same checked prototype facts, not another declaration
read. Parameter grouping and result category are independent: failed grouping
retains a readable return category, and a polymorphic prototype remains usable
when the separate target declaration is unreadable. `invoke_layout_at` exposes
failed grouping; `invoke_result_at` exposes the independent return state.

P1 currently publishes only with a complete instruction directory, entry layout
and established control/protection associations. Unknown width or unreliable
control still produces `DecodedOutcome` with retained items and a classified error;
this includes resource/invariant construction stops rather than dropping the prefix.
That snapshot MUST NOT be promoted or used to invent a graph. Local reference,
register-span, argument-grouping, result-use and return-kind issues do not themselves
prevent publishing a method when these minimum guarantees hold. Broader control
uncertainty handling remains governed by the target contract and the engine README.

`RawMethodView::visit_layouts` and `visit_instructions` are read-only original
inspection paths over safe ranges. They are not substitute analysis routes.
Engine development inspection uses the single `dbg_shell` example; it observes
retained products rather than starting a phase-specific retry.

P1 byte correspondence is inspected by `roundtrip`/`roundtrip-all`: the shell
independently re-encodes retained instruction/payload facts and compares original
bytes. This evidence does not validate P2 behavior or final Java semantics. P2's
own translation obligations are in [02-flow](02-flow.md).

## Declaration prefix view

`input::prefix` reads complete class-definition or string-pool lists from a logical
member prefix. Classes need the header, declaration tables and their descriptor
items; strings need the string-ID table and every payload it references. It uses the
same checked reader: `Cursor` range arithmetic, the same `string_data_item` read
(declared count plus terminator), the same table strides, link-overlap rejection and
class descriptor shape check as `LocalDeclarations::checked_class`.

The one obligation that differs is the bound of a string item. The whole-member
reader bounds every item by its `map_list` section; the prefix view bounds it by the
declared `data` range and the accepted byte length, and therefore reads no map. That
is a weaker bound, so the view MUST:

- report `Needs(bytes)` instead of a truncated value whenever a table, an item or one
  of its units is not yet inside the accepted bytes;
- report `Whole` for DEX 041, whose header chain and map need the whole member;
- return every requested definition or pool entry in original table order, or none;
  never publish a prefix of the list;
- leave classification of a genuinely malformed member to the whole-member read,
  which is what a caller falls back to.

A ready string pool borrows the accepted input once and stores only payload start,
payload end and declared unit count per entry. These offsets fit the physical
extent declared by DEX 035–040; DEX 041 still requires the whole-member reader.
Visitation rebinds established spans to the same immutable input without reading
string-ID records, length prefixes, terminators or encoded units again.

A caller MUST fall back to the whole member whenever the view does not answer, and
MUST NOT shorten a result because a prefix was unavailable. Hierarchy relations
(superclass/interface lists) are not part of the view; they require the whole member.

## Application input safety

The application owns input snapshots, physical archive identity and decompression.
Whole-entry DEFLATE uses libdeflater. Prefix DEFLATE uses the operator-approved
`flate2`/`zlib-rs` streaming reader, bound to one entry and scratch buffer. Growth
resumes the stream and exposes only its measured output bytes. An insufficient-space
result from a one-shot codec MUST NOT be treated as an established prefix length.
A leased regular file is memory-mapped read-only instead of copied, so residency
follows the pages a request reads. The mapping is the sole `unsafe` operation in
the workspace, it never outlives the descriptor it was made from, and no code
takes a mutable view of it. The host MUST NOT truncate or rewrite the file while
a request holds its lease; that obligation replaces the byte-copy snapshot the
application previously kept. ZIP ranges, sizes, supported compression and the
declared payload size remain checked; extraction publishes one selected original
entry atomically
and MUST NOT rewrite DEX headers or expose a partially written successful file.
AXML decoding preserves usable partial content with explicit diagnostics rather
than silently removing records. See the [CLI README](../../../apps/diana/README.md) for current selectors,
publication options and output behavior.

All native reads, growth, decompression and publication MUST preserve classified
failures and backpressure. There are no numeric resource quotas.
Changes require concrete original-input and semantic evidence under
[CONTRIBUTING](../CONTRIBUTING.md); removed test/fixture contracts are not active
acceptance jobs.
