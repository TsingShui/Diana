# Diana CLI

Diana is the native command-line front end of the
[Diana engine](../../crates/diana-decomp/README.md). `get-class` and `get-method`
deliver Java-like text recovered from original P0/P1 facts by the active backend.
The binary is `diana` (`diana.exe` on Windows). CLI binary releases use
`diana-v<version>` tags and are available from
[GitHub Releases](https://github.com/TsingShui/Diana/releases). Cargo installation
builds from Git or a local checkout; the crate is not published to crates.io.

The [approved architecture](../../docs/diana/PIPELINE_DESIGN.md) owns product
boundaries. This package owns input mapping, APK/ZIP access, selection,
command routing and file publication. Decompilation uses only the analyzed input;
external classpaths, declarations and runtimes are not required.

## Commands

```text
get-class   Decompile one class
get-method  Decompile one method
outline-class   Show class members and structured method outlines
outline-method  Show one method outline, with optional call expansion
entries     List physical archive entries
extract     Extract one physical entry
strings     List original string values
xref        Find references, hierarchy and method relationships
resources   List resource IDs, configurations and values
manifest    Decode the binary AndroidManifest.xml
classes     List class definitions
tree        Show the code package hierarchy
fields      List defined fields
methods     List defined methods
help        Print help for a command
```

Commands accept `--format text|json` and `-o <file>`. Extraction always writes
the requested payload to `-o`, with no stdout result. Index queries return every
match. `xref` defaults to `--limit 50` with an explicit warning when limited;
`--no-limit` returns every reference. An empty match is a normal empty result.
Fuzzy searches rank matches across all input images before presentation. Fuzzy
is the default for filtered searches and xref; unfiltered lists and the other
matching modes retain completion order.

| Command | Presentation |
|---|---|
| `entries` | `name, compressed_size, uncompressed_size, compression` |
| `classes`, `fields`, `methods` | `at` |
| `tree` | Package hierarchy and subtree definition counts; optional class leaves |
| `strings` | `at, value` |
| `xref` | `context, match, use` |
| `resources` | `context, id, name, config, value` |
| `manifest` | XML, or JSON with `status, xml, warnings` |
| `get-class`, `get-method` | Backend Java-like text, or JSON schema 4 |
| `outline-class`, `outline-method` | Structured tree with call links, or JSON schema 1 |
| `extract` | One output file |

`classes` supports `--filter`, `--extends`, `--implements`, `--subinterface`
and `--kind`. Hierarchy filters inspect direct original superclass/interface
edges. `fields` and `methods` accept `--class` as an exact descriptor or dotted name.

`classes`, `methods`, `fields`, `strings`, `resources` and `xref` share four matching modes:

| Mode | Selection |
|---|---|
| Default, or `--fuzzy` | Case-insensitive ordered subsequence through `nucleo-matcher` |
| `--substring` | Case-insensitive contiguous substring |
| `--regex` | Case-insensitive Rust regular expression |
| `--exact` | Complete, case-sensitive UTF-16 value equality |

Mode switches are mutually exclusive. List commands require `--filter` when a
mode switch is supplied. Classes match descriptors, methods/fields match names,
and strings match values. Substring and fuzzy modes treat metacharacters and
spaces literally; no fzf operator syntax is parsed. Fuzzy relevance is the nucleo
score divided by candidate scalar length, favoring compact matches. Ties use raw
score, shorter candidate length, then original coordinates.
Other modes retain arrival order. Ranking retains every match, so weak matches
remain available after the stronger results. Fuzzy presentation waits for the
complete scan and buffers the matched rows with fallible allocation.

```sh
diana methods app.apk --filter oncrt
diana methods app.apk --filter onCreate --substring
diana methods app.apk --filter onCreate --exact
```

Add `--regex` to interpret `--filter` as a Rust regular expression:

```sh
diana methods app.apk --filter '^on(Create|Resume)$' --regex
diana fields app.apk --filter '^(s?instance)$' --regex
diana strings app.apk --filter 'https?://' --regex --format json
diana classes app.apk --filter '^Lcom/example/' --regex
```

Regex also defaults to case-insensitive matching; inline `(?-i)` enables case
sensitivity. Regex patterns search within semantic text, not locators or printed
escape sequences; use `^` and `$` for whole-value matching.

Fuzzy and regex matching use the existing UTF-8 view where available. Other
MUTF-8 values are decoded to Unicode text, combining surrogate pairs and replacing
unpaired surrogates with U+FFFD for matching only. Fuzzy matching uses simple
Unicode case folding without accent normalization. Published original values and
identities do not change. Matcher representation limits produce explicit errors,
never truncated input or an empty successful result.
Rust regex syntax supports groups, alternatives, repetition and Unicode classes,
but not look-around or backreferences. Patterns compile before input scanning;
invalid syntax exits 2 without publishing output.

Query `--jobs` bounds concurrent physical DEX roots; its default is the available
CPU count, capped by the number of roots. Workers claim roots dynamically, with
larger query roots started first. Logical headers within a root retain their own
identities. `get-class --jobs` instead sets Engine's
method worker count; its default is one. Explicit job counts must be positive.

## Package tree

```sh
diana tree app.apk
diana tree app.apk --prefix com.example -L 2
diana tree classes.dex --classes --format json -o tree.json
```

`tree` consumes the ordinary class query, without reading method bodies.
`--prefix` selects an exact root package in dotted or slash-separated form;
matching respects package boundaries. Without it, `.` represents the input root,
including classes in the default package. An unmatched root has zero definitions.

`-L/--level` selects a positive package depth relative to that root.
`--classes` includes classes in visible packages; class leaves do not consume
package depth. Counts include descendant definitions beyond the visible depth.
Equal class names in different DEX entries remain separate leaves, each with a
canonical `get-class` locator. Dollar signs remain part of the original class name.

Text and JSON present the same selected hierarchy. JSON schema 1 contains
`complete`, `issue` and `tree`: `tree.root` indexes the flat `tree.packages`
arena. Each package carries `parent`, `children`, its display `name`, original
`name_units`, subtree `class_count`, and optional `classes`. Leaves carry
`name`, `at` and original `descriptor_units`. Child indexes avoid recursive
input-derived nesting. A scan failure retains recovered nodes, marks the tree
incomplete in both formats, and exits nonzero. Package/class siblings are not
alphabetically sorted. File output uses the ordinary atomic publisher.

## Class and method outlines

```sh
diana outline-class app.apk 'com.example.Worker'
diana outline-method app.apk 'Lcom/example/Worker;->run()V' --call-depth 2
diana outline-class classes.dex 'com.example.Worker' --expand-all --format json -o outline.json
```

These commands accept the same exact selectors as `get-class` and `get-method`.
The default shows the selected method, or every original class member, without
starting analysis of additional callees. `outline-class --jobs N` selects the
ordinary Engine method workers. Imports/package lines are omitted from the tree;
the retained publication still contains them.

P9 records lexical scope, text ranges and original call facts during its existing
render traversal. The outline preserves conditions, loops, ordered catches,
selected finally/synchronized forms, writes, exits, explicit transfers and local
gaps. It does not parse Java-like text or recover missing structures. Each method,
class and available field declaration carries its canonical locator. An empty
class initializer remains a member even when its ordinary code spelling is empty.

`--call-depth N` permits that many cross-method levels; zero is the default.
`--expand-all` instead follows every available referenced declaration body. The
options are mutually exclusive. Class members keep their member positions, while
additional bodies appear under `expanded methods`, not as invented class members.
Each delivered call occurrence retains its original PC; repeated bodies and
recursion link to a single method node. Origin-only cleanup and ABI attachments
are marked as attachments, not additional executions. Breadth-first expansion gives a shared method its shortest depth from
the selected roots. Loops show structure, not iterations. Expansion is finite in
the input's original method identities, but has no implicit row or body limit.

External owners, ambiguous definitions, missing declarations and dynamic dispatch
remain explicit. Virtual/interface/super expansion shows the referenced declaration
body, not a proven runtime target; custom/polymorphic targets are not guessed.
Expansion caches one Engine per used physical root and uses the ordinary method
entry. An unavailable callee does not erase the caller or another available body.

Text and JSON present one flat node arena and the same call states. JSON contains
`root`, `nodes` with parent indexes, `calls`, concrete `issues` and the retained
ordinary P9 `publications`; source ranges index those publications. `complete`
describes traversal/delivery availability, not semantic verification. Explicit
depth boundaries and external references are normal leaves. Local input, selection,
analysis or rendering failures retain output and exit nonzero. File publication
uses the ordinary atomic publisher.

## Original identity and selection

```text
class-at  = e<entry>:l<logical>:c<class-def>::<descriptor>
method-at = e<entry>:l<logical>:c<class-def>:m<method-id>::<owner>-><name><proto>
field-at  = e<entry>:l<logical>:c<class-def>:f<field-id>::<owner>-><name>:<type>
string-at = e<entry>:l<logical>:s<string-id>
```

`e` is the physical ZIP central-directory record index. A bare DEX uses `e0`.
`l` is the original header-chain ordinal within that physical container: zero
for an ordinary DEX, potentially several ordinals for DEX 041. `c`, `m`, `f`
and `s` are original table indices. Engine's request-local `DexId` is a separate
coordinate; it is not a physical entry or logical-header ordinal. A canonical
locator used by `get-class` or `get-method` opens only its physical root.
Unrelated DEX payloads are not decompressed or required to be readable for that
retrieval. An unqualified class name searches all roots to establish a unique match.

`get-class` accepts a class-at, descriptor or dotted class name. `get-method`
accepts a method-at or the complete `Lowner;->name(args)return` reference.
Use locators printed by `classes` and `methods`, shell-quoting dollar signs,
spaces and arrows. An ambiguous class or method reference is not guessed.

Class locators must agree with their checked descriptor. Method selection
consumes the same Engine's cached P0 directory and checks membership, original
MethodId, name and complete prototype. A reference in `method_ids` alone is not
a definition in the selected class. An unavailable declaration or unfinished
directory does not establish that a method is absent.

Class retrieval analyzes all known original members. Single-method retrieval
calls `Engine::analyze_method` with the selected ID and uses its standalone
P8/P9 display; it does not analyze siblings or slice text out of a class result.

`xref` takes `string`, `type`, `method`, `field`, `impl`, `extend` or `resource`. Names and
values use the same default ranked fuzzy matching and explicit mode switches as
`--filter`. Literal type/owner patterns accept dotted fragments, converted to
slash-separated descriptor fragments. Method/field patterns accept a bare name,
`owner->name`, or `owner->` for every member of matching owners. In substring and
fuzzy modes, an optional method prototype or field type is exact.

In regex mode, owner and member expressions are compiled separately; owner regexes
operate on original descriptors without dotted-name conversion. Parentheses and
colons in member regexes are syntax, not signature separators. Use exact mode or
a coordinate to constrain the complete member signature.

```sh
diana xref app.apk method oncrt
diana xref app.apk method 'example.Activity->onCreate' --substring
diana xref app.apk method '^on(Create|Resume)$' --regex
diana xref app.apk field 'example.Config->'
diana xref app.apk impl Runnable
diana xref app.apk extend activity
diana xref app.apk string 'https://example.invalid/' --exact
```

A canonical coordinate resolves and validates the selected original value, then
searches all input images by that complete value. Class coordinates select type
or hierarchy targets; method/field coordinates also check definition membership
and the complete original signature. String coordinates select an exact pool
value. `--exact` accepts an exact string, complete type or complete member reference
without requiring a target definition. For `string --exact`, coordinate-shaped
text is a literal string value.

`impl` reads direct interface-list references, including interfaces extending
interfaces. Its `use` is `implements` or `extends-interface`. `extend` reads the
direct superclass reference and reports `extends`. Hierarchy rows carry class
coordinates, with no invented method or code-unit offset. These queries read
only checked declarations, not method bodies. Add `--recursive` for indirect
relationships across input DEX images. Indirect rows retain a depth and `via`
witness; missing, ambiguous or cyclic declaration paths are explicit Unknown
issues, not evidence that no relationship exists. Method queries support `--transitive N` for that many upstream caller
levels, with exact method identities after the initial query match. Fuzzy caller
expansion carries the best originating relevance for each frontier method;
owner/name scores are combined when both are fuzzy. Coordinate targets remain
exact and unranked regardless of a fuzzy switch.

`--class` restricts the referencing class by exact descriptor or dotted name.
Operand queries do not require the target to have a definition in the input.
A local reader failure retains independently recovered rows and continues at the
smallest reliably located scope. A bad code item does not stop sibling methods,
classes or physical DEX roots; a damaged instruction suffix does not erase its
already decoded reference prefix. The
`context` locator retains physical entry and logical DEX identity, so xref rows
omit the redundant entry-name column.

`xref` defaults to `--limit 50`. The limit applies to the total displayed rows
across all requested caller depths, identically in text and JSON. `--limit N`
overrides it; `--no-limit` removes the limit. The existing `--limit 0` spelling
also returns all rows. Explicit `--limit` and `--no-limit` cannot be combined.
Fuzzy queries rank all matches by length-normalized relevance, raw score and
candidate length, then caller depth and original coordinates, before applying
the display limit. Other modes select
rows in arrival order. All requested depths are scanned before display selection,
so hidden rows still contribute to the caller frontier and total count.

Only when the total exceeds the current limit, text starts with:

```text
//Diana Warn: Too much results, only show top 50 , use --no-limit or --limit N
```

The number in this warning follows the effective limit. JSON remains a valid
document: xref schema 3 carries the same warning in `warnings`, alongside
`complete`, `total`, `recovered_total`, `shown`, `truncated` and `issues`. A complete
scan has an exact `total`; an incomplete scan has `total: null` and reports only
its recovered count. Issues retain original scopes, failure kinds/reasons and
available byte ranges. Display truncation and incomplete coverage are distinct. A limit does not change original reference
identities, and unlimited output has no truncation warning.

## Declaration and dynamic references

Type xref includes field types, method parameters/returns, superclass/interface
references, annotation descriptors and metadata values, as well as instruction
operands and member-signature types. String/member searches include their original
annotation/static-preparation references. Declaration rows use class, field or
method coordinates without an invented code-unit offset; `use` identifies
`field-type`, `parameter-type[N]`, `return-type`, `annotation` or `static-value`.

Call-site and method-handle references expand through the shared checked reader.
Their `use` retains the original instruction and reference path, including
bootstrap/member kinds and encoded-value byte positions. This is encoded
reference evidence, not a claim of runtime dispatch. A broken dynamic suffix
retains established header/argument leaves and an explicit local issue.

For method definitions, select a relationship instead of reference sites:

```sh
diana xref app.apk method 'Lexample/I;->run()V' --exact --relation implementations
diana xref app.apk method 'Lexample/Base;->run()V' --exact --relation overrides
diana xref app.apk impl example.I --exact --recursive
```

These queries consume input declarations, flags and hierarchy paths without
opening method bodies. Results retain actual declaring methods: inherited
implementations do not fabricate methods in subclasses. `use` names the receiving
class and witness path, so one declaration can appear for multiple receivers.
`--class` selects the receiver for method-relationship queries. Interface default
methods remain explicitly labeled candidates, not resolved runtime dispatch.
Unestablished return covariance, missing/ambiguous ancestors and conflicting
defaults produce Unknown issues. Caller expansion and method-relationship
selection cannot be combined.

## Resources

```sh
diana resources app.apk --filter app_name --substring
diana resources app.apk --filter 'https://' --substring --format json
diana xref app.apk resource 0x7f120123
diana xref app.apk resource string/api_host --substring
```

The catalog reads the input APK's `resources.arsc` only. Each configuration remains
a separate original row, with a physical-entry/byte locator. `default` identifies
a zero-qualifier configuration; other configurations retain their complete encoded
configuration as hexadecimal. Compact, dense, sparse and 16-bit entry-offset forms
share the checked chunk/string-pool readers. Complex values retain ordered map
entries; unsupported or damaged local values keep their available IDs/names and
explicit unavailability. Dynamic resource references remain raw, not linked to
external framework tables. Catalog schema 3 has the same coverage/count/issue
fields as xref.

Resource xref matches constant/array-payload and preparation/annotation integer
bits against catalog IDs. Every such hit is labeled a candidate: equal bits do
not prove resource access or data flow. Numeric IDs can still be searched when
names are absent or the input is a bare DEX; the name and resource coverage remain
explicitly unavailable. A resource-name/value search requires the resource table.

## Decompilation text and JSON

Text is exactly the existing P9 UTF-8 text. The notation is
`diana-java-like-v1`, described in [P9 rendering](../../docs/diana/spec/09-rendering.md).
Available expressions, original members, explicit operations and local recovery
gaps remain visible. Java legality or compilability is not an output gate.
Diagnostics are not hidden behind a verbosity flag.

Schema 4 has these top-level fields:

- `schema_version: 4` and `command: "get-class" | "get-method"`.
- `selection`: physical `entry` (`null` for bare DEX), `logical` header
  ordinal, container-relative byte `header_offset`, Engine `class` key,
  optional `method` ID, `class_at` and optional `method_at`.
- `result`: the rendered publication with `class, notation, text`.

`result.text` is byte-for-byte the text presentation. Local member failures
remain content of that publication. A shared rendering failure is returned as a
request error; an unfinished prefix is never reported as a successful complete
result.

The old schema 3 `{kind, product}` wrapper, schema 2, Java/Mixed status
projections, whole-class method-text slicing, `--verbose` diagnostic gate and
declaration-catalog adapter are removed.

## Query and manifest output

Query text is UTF-8 TSV with the column header. Display fields escape backslash,
tab, CR, LF and NUL; isolated surrogates remain escaped instead of being replaced.
Query JSON uses `{schema_version, command, items}`. `strings` uses schema 2 with
`at` and `value`; the former `entry` name column is removed. Its locator retains
the exact physical-entry identity. `xref` and `resources` schema 3 retain original locators and publish explicit
display totals, coverage and local issues as described above. Other table queries
remain schema 1.
`entries` reads the ZIP central directory only; it does not open or decompress
DEX payloads.

`manifest` uses XML or JSON schema 2 with `schema_version, command, status, xml,
warnings`; the constant entry-name field is removed. Recoverable AXML gaps remain
XML comments, JSON warnings and stderr
diagnostics. `--strict` reports a partial document as failure after publication.

## File publication and exit codes

Result serialization streams through one fallibly allocated transfer buffer to
stdout or a private temporary file beside the requested target. Large writes
bypass that buffer; buffering is not an output limit. A file is renamed into place
only after serialization and explicit flush complete. Writer/serializer/flush/rename failures do not replace the target.
A P9 rendering stop is a fully delivered failure report, not an interrupted file
write. Its text remains explicitly incomplete as described above.

A directory, symbolic-link target or input-file alias is refused. On Unix the
alias check also covers hard links. A closed stdout pipe is an output error,
not successful truncation. The CLI/host owns process termination; there are no
cooperative cancellation callbacks or numeric work/allocation/output quotas.

| Code | Meaning |
|---|---|
| 0 | A result was delivered, including local recovery gaps or a partial XML document |
| 1 | Input, unresolved selection, construction, rendering or output failure |
| 2 | Invalid command, flag or selector syntax |
| 3 | The requested single object or entry does not exist |
| 4 | The selector matches several definitions |

`extract <input> <entry> -o <file>` writes one physical record. Duplicate names
require `--entry-index`; name and index must agree. Size validation
applies to the original decompressed bytes. Binary XML is decoded by default;
`--raw` keeps the payload. An existing extraction target needs `--force`.
Extraction refuses directories, symbolic links and input aliases.

## Development

Build from the repository root:

```sh
cargo build --locked -p diana
cargo run --locked -p diana -- get-class classes.dex 'Lexample/Main;'
cargo run --locked -p diana -- get-method classes.dex 'Lexample/Main;->run()V' --format json
```

Use the engine's [`dbg_shell`](../../crates/diana-decomp/README.md) for P0–P9
inspection. The CLI does not expose separate phase constructors or a retry chain.
