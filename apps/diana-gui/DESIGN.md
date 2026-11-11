# Diana GUI: resource, identity, navigation, view and result contracts

This document defines the data contracts between the GUI host and decompilation
capability modules, and constrains later integration.
It is not a claim about implemented capability. Current implementation status is in
[README](README.md); reference sources and licence boundaries are in
[SOURCE_MAP §6.3](../../docs/diana/SOURCE_MAP.md#63-rxd-capability-reference).
Fields express semantic requirements and freeze no Rust layout, trait signature,
serialisation or plugin ABI.

## 0. Responsibilities and delivery direction

| Participant | Responsibility |
| --- | --- |
| Host (GUI host) | owns input, manages resources and sessions, submits background requests, dispatches results |
| Container reader | reads member directories and bytes safely, provides original container associations |
| Backend Adapter | maps its engine's public inputs and delivered products into this contract's data |
| Engine | parsing, analysis, recovery and its internal task scheduling |
| Rendering layer | displays delivered data with GPUI Kit, sends user operation requests |

The Host MUST keep the borrow of bytes and declaration facts that engine input needs valid.
An Adapter MUST obtain products through its engine's formal entries; when a producer is
missing, the Adapter MUST report the missing capability instead of accessing private IR,
re-running stages or starting another reconstruction route.
The rendering layer MUST NOT start analysis, decide recovery strategy or accept GPUI
controls supplied by a backend.
DEX analysis and method scheduling use the public API of the
[active engine](../../crates/diana-decomp/README.md).
This contract grants no exception for external processes, non-Rust runtimes, `unsafe` or
externally declared input.

## 1. Resource contract: which input is analysed

A resource is one immutable input snapshot managed by the Host; it may be an opened file,
a container member, or bytes derived from original input. A resource is not a class, a
function or decompiled text.

| Field | Meaning |
| --- | --- |
| `resource_id` | resource reference allocated by the Host and never reused for another snapshot |
| `kind` | file, container member or derived input |
| `display_name` | display name that takes part in no selection, deduplication or original identity decision |
| `byte_length` | established actual byte length; Unknown when that fact was not obtained |
| `origin` | the position the user opened, or the parent resource, original member reference and derivation operation |
| `availability` | input obtainable, or the concrete unavailable range and cause |

- The Host MUST keep the bytes behind one `resource_id` unchanged; when it reopens or
  replaces input, the Host MUST publish a new resource reference instead of changing the
  old snapshot in place.
- The Host MUST obtain members through the container reader. When member names repeat,
  the Host MUST retain each member's original reference instead of merging or overwriting
  by path string.
- The container reader MUST perform the required range and arithmetic checks before
  reading, slicing, inflating and allocating. An unestablished length declaration MUST NOT
  be treated as the actual byte length.
- The Host MUST retain the parent/child origin chain. Bytes produced by inflation or
  decoding MUST use their own coordinate space; the Host MUST NOT assume their offsets
  equal outer file offsets.
- An Adapter MUST consume only the input resources admitted for this analysis target and
  associated resources from the same source. It MUST NOT supply missing declarations from
  adjacent directories, classpaths, system declaration libraries or the network.
- When one member is unreadable, the Host MUST retain the other members that can still be
  read independently and the error of that member.

One resource may establish several analysis sessions, for example ELF Native and Flutter
analysis. Identification returns matching candidates and their evidence and starts no
decompilation; the Host MUST NOT treat the first match as exclusive ownership, and MUST
NOT silently switch to another backend to hide an error in one identified format.

## 2. Identity contract: what is referenced, and where a position belongs

A runtime reference is used for routing; original identity comes from input facts. They are
not the same concept.

| Name | Meaning and valid scope |
| --- | --- |
| `BackendId` | capability implementation, for example `diana.dex`, `rxd.dex`, `rxd.flutter` |
| `FormatId` | input format or dialect; separate from the capability implementation and not used for exclusive routing |
| `SessionId` | one backend binding, associating a BackendId, a resource set and effective parameters |
| `EntityRef` | SessionId plus the backend's entity key; the entity key retains its owning input and original coordinate space |
| `SubjectRef` | one ResourceId or EntityRef serving as the target of a view request |
| `NodeRef` | node reference within resource navigation or session navigation; not equal to entity identity |
| `LocationRef` | position reference within a backend scope, interpreted and resolved by its owning Adapter |
| `DocumentRef` | reference to one published display product, including its content version |

- The Host MUST keep separate namespaces for different backends, sessions and inputs and
  must not merge entities because of a same-named symbol or equal numeric coordinates.
  After a session closes, the Host MUST NOT re-point an old reference at a new object.
- An Adapter MUST keep the meaning of an entity key stable within one session and must
  retain original selection information such as logical DEX, nested archive member, module
  or address space.
- An Adapter MUST NOT substitute a display name or current list index for original entity
  identity.
- The rendering layer MUST hand a LocationRef back to its owning Adapter. It MUST NOT guess
  conversions between DEX instruction offsets, IL offsets, Native virtual addresses and
  file byte offsets.
- The range of a text link MUST bind to a specific DocumentRef version; its unit is UTF-8
  bytes, not an original instruction offset. After the document is replaced, the rendering
  layer MUST NOT reuse old text ranges.
- The Host MUST distinguish a missing position from a legitimate zero-valued position.

Different navigation groups may point at the same EntityRef. Caller, target and original
position in a reference result may exist separately; when an exact source mapping is
missing, the Adapter MUST state that it cannot be located and must not fabricate a line
number. Original names, original tokens and original addresses are not rewritten because
the GUI escaped them for display.

## 3. Navigation contract: browse entries, assuming nothing is a class or method

Navigation provides structures expanded on demand; it does not mean all content has been
analysed.

| Field | Meaning |
| --- | --- |
| `node_ref` | node reference whose owner is the Host's resource navigation or one analysis session |
| `label` | displayable name, separate from original identity |
| `kind` | group, resource or entity, plus an optional semantic role |
| `children` | known leaves, children that may be requested, or an unknown child relation |
| `target` | optional ResourceId or EntityRef; a pure group may have no analysis target |
| `diagnostics` | concrete problems of this node; they do not replace the results of sibling nodes |

- The Host MUST route expansion requests by node owner. Resource members are produced by
  the container reader; analysis entries are produced by their owning engine, and the
  Adapter only projects them.
- An Adapter MUST retain established nodes, original order and local unavailability
  information.
  An incomplete child read MUST NOT be represented as a "successful empty directory" or as
  "known leaves".
- The Host MUST support pure group nodes and several navigation nodes referencing the same
  entity.
- The rendering layer MUST choose icons and controls from the semantic role and must not
  accept backend-drawn trees or menus.

Expansion results follow the coverage contract of §5. Incremental delivery belongs to the
same navigation request and is not implicit pagination; scroll position, queue capacity or
the number of rows visible in the UI MUST NOT truncate a successful result.

## 4. View contract: what an entry can show, and which unit is actually processed

A view is the display capability of one SubjectRef, not a globally fixed Java/DEX mode.
A pure navigation group without a SubjectRef makes no promise of owning an analysis view.

| Field | Meaning |
| --- | --- |
| `view_id` | view identity stable within a backend; the display title is not a routing key |
| `title` | name displayed by the GUI |
| `kind` | Text, Table, Properties or Bytes |
| `availability` | requestable on this target, or unavailable with a concrete cause |
| `render_target` | the actual analysis/display unit, which may differ from the entry the user selected |
| `parameters` | typed parameters declared by the backend and their effective values for this request |

- An Adapter MUST provide view availability from the current build, runtime conditions and
  input facts, not treat a compile-time feature as proof that the engine is installed or
  that this input is processable.
- An Adapter MUST state the real granularity. Methods, functions, classes, code objects,
  modules and whole chunks MUST NOT be forced into one `decompile(function)` call shape.
- The Host MUST associate documents by session, actual target, view and effective
  parameters, and must not mislabel one module's result as several unrelated function
  results.
- An Adapter MUST report the requested view and the actually delivered representation
  separately. When C# is unavailable but IL is obtained, the result MUST state IL and the
  cause, and must not label IL text as a successful C# result.
- The Host MUST NOT switch BackendId automatically, merge results of different backends or
  rotate through retries to fill in a view.
- A Text language/dialect label only controls presentation; the rendering layer MUST NOT
  make highlighting or language legality a gate on output visibility.

Text supports as-is readable text, a language label and optional location associations.
Table provides typed columns, rows and optional references; CLI tab-separated text or
arbitrary JSON cannot be treated as the whole GUI contract.
Properties provide ordered names and values, with a missing value separated from an empty
value. Bytes provide a resource reference and an established range; reading and export are
still performed by the Host, which owns the input, through the owning reader, and bytes are
not truncated into a preview that is then claimed to be a complete extraction result.
Search and cross-reference belong to explicitly requested optional capabilities; their
entries reuse EntityRef and LocationRef and return results that state their original
semantics.

## 5. Result contract: what was delivered, and what is unavailable

All expansions, locations and view requests use the same result semantics; concrete
payloads stay typed.

| Field | Meaning |
| --- | --- |
| `request_id` | request correlation allocated by the Host, not an original entity identity |
| `binding` | resource reference, optional session, request target, view and effective parameters |
| `payload` | delivered nodes, located targets or display product; a display product carries the actual target and representation |
| `coverage` | requested range, covered range, known unavailable range and unknown coverage |
| `diagnostics` | original classification, concrete backend code, cause, affected targets and known positions |
| `termination` | the request ended normally, or the request was interrupted with a concrete affected scope and cause |

- A producer MUST deliver coverage facts explicitly. A complete representation means the
  requested range was delivered, not that the output was independently verified, compiles
  or has proven semantics; partial coverage is a normal result.
- A producer MUST retain mixed results. When one member, fragment or position fails, it MUST
  NOT erase the parts of the same request that can still be delivered independently, or
  pass an unfinished snapshot off as a complete product.
- When one document mixes recovered text and original instructions, the producer MUST
  retain each fragment's actual representation and unavailability information and must not
  hide the difference behind one success mark.
- An Adapter MUST retain material uncertainty and concrete error classification. The error
  category itself MUST NOT decide the stop scope; widening to other members, resources or
  sessions requires a concrete shared-input/resource reason.
- When an unavailable range is known, the producer MUST state that range and cause; when
  the range cannot be determined safely, the producer MUST state Unknown instead of
  inventing an interval or omitting a count.
- The Host MUST associate both the request and the current view binding. A late result MUST
  NOT overwrite another session, parameter set or document version; the user closing a tab
  does not invalidate the underlying input lease early.
- An incremental payload MUST keep the semantic order established by the backend. Before
  the end arrives, the Host MUST NOT treat a temporarily empty queue as a completed
  request; an interrupted transport MUST NOT be represented as complete success.
- A search producer MUST distinguish a successful zero match, a missing target, an
  unsupported capability and unknown coverage, and must not represent them all as an empty
  list.
- The GUI MUST show available content and concrete local problems, not a whole-result
  verification badge.

This contract adds no engine cancellation token, numeric resource quota, runtime
independent verification or new analysis route.
Where a result goes after a view closes belongs to the Host; the ownership and delivery
obligations of already started engine tasks do not change because of that.

## 6. Mapping examples of format differences

The following table illustrates differences the contract accommodates; it is not a claim
that a backend is integrated or that a particular input is supported.
Capability evidence is in SOURCE_MAP §6.3.

| Capability | Resource/navigation | View and granularity differences |
| --- | --- | --- |
| DEX | APK member → logical DEX → package/class/member | facts, instructions and the actually delivered recovery result; logical DEX identity retained |
| JVM | JAR/nested archive → class → member | Java, bytecode; coordinates retain the archive member chain |
| .NET | bundle → assembly → type/method/resource | IL independent of optional C#; the actually returned representation is stated |
| Native | image → symbol/function | assembly and pseudo code use their own established location relations |
| Ark | HAP member → dynamic ABC → record/method | not every ABC dialect is treated as supported |
| Flutter | Dart AOT target inside ELF → class/function | may share input resources with Native analysis without sharing entity coordinates |
| PYC/PYZ | archive member → module → code object | module or code body; a fragment contains no binding facts outside the selected range |
| Lua family | chunk → prototype | code-object navigation does not mean every object has its own source-reconstruction entry |
| WASM | module → section/function | instructions, structural information and implemented C-like reconstruction |
| EVM | bytecode → selector/instruction/metadata | the existence of a unified interface source view promises no EVM source recovery |
| Archive resources | ASAR/ZIP/HAP/PyInstaller/wxapkg members | directories, properties, byte reads and export, not disguised as a decompiler |
