# AntennaPod 3.12.3: source-aligned APK review

## Baseline finding and follow-up

Diana at `b214ec60086a09e1ab28b9dac89aa402ebb042c9` delivered every
method declaration in this APK without member stops or rendering failures.
A source-first review of 13 methods found a concrete text-fidelity problem:
character argument views survive in P7 but disappear from natural-looking calls
in P9. The retained call binding is correct; the text can suggest a different
overload. This is not an executed behavioral failure or a Java-compilation gate.
The fixture-backed follow-ups below address C1 and the reproduced P6 ownership
limitations R1/R2. The original review table and observations remain scoped to
that recorded baseline.

The reviewed control, retry and cleanup paths have no identified contradiction
under the source models below, apart from the baseline character-use spelling.
Some baseline protection and retry structures remain explicit. Their notation is a
readability limitation, not evidence that code or effects disappeared.

## Input and source correspondence

| Item | Recorded input |
|---|---|
| Application | AntennaPod, `de.danoeh.antennapod` |
| Variant | F-Droid `free` release |
| Version | `3.12.3`, version code `3120395` |
| APK | [F-Droid release](https://f-droid.org/repo/de.danoeh.antennapod_3120395.apk), 17,758,020 bytes |
| Informational APK SHA-256 | `6e79f28f69cef1a8b2e4f5f88c9c3cf3c3d7c51356f01394ac23b0dc5d56a00b` |
| Source | [AntennaPod](https://github.com/AntennaPod/AntennaPod/tree/175e5ef8fe3e125a5014d70ef0c57cf85661c71b), tag `3.12.3`, commit `175e5ef8fe3e125a5014d70ef0c57cf85661c71b` |
| License evidence | Upstream [LICENSE](https://github.com/AntennaPod/AntennaPod/blob/175e5ef8fe3e125a5014d70ef0c57cf85661c71b/LICENSE), GPLv3 text |
| Binary source evidence | Embedded `META-INF/version-control-info.textproto` names that same commit |
| Build evidence | F-Droid build entry names that commit and `free`; APK build metadata names AGP `8.11.0` |

The F-Droid preparation removes lines containing `play` from
`app-wearos/build.gradle` and `net/sync/wear-interface/build.gradle`.
The reviewed Java bodies are outside those files. The release enables shrinking,
but [app/proguard.cfg](https://github.com/AntennaPod/AntennaPod/blob/175e5ef8fe3e125a5014d70ef0c57cf85661c71b/app/proguard.cfg)
uses `-dontobfuscate` and retains application classes and members. Calling this
APK simply "obfuscated" would misdescribe the available correspondence.

The embedded revision and build record support source alignment; they do not
establish a reproducible-build result. Digests identify the inspected artifacts,
not a semantic verification or acceptance gate. Source, APK bytes and generated
outputs remain external review inputs; no upstream implementation is vendored.
Source is an independent review oracle, never an Engine declaration or classpath
input. Third-party declarations and runtime behavior absent from DEX remain
unknown to the engine.

## Retrieval coverage

| DEX entry | Classes | Declared methods | Expressed bodies | No-code declarations |
|---|---:|---:|---:|---:|
| `classes.dex` (`e4`) | 8,391 | 50,897 | 47,804 | 3,093 |
| `classes2.dex` (`e5`) | 650 | 2,609 | 2,544 | 65 |
| `classes3.dex` (`e6`) | 3,035 | 16,897 | 15,997 | 900 |
| Total | 12,076 | 70,403 | 66,345 | 4,058 |

Each DEX's delivered class/member counts matched the ordinary CLI inventory.
The survey reported zero class errors, stopped members, rendering failures,
render issues, upstream type issues and upstream storage issues. These counters
are retrieval observations, not whole-APK semantic acceptance.

The application namespace contains 2,477 classes and 10,398 declared methods,
including generated code. The source checkout contains 622 Java files and
14 Kotlin files. Six selected classes delivered 36 method declarations; the
source-aligned review below targets 13 methods, not all those bodies or all
application members. Third-party code and Kotlin behavior were not semantically
reviewed in this sample.

The debug counter `name_issues` counts any type-name-use reason, including
`NotPrinted`; it is not a count of missing members. All 29 populated name reasons
in the six selected classes were `NotPrinted`. No global reason breakdown was
performed.

## Source-first review

Expectations were recorded from the pinned source before inspecting the selected
Diana text. Whole-class textual diffs were used for navigation, not as an equality
oracle. Original DEX decoding and delivered products adjudicate suspected issues.

Source paths below are relative to the pinned upstream repository. The owner
names use the original `de.danoeh.antennapod` package prefix.

| Owner | Source path |
|---|---|
| `model.feed.FeedItemFilter` | `model/src/main/java/de/danoeh/antennapod/model/feed/FeedItemFilter.java` |
| `net.download.serviceinterface.FileNameGenerator` | `net/download/service-interface/src/main/java/de/danoeh/antennapod/net/download/serviceinterface/FileNameGenerator.java` |
| `parser.feed.util.DateUtils` | `parser/feed/src/main/java/de/danoeh/antennapod/parser/feed/util/DateUtils.java` |
| `storage.importexport.OpmlReader` | `storage/importexport/src/main/java/de/danoeh/antennapod/storage/importexport/OpmlReader.java` |
| `storage.importexport.OpmlWriter` | `storage/importexport/src/main/java/de/danoeh/antennapod/storage/importexport/OpmlWriter.java` |
| `ui.chapters.ChapterUtils` | `ui/chapters/src/main/java/de/danoeh/antennapod/ui/chapters/ChapterUtils.java` |

### Semantics

`~` means manual agreement on the stated paths/models with a different shape;
`?` means the text-level result is not established. Neither is an execution PASS.
Selectors are the original DEX `class_def` and method ID, not class-local method
positions. `URL/CacheControl` and `URL/boolean` identify the original overloads.

| Method | DEX/class/method | Judgment | Independent obligation and observed reasoning |
|---|---|---|---|
| `FeedItemFilter.matches(FeedItem)Z` | `classes.dex` / 8017 / 54019 | ~ | Ordered flag checks and repeated `isNew`, `isPlayed`, media and feed getters remain. Conflicting played/unplayed flags reject at the original call; null media bypasses the history-time rejection. Null feed bypasses state restrictions. With no include flags only state 0 accepts; with selected flags each recognized state uses its own flag, while an unrecognized state reaches true. The explicit jump reaches that same final return. |
| `FileNameGenerator.generateFileName(String)String` | `classes.dex` / 8167 / 54687 | ? | The scan tests bounds before `charAt`, checks the last accepted output character for space collapsing, and advances at L22 on every path. Empty output takes the RNG fallback. Length 241 stays unchanged; length 242 takes prefix 209, underscore and MD5. Character `append` binding is not faithfully disclosed in text: C1. |
| `FileNameGenerator.randomString(I)String` | `classes.dex` / 8167 / 54689 | ? | One `Math.random` call, multiply and index selection remain per iteration; the increment remains after append. The selected char-array load feeds the original character overload, but its use view is hidden: C1. |
| `FileNameGenerator.md5(String)String` | `classes.dex` / 8167 / 54688 | ~ | UTF-8 conversion precedes digest. Each signed byte is masked with 255, ORed with 256 and rendered through substring 1..3. The two selected checked exceptions share a null result; other exceptions propagate. The caller retains string concatenation of that null result, not an invented digest. |
| `DateUtils.parse(String)Date` | `classes.dex` / 8337 / 55145 | ? | Null throws before formatter access. Fast success skips fallback; only ParseException enters it. Normalization, bounded fraction scan, padding/truncation, 30 ordered patterns, ParsePosition reset, non-null/full-consumption check and exception retry remain. Exhaustion recurses after removing a weekday prefix before logging/null. Retry is explicit, and `replace(char,char)` loses its character views: C1/R1. |
| `DateUtils.parseOrNullIfFuture(String)Date` | `classes.dex` / 8337 / 55146 | ~ | Calls parse once; null returns before constructing now. Non-null constructs now once and returns the same date iff it is not after now. This is conditional on the parse/helper model, not validation of the date library. |
| `DateUtils.parseTimeString(String)J` | `classes.dex` / 8337 / 55147 | ~ | Keeps integer parsing, long multiplication/addition and float multiplication before conversion to long. Source models `01:02:03.5 -> 3723500`, `02:03.5 -> 123500`, `12 -> 0`, and four segments `1:2:3:4 -> 62000` follow the displayed branches. A bad minute precedes seconds parsing. NaN-to-long gives 0; `0:Infinity` reaches Long.MAX_VALUE. No new segment validation is invented. |
| `OpmlReader.readDocument(Reader)ArrayList` | `classes2.dex` / 362 / 2869 | ~ | Parser preparation precedes the loop; each iteration advances once. `isInOpml` persists across calls as in source. Title/text fallback and ordered URL/type reads remain; missing xmlUrl skips add, missing text uses xmlUrl. No reader close is invented. DEBUG-only logging and empty switch arms are absent in release DEX, not lost by rendering. |
| `OpmlWriter.writeDocument(List,Writer)V` | `classes2.dex` / 364 / 2873 | ~ | Preserves serializer setup, tag/attribute order, two title getters and selected repeated type/link getters. Non-subscribed feeds continue to the iterator test without outline work. Finish log follows endDocument; no writer close appears. |
| `ChapterUtils.loadChapters(Playable,Context,Z)V` | `classes2.dex` / 462 / 3261 | ~ | Already-loaded/non-forced return precedes I/O. Database, podcast-index and media reads retain their selected order, followed by two merges. Ordinary null result sets an empty list; interruption logs and sets null for retry. The original cast/check remains before FeedMedia-specific work. |
| `ChapterUtils.loadChaptersFromMediaFile(Playable,Context)List` | `classes2.dex` / 462 / 3262 | ~ | Each format attempt acquires a fresh stream. Nonempty success closes before returning; empty results close before the next format. Interruption precedes other IOException handling and rethrows without later attempts. A body failure plus close failure retains the body throwable and addSuppressed; successful-body close failure takes the original outside handlers. Explicit chains preserve these paths: R2. |
| `ChapterUtils.loadChaptersFromUrl(String,Z)List` | `classes2.dex` / 462 / 3264 | ~ | Forced refresh uses network only. Otherwise cache is tried first, with network exactly when cache is null or has at most one chapter. Larger cached results return directly. |
| `ChapterUtils.loadChaptersFromUrl(String,CacheControl)List` | `classes2.dex` / 462 / 3263 | ~ | Acquisition failures retain null response; later failures snapshot the acquired response. Normal success captures parse result then closes. IOException logs then closes; interruption rethrows through cleanup. Other failures also close when non-null. Each selected route closes once; a throwing ordinary finally-close replaces the pending result/throwable. Explicit copy phases preserve response identity: R2. |

Library methods, injected failures and mutable getters are explicit review
models, not executed helpers. Parser, HTTP, filesystem, media parsing and Android
runtime implementations were not executed. Unreviewed helper internals and
unvisited application paths remain uncertain.

## Findings

### C1: supplied character-use views disappear from text

**Classification:** confirmed text-fidelity/member-binding disclosure defect;
no executed behavioral mismatch is claimed.

In `generateFileName`, DEX PC 58 is `invoke-virtual`, registers `[0, 3]`,
method reference `55945`, `Ljava/lang/StringBuilder;->append(C)Ljava/lang/StringBuilder;`.
P7 retains the original `LC` shorty and an argument read with
`IntegralView { declared: TypeNameId(28) }`; that type name is `C`.
The call qualification is `natural`. P9 instead displays:

```text
int v4 = v1.charAt(v13);
...
stringBuilder.append(v4);
```

The same original method reference is called at PC 20 in `randomString`,
registers `[0, 2]`. Its argument has `IntegralView` of `C` as well, but is displayed
as an int array-load carrier followed by a plain `append(v8)`.
Under the ordinary Java StringBuilder model, appending character `A` yields
`A`, whereas appending its integer code 65 yields `65`. This is a source-derived
counterexample to treating the visible call as the integer overload, not an
observed execution of Diana output.

A second class exposes the same mechanism without a carrier: `DateUtils.parse`
PC 25 retains `String.replace(CC)String`. Its literal arguments 47 and 45 each
carry a supplied `IntegralView` of `C`, but P9 displays `v5.replace(47, 45)`
instead of disclosing their character interpretations.

The producer facts exist. P7 has not selected the integer overload or lost the
character data. The read renderer in
[`p9/expressions/read.rs`](../../crates/diana-decomp/src/p9/expressions/read.rs)
prints the carrier without the supplied integral view.
[`p9/expressions/literals.rs`](../../crates/diana-decomp/src/p9/expressions/literals.rs)
reduces `IntegralView` to the integral family and prints the numeric word. The
repair needs a general scalar-use spelling rule across carrier and literal
expressions; replacing
StringBuilder calls by name or globally declaring word carriers as char would
mask the cause. Original argument evaluation and storage/copy phases still apply.
No external declarations are needed to retain the DEX-supplied character binding.

### R1: retry loop remains inside an explicit handler graph

**Classification:** readability limitation; no identified control-flow error on
the reviewed retry paths.

`DateUtils.parse` recovers the fraction scan as a while, but the later pattern
retry uses L123, H134 and a goto inside a P6-supplied `Graph { reason: Irreducible }`.
Each unsuccessful or caught attempt still advances once. Non-null/full-consumed
results return before advancement; exhaustion reaches the weekday fallback.
The reason names the delivered plan's refusal, not a proof that the source loop
is inherently irreducible. Natural recovery needs an owning-stage investigation
of the handler rejoin and enclosing protection, not a renderer-side loop search.

### R2: resource and response cleanup remain largely explicit

**Classification:** readability limitation; no identified cleanup-order error on
the reviewed paths/models.

The three media-format attempts expose close/rethrow/addSuppressed chains rather
than source-level resource scopes. The private URL loader has a P6-supplied
`ProtectionBoundary` graph, selected response snapshots and shared cleanup.
These forms are verbose but preserve the reviewed primary/suppressed exception,
close-once, return-capture and recovery ordering. Replacing them with a broad
finally without those proofs could change which failure replaces which result.

## Fixture-backed follow-up

The later scalar-use repair consumes the existing range-proved P4/P7 views:
word carriers state `(value via char/byte/short)` at their selected uses, while
exact character constants print escaped UTF-16 literals. It adds no runtime
conversion, changes no local storage and does not infer external overloads.

- [Scalar-use reduction](../fixtures/atoms/scalar-use-views/README.md): reproduces
  carrier/literal disclosure, char/int coexistence, signed boundaries and escaping.
- [Fresh protected-join challenge](../fixtures/exceptions/scalar-views-protected-joins/README.md):
  exercises producer Phis, loop/catch/cleanup, snapshots, composition and guards
  that retain unproved views as original words.
- [Captureless retry reduction](../fixtures/exceptions/handler-retry-rejoin/README.md):
  reproduces R1 after retaining the actual absent captures and shared handler/update
  entry. The scalar repair does not claim natural-loop recovery.
- [Response/resource reduction](../fixtures/exceptions/response-close-snapshots/README.md):
  reproduces R2's protection/snapshot/suppression form, without declaring its
  verbose notation a behavioral defect or claiming a cleanup repair.

The isolated scalar repair 7347d50 repeated the same APK's complete retrieval
without changed coverage or failure counters. Six selected class source products
were equal to the review baseline; only FileNameGenerator and DateUtils changed text. The original char
append calls now disclose their views, and replace(CC) prints slash/dash char
literals. The other four selected texts are unchanged. Control/cleanup reasoning
is reused on that basis; character argument spelling was reread directly.
This is a bounded source/product/text review, not execution of AntennaPod.

Integration at 3ecfe5e retains concurrent changes through f233202. Repeating the
comparison against that latest-main baseline again gives equal six-class source
products and the same two scalar-only text changes, with unchanged full-APK
coverage/failure counters. Constructor grouping separately changes
FeedItemFilter.without and ChapterUtils.openStream relative to the initial
review. Direct reading retains allocation before argument calls, each argument
call once, initialization after arguments and the original outer-wrapper order.
The [scalar fixture record](../fixtures/atoms/scalar-use-views/README.md#combined-integration-observation)
records combined tests and corpus checks with their execution limits.

### P6 retry and cleanup ownership follow-up

The P6 exception-rejoin repair was developed against 73fc8793a91d766d021d117aa83ddfd644f7ac82.
It retains the scalar repair and supplies the missing owning-stage facts:

- R1: captureless handlers may share original normal updates or retreating gotos,
  subject to complete entrance and reach closure. Captured handlers still reject
  normal arrivals. No normal dominance, capture, edge or result is fabricated.
- R2: one guarded footprint separates protected work from its actual unguarded
  completion suffixes and shared handler continuations. Original P5 capture
  locals identify shared exception contexts, not value aliases. Loop recovery
  reentries retain their existing role; inner scopes consume an established
  outer body boundary. Original snapshots and outlet transfers remain supplied.

Fresh [nested retry](../fixtures/exceptions/captureless-retry-nesting/review.md),
[dual cleanup](../fixtures/exceptions/cleanup-outlet-frontiers/review.md) and
[shared context](../fixtures/exceptions/shared-cleanup-contexts/review.md) challenges
were authored after the respective drafts, with expectations before inspection.
Nine ordinary-Engine checks give two passes/seven failures on the fixed baseline,
then nine passes on the candidate. Both full isolated configurations pass 155
checks. All 111 prepared fixtures deliver 179 classes/853 methods with no retrieval/
render failures; complete P2–P5 and original-coordinate P6 facts remain exact.
A terminal-only draft that weakened existing cleanup nesting/recovery layout was
rejected. Its narrower same-local context proof and loop-reentry distinction were
checked against the original regression obligations, not accepted by weakening them.

The same APK still delivers 12,076 classes/70,403 declarations, with 66,345 expressed
bodies and 4,058 no-code declarations. Failure and upstream issue counters remain
zero on all three images. The13 selected methods retain P2–P5 and original P6
coordinates; only DateUtils and ChapterUtils change class text. Direct reading
covers the three changed subjects:

- DateUtils.parse now has a natural 30-pattern retry while, without the former
  Irreducible handler graph. Reset/applyPattern remain before the protected parse;
  null/partial results and selected exceptions advance once. Full consumption
  exits to the original committed Date without that increment. Exhaustion still
  recurses after weekday removal or logs/returns null.
- Media loading now groups each read/empty-test/log body and its original
  primary/secondary cleanup in natural nested catches. Empty results close before
  the next fresh stream; success closes before returning its captured list.
  Normal close failure goes to the original outside handlers, not another close.
  Body failure keeps its primary and suppression; interruption still precedes
  general IO recovery and stops later attempts.
- The private URL loader no longer has a ProtectionBoundary graph. Its enclosing
  all-catch and acquisition typed catches own the complete available context.
  Pre-acquisition failures retain null; later failures/logging use the acquired
  response. Distinct post captures and the shared cleanup remain explicit where
  their crossing contexts do not establish another lexical owner. Shared cleanup
  stays outside both body/catch adapters; each selected route closes once.
  Its failure replaces the pending return/throwable, never adding suppression.

The other four selected class texts are unchanged. Ordinary CLI get-method text
and JSON agree exactly for all three changed subjects, with no rendering issues.
These are bounded source/model/DEX/product readings and executed product tests,
not execution of AntennaPod, native fixture models or decompiled Java. Natural
catch ownership does not assert a general finally or try-with-resources rewrite.

### Combined integration observation

Merge 271eb4766c33a5af080888af931b2d16594b1ba1 retains concurrent terminal-follow
conjunction and monitor/catch-binding recovery through
cfc9da31c89a5d91ce3aca655b38020887d347ab. Both combined nextest configurations
pass 164/164, with zero skipped. Formatting, package all-target/all-feature Clippy
with warnings denied and release CLI/dbg_shell builds pass. Against that
latest-main baseline on identical bytes, 114 fixtures/182 classes/872 methods
retain complete P2–P5 and original P6 coordinates, with no retrieval/render
failures and the same six fixture text deltas. The APK keeps the complete
coverage above; the 13 selected methods again retain those source facts, and only
DateUtils/ChapterUtils change class text relative to latest main. Three changed
subject CLI text/JSON publications agree exactly without issues.

Direct integration reading additionally checks scoped header bindings for
resource primaries/secondaries and new response catches: each original capture
still binds once, before its receiving/body phases, with no new normal binding.

### Natural catch display observation

The P7 display rule that omits a chain already fully stated by printed catches was
compared on the same APK bytes. Selected-class text shrinks from eight chain lines
and 49 inline markers to six and 30. Besides the previously repaired methods,
`FileNameGenerator.md5` loses its two-exception union chain: the printed
`catch (NoSuchAlgorithmException | UnsupportedEncodingException)` now states both
original types at the one shared handler, and `all -> propagate` remains the
method-level route. `MessageDigest.getInstance`, the UTF-8 conversion, digest,
per-byte mask/OR/substring and string append order are text-identical. Failure
still returns null and other exceptions still propagate; the caller's existing
string concatenation of that null is unchanged. Full APK coverage, failure
counters and the 13 selected methods' P2-P6 coordinates remain unchanged. This is
a bounded text review, not execution.

Split-context captures whose values escape their handler remain explicit. DateUtils
text is exact to the isolated repair. The public URL wrapper separately gains the
concurrent terminal-follow disjunction: force still selects network immediately;
otherwise cache loads once, null skips size, size<=1 selects network once and a
larger list returns the original cache object. Static cache-policy access remains
before its call. No R1/R2 native callback model, APK/device or Java-output execution
is claimed by these combined product checks and bounded readings.

## Reproduction

Use the pinned APK and extract its three named DEX entries into `$DEX_DIR`.
Set `$APK` to that APK. Keep downloaded inputs and generated output outside
version control. From a dedicated Diana worktree at the recorded revision:

```sh
export CARGO_TARGET_DIR="$PWD/target"
cargo build -p diana --release --locked
cargo build -p diana-decomp --example dbg_shell --release --locked

target/release/diana get-class "$APK" \
  de.danoeh.antennapod.net.download.serviceinterface.FileNameGenerator \
  --format json --jobs 2

target/release/examples/dbg_shell --dex "$DEX_DIR/classes.dex" \
  --exec '{"cmd":"p1","class":8167,"method":54687,"details":true}' \
  --exec '{"cmd":"p1","class":8167,"method":54689,"details":true}' \
  --exec '{"cmd":"p1","class":8337,"method":55145,"details":true}' \
  --exec '{"cmd":"p6","class":8337,"method":55145,"details":true}'

target/release/examples/dbg_shell --dex "$DEX_DIR/classes2.dex" \
  --exec '{"cmd":"p6","class":462,"method":3263,"details":true}'
```

The full survey ran the existing `dbg_shell` command below on each DEX, with two
method workers per image and three images in parallel:

```sh
target/release/examples/dbg_shell --dex "$DEX_DIR/classes.dex" \
  --method-jobs 2 --exec '{"cmd":"p9-all","text_only":true}'
```

Release builds, ordinary CLI inventory/retrieval and the full three-image survey
ran successfully. Engine regression checks through
`cargo nextest run --locked -p diana-decomp --no-fail-fast` passed 115/115,
with zero skipped; the `--all-features` run also passed 115/115.
Package Clippy with all targets/features
and `-D warnings`, workspace formatting and relative report-link checks passed.
These checks cover the repository's existing tests, not execution of AntennaPod.

Integration recheck at `f426292da4610cbd483b48766c18b590cd1dcf87` retained the
concurrent coroutine changes through `899624e7785fa1b507fabf089bb347013db7d9e2`.
Its production sources equal that main revision; this review changes only two
quality documents. Both nextest configurations passed 120/120, with zero skipped;
formatting, package Clippy and release builds passed. Repeating the complete APK
survey gave the same coverage and zero failure counters. All six selected class
texts were byte-identical to the reviewed baseline, so the findings above remain
applicable and their manual review is reused on that basis.

Not run: application/source execution, device tests, Java recompilation of Diana
text or a final-plan behavioral executor. This review is a discovery record,
not a completed repair iteration under
[EVOLUTION](../../docs/diana/EVOLUTION.md).
