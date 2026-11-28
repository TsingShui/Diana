# Diana DecBench

Diana DecBench evaluates a decompiler build against a reproducible corpus. It
records independent evidence for each original method, mines concrete failures,
and tracks a repair through a fresh evaluation and a regression sweep. The
[subproject](.) lives in the Diana repository and uses the ordinary Diana CLI.

## Run it

Python 3.11+, SQLite 3.35+, a JDK and a built Diana CLI are required. Run these
commands from the repository root. When comparing revisions, keep each executable
and its source revision identifiable. Isolate worktrees, build outputs, and
mutable state when concurrent or overlapping work needs it.
[Working and verifying](../../AGENTS.md#working-and-verifying)

```sh
cargo build --locked -p diana
python3 quality/decbench prepare
python3 quality/decbench run --binary target/debug/diana \
  --revision "$(git rev-parse HEAD)" --output .cache/decbench/current.json
python3 quality/decbench report .cache/decbench/current.json \
  --output .cache/decbench/current.html
```

The run needs one executable. The original input and independently authored
expectations define the comparison. It prints the JSON report path, returning
0 when the requested checks pass, 1 for a definite contradiction, and 2 if
evaluation is incomplete. A run with both contradictions and unchecked evidence
records both and returns 2.

`suites` lists the available corpus. `run --suite ID` can be repeated. An authored
suite can be narrowed with `--at LOCATOR`. `--declared-only` selects the authored
suite manifests. The default also checks definitions for the whole prepared DEX
fixture tree. `--prepared PATH` selects a prepared APK and JVM corpus.
`run --input PATH` evaluates an authorized APK, DEX, JVM class, or flat JAR.
The JVM oracle uses the JDK's original descriptors and archive positions.
Nested JVM arrangements use their explicit fixture expectations. To update an
existing prepared corpus after adding fixtures, choose a new directory with
`prepare --prepared PATH` and pass that directory to `run --prepared PATH`. The
APK check uses physical ZIP locations and root DEX membership, and declares
incomplete coverage for a root outside the reader's scope. For large inputs,
`--inventory-only` measures member
identity without a per-method retrieval process. Use `--state PATH` before the
subcommand to isolate local campaigns and worktrees.

`run --resume` checkpoints each completed input independently. A checkpoint is
reused only for the same executable, input bytes, evaluator, oracle and selection.
Collection errors and unchecked suites are attempted again. Omit `--resume`
when reproducing a finding or measuring execution time.

## Replay a public APK corpus

`corpora` lists the opt-in projects. The checked-in manifests pin the APK release,
source revision, and reviewed methods for FairEmail 1.2338, AntennaPod 3.12.3,
and NewPipe 0.28.8. Automatic discovery selects Java source files.
`prepare --corpus ID --download` fetches the release and source archive into
`.cache/decbench/public-corpora/`. With `--apk PATH --source PATH`, it imports
local copies instead. Runs only read cached inputs. The evaluated APK and
selected source files must match the case evidence. Downloaded APKs, upstream
source, peer tools, and generated reports belong in ignored local storage.

```sh
python3 quality/decbench corpora
python3 quality/decbench prepare --corpus antennapod-3.12.3 --download
python3 quality/decbench run --corpus antennapod-3.12.3 \
  --binary target/debug/diana --revision "$(git rev-parse HEAD)" \
  --output .cache/decbench/antennapod-diana.json
python3 quality/decbench run --corpus antennapod-3.12.3 \
  --jadx-jar /path/to/jadx-1.5.7-all.jar \
  --output .cache/decbench/antennapod-jadx.json
python3 quality/decbench review .cache/decbench/antennapod-diana.json \
  .cache/decbench/antennapod-jadx.json --output .cache/decbench/antennapod-review.json
python3 quality/decbench report .cache/decbench/antennapod-review.json \
  --output .cache/decbench/antennapod-review.html
```

The peer adapter takes the fat JAR from the [official JADX 1.5.7 release](https://github.com/skylot/jadx/releases/tag/v1.5.7).
It binds each body to the exact original DEX entry and full descriptor and keeps
the original JADX spelling. Both runs evaluate the same independently selected
source and DEX evidence. `mine` finds verified regressions and peer gaps. `review`
links each paired method to its pinned source and identifies places to inspect,
such as a missing body, unresolved types, or a different cleanup form. Review
signals carry no correctness verdict. Only independently failing Diana checks
enter the repair queue.
The review also surfaces explicit jumps or unresolved types present in both
outputs and methods whose two parsed CFGs differ from the source CFG.

The default AntennaPod cases cover the 13 methods in the
[source-aligned report](../reports/antennapod-3.12.3.md) and the discovered
[subscription layout case](../fixtures/branches/subscription-layout-order/).
Select `--at LOCATOR` for one method. Add `--execute-models` to compile the source method and a
Java-compatible recovered method in the same authored model, with known
initial state and recording stubs. The
[chapter-order model](../fixtures/exceptions/response-close-snapshots/chapter-source-order/) checks ordinary
completion as well as a network interruption. It runs the modeled method in
scratch storage. The APK and Android services are outside the execution scope.
Use the same model setting on each arm of a comparison.

Each selected original code item has a body-availability check. A declaration
without a body and a decompiler's diagnosed failure stub can establish a
recovery gap. A missing tool or unidentifiable display leaves body coverage
unknown. The raw publication and independently supported call evidence remain
visible.

Declaration type checks compare explicitly displayed primitive return and
parameter types with the exact original DEX descriptor. Primitive arrays
retain their rank. Same-arity overloads, unknown spelling, and unresolved owner
wrappers leave the affected evidence unchecked. This check does not cover
reference type inference or local variables.

The [response cleanup](../fixtures/exceptions/response-close-snapshots/chapter-response-close/)
and [cache selection](../fixtures/branches/chapter-url-cache/) models also run
selected original DEX instructions against recording stubs. The report records
each model's original code-unit positions, branch targets and handler
transfers. Unsupported raw paths remain visible with their uncertainty.

Run one command across the cached projects to save a campaign bundle. Each
project and shard preserves its selection, producer reports, and processing
status. The index and opportunities files summarize independently evaluated
changes. The optional peer adds a side-by-side source review.

```sh
python3 quality/decbench campaign --binary target/release/diana \
  --jadx-jar /path/to/jadx-1.5.7-all.jar --execute-models --inspect-structure
python3 quality/decbench campaign --binary target/release/diana \
  --profile sample --shards 8 --limit 512 --inspect-structure
```

The sample limit applies per shard. Repeat `--corpus` to select projects. Use
`--resume` to reuse comparable completed evaluations. The campaign writes new
evidence under the configured output directory and reads only prepared assets.
It scans each project's pinned source and DEX once, then verifies the
selections for each shard. Each project includes a coverage report that shows
original definitions, source file populations, selected methods, and methods
measured by each evidence family.

```sh
python3 quality/decbench compare-campaign --baseline /path/to/earlier/campaign.json \
  --candidate /path/to/later/campaign.json --output .cache/decbench/campaign-delta.json --gate
python3 quality/decbench report .cache/decbench/campaign-delta.json \
  --output .cache/decbench/campaign-delta.html
python3 quality/decbench coverage --corpus antennapod-3.12.3 \
  --report .cache/decbench/antennapod-diana.json --report .cache/decbench/antennapod-jadx.json
```

Campaign comparison pairs exact producer roles, shard selections, method
identities, APKs, oracles and evaluator versions. The gate fails on newly
failing checks, lost evidence, setup failures or incompatible scope. A known
unchecked result can remain in a compatible comparison. Coverage accepts
repeated `--cases` and `--report` arguments for generated shards.

### Scan all source-aligned methods

Use the full profile to scan each project's reviewed and independently
verifiable Java candidates. It runs `diana export` once per APK using Diana's
default worker selection. Every shard evaluates methods from the same export.
The scan partitions larger selections automatically. The `--limit` setting
controls the maximum size of each shard.

```sh
python3 quality/decbench prepare --corpus fairemail-1.2338 --download
python3 quality/decbench campaign --corpus fairemail-1.2338 --profile full \
  --binary target/release/diana --revision "$(git rev-parse HEAD)" --timeout 120
```

The export reader uses original class coordinates and complete method
descriptors to attribute text. Ambiguous signatures and unavailable output
leave the affected checks unchecked. The export directory is temporary.
The campaign retains per-shard case manifests and reports, corpus coverage,
and a `bench.html` for human review. The bench page lists methods with
differences and shows the pinned original Java and Diana output side by side.
`review.html` retains the detailed evidence. Supply `--jadx-jar` for a paired
technical review.

Rebuild a bench page from a saved campaign without running either
decompiler. Add `--run` to include saved public run reports with executed
scenario failures and their event traces. Use `--prepared` when the source
cache has moved.

```sh
python3 quality/decbench bench --campaign /path/to/campaign \
  --output .cache/decbench/bench.html
```

Use `triage --baseline PATH --at LOCATOR --binary PATH` on a shard report to
reproduce a candidate through the exact method retrieval command. Compare full
campaigns with the same source and selections to track change across builds.

### Discover source-aligned methods

Discovery reads the cached source archive and original DEX before running a
decompiler. It matches unique top-level Java methods by their original
descriptor, verifies each selected call against source and DEX, then samples
across modules, control forms, and classes in a stable order. It excludes the
curated methods and writes the selected cases to the cache. Coverage is limited
to methods the independent matcher can establish. It checks displayed call
presence, selected argument values and character types, and simple direct field
accesses that the source and original DEX can both identify.

```sh
python3 quality/decbench discover --corpus antennapod-3.12.3 --limit 100
python3 quality/decbench run --corpus antennapod-3.12.3 \
  --cases .cache/decbench/discovery/antennapod-3.12.3-100.json \
  --binary target/debug/diana --output .cache/decbench/antennapod-sample-diana.json
```

Reuse the same generated manifest on the peer and on subsequent Diana builds.
Use `--shard 2/4` to select one nonoverlapping partition of a larger run.
Each partition is ordered deterministically and can contain up to 512 methods.
`--inspect-structure` records the parsed source CFG and recovery CFG for review.
Branch condition expressions and exception effects are outside this graph's
model. A visible CFG difference directs inspection and carries no failure
verdict. `review` preserves the CFG evidence alongside both backend outputs.
Standalone APK input with `run --input` supports member inventory and
publication checks. Capturing the input and source revision alone establishes
provenance. The corpus cases specify what quality evidence is measured.

The [public corpus workflow](../../.github/workflows/decbench-public.yml) is
manually dispatched for a selected catalog entry and evidence profile. Set the
repository variable `DECBENCH_PUBLIC_ENABLED` to `true` to enable the weekly
public matrix. It evaluates reviewed cases with the authored models and a
separate 100-method sample for each project. It keeps the previous compatible
Diana result as a regression baseline and uploads generated evidence.
The fixture-only weekly workflow runs without public APK downloads.

### Add a public corpus

Choose a specific APK release from the upstream publisher or a public
distributor. Match the release variant and version code to an immutable
upstream commit. Check the APK's embedded revision when available and record
the publisher or distributor's build evidence and the source license.

Add two JSON files under `quality/decbench/corpora/` and set `schema_version` to
`1` in both. Use the NewPipe entries as examples. [Project catalog](corpora/newpipe-0.28.8.json) ·
[Reviewed cases](corpora/newpipe-0.28.8.cases.json)

- In `<id>.json`, set `id` to the filename stem and provide `name`, `version`,
  `apk`, `source`, `source_commit`, `source_root`, and `provenance`. Each asset
  needs a version-specific HTTPS `url` and a `max_bytes` bound for the downloaded
  file. Use a gzip-compressed tar archive whose URL includes the full source
  commit and whose top-level directory matches `source_root`. Record the
  distribution, build evidence, and license under `provenance`. Include
  `source_repository_url` to enable discovery and pinned source links.
- In `<id>.cases.json`, record the signed APK's `input_sha256`, the matching
  `source_revision`, and `revision_entry` when the APK embeds that revision.
  Use the same digest in the catalog's optional `apk.sha256`. Each method needs its
  original physical `at` locator, complete `member` descriptor, `source`, and
  independent `obligations`. Give the source path relative to `source_root`,
  inclusive `start_line` and `end_line`, and the SHA-256 of the whole source
  file. Include a link to those lines at the pinned revision. Give each
  obligation a stable `id`, `kind`, and description. Call witnesses also list
  `targets` with `owner`, `name`, and `descriptor`. Argument-value witnesses
  identify zero-based `argument_indexes`. An optional `model` names an
  authored execution model relative to `quality/fixtures/`.

Use the independent DEX inventory to confirm the original locator and complete
descriptor. Check the selected source span and obligation against the original
instructions before looking at either decompiler output. The evaluator checks
the pinned APK and selected source bytes. Acquisition checks file bounds and
container formats. See the [file verification policy](../../docs/diana/CONTRIBUTING.md#file-verification).

```sh
python3 quality/decbench corpora
python3 quality/decbench prepare --corpus newpipe-0.28.8 \
  --apk /path/to/org.schabi.newpipe_1013.apk --source /path/to/NewPipe-source.tar.gz
PYTHONPATH=quality/decbench python3 - <<'PY'
from pathlib import Path
from dex_truth import apk_methods

apk = Path('.cache/decbench/public-corpora/newpipe-0.28.8/input.apk')
for at, member in apk_methods(apk):
    if member == 'Lorg/schabi/newpipe/util/Localization;->getDurationString(J)Ljava/lang/String;':
        print(at, member)
PY
python3 quality/decbench run --corpus newpipe-0.28.8 --at e4:l0:c7707:m56282 \
  --binary target/debug/diana
```

Track the manifests and authored evidence. Keep APKs, JADX JARs, source
archives, and generated reports in ignored `.cache/` storage. A catalog entry
is available for local use as soon as its assets are prepared. To add it to CI,
update the dispatch choices and scheduled corpus matrix in the
[public corpus workflow](../../.github/workflows/decbench-public.yml).

### Measure two builds on the same host

`timing` verifies the selected public cases, warms both executables, and then
alternates their order across matched trials. It records each exact original
method and complete inventory pairing. Incomplete measurements stay visible
with diagnostics. Medians include process startup, input loading, and JSON
publication. Use release builds on a quiet host for a performance comparison.
The report includes trial-to-trial variation and flags methods whose output
bytes differ across the paired runs.
The public workflow accepts an optional `timing_baseline` commit for a manual run.
It builds both release executables in isolated target directories on the same
runner and attaches the raw timings and HTML to the evidence artifact.

```sh
python3 quality/decbench timing --corpus antennapod-3.12.3 \
  --baseline /path/to/baseline/diana --candidate /path/to/candidate/diana \
  --repeats 5 --output .cache/decbench/antennapod-timing.json
python3 quality/decbench report .cache/decbench/antennapod-timing.json \
  --output .cache/decbench/antennapod-timing.html
```

## Corpus and evidence

- **Original member identity and descriptor.** An independent standalone DEX
  reader derives definitions and original method types from the DEX tables.
  It checks both missing and extra declarations against Diana's complete
  method inventory. A partial inventory cannot establish absence. The
  reader documents the supported DEX formats in its source.
- **Publication and recovery facts.** A fixture can require a particular
  original identity, readable recovery, diagnostic kind or range, and
  named facts in the selected publication. Expected partial recovery is a
  valid result when the exact independent requirements are met.
- **Behavior.** The authored DEX and JVM probes exercise branches, fault paths,
  results, exceptions, and ordered effects. The JVM matrix varies debug
  metadata at the same Java target. Execution is limited to known authored
  inputs whose bytes the probe verifies. It uses isolated scratch directories
  and timeouts. If recovered notation cannot be exercised by the probe, mark
  the scenario `UNCHECKED`. Output compilability does not determine a
  semantic verdict or the availability of production output.
- **Control-flow structure.** A parse-only JDK front end reads the authored
  Java reference and recovered publication. The benchmark compares normalized
  control roles and edges for branches, loops, and switch entries and
  fallthrough. The checker treats condition expressions as opaque and permits
  swapping the two arms of a binary test. Exception regions remain outside
  this CFG model. A graph difference
  is a structural finding. An unsupported or unparseable method stays
  `UNCHECKED`, including recovery notation outside the Java grammar.
- **Local variable types.** For JVM debug classes, the benchmark reads the
  original local-variable metadata and correlates it with a declared recovered
  binding or a unique returned local. Comparisons use original method descriptors
  and fixture-defined anchors. Missing debug metadata or a binding that cannot
  be uniquely established stays `UNCHECKED`.
- **Call bindings.** The call probe reads original invoke targets and resolves
  selected straight-line calls in recovered Java with the JDK symbol resolver.
  The report pairs the original bytecode position, owner, method name,
  descriptor and supported dispatch form with the binding in the recovered
  text. Ambiguous site correspondence or unresolved inheritance leaves the
  affected evidence `UNCHECKED`.
- **Field effects and value dependencies.** The effect probe compares authored
  Java and the recovered publication along supported straight-line paths. It
  records the ordered field reads and writes, which values reach writes and
  returns, and where the first difference occurs. The probe follows value
  origins through local assignments. Branches, unknown calls and unsupported
  aliases stay `UNCHECKED`.
- **Timing.** Reports record elapsed time and per-method CLI durations,
  including process overhead. Compare fresh runs with matching toolchains,
  worker counts and environments.

A method verdict summarizes its declared checks. Definition-only checks
establish original definitions and publication availability. Behavior
scenarios record actual and expected outcomes separately. A failed invocation,
unsupported case or missing oracle remains visible as incomplete evidence.

### Add an independently specified suite

Keep a version-1 `decbench*.json` manifest under an existing fixture directory,
beside its input, source and independent expectations. The manifest declares a
unique `id`, an `input` relative to the fixture or prepared corpus, and an array
of `methods` with original `at` and `member` fields. `prepared: true` selects the
prepared corpus. A committed input can declare `input_sha256`. Use
`oracle_files` to identify the independent source and scenario data. The
[guarded tail fixture](../fixtures/branches/mutable-tail-guards/decbench.json)
is an executable example.

A method can declare expected structured `issues` and observable publication
facts with `contains` or `omits`. Use those for actual product facts and scoped
presentation contracts. A fixture-owned `probe` supplies executable scenarios
for methods marked `behavior: true`. Each scenario supplies an ID, expected and
observed outcomes, and one of `PASS`, `FAIL`, `UNCHECKED` or `SETUP_ERROR`.
Qualify new behavior probes with an incorrect recovered program or an authentic
pre-fix revision that exposes the targeted discrepancy.

### Compare control structure and local types

A suite can configure `structure` with an authored Java `reference`, original
`class_at`, and `class_member`. Set `structure: true` on each selected method.
Control-flow evidence is categorical. The report keeps the original and
observed structures separate from any executed behavior checks.

A `local_types` suite declares the original class identity and optional
`type_aliases` for independently known reference types. In a selected method,
each local check names its original debug local by `name` and `slot`, with an
optional `start_pc` for reused slots. The recovered side uses a named local or
catch binding, or the unique `returned-local` role. See the
[JVM fixture manifests](../fixtures/jvm/source-recovery/) for examples.
The stripped-data control is opt-in and illustrates unavailable type evidence.

```sh
python3 quality/decbench run --binary target/debug/diana \
  --suite jvm-source-structure --suite dex-structure-branch-switch-packed
python3 quality/decbench run --binary target/debug/diana --suite jvm-local-types-debug
python3 quality/decbench queue --metric structure --status pending
python3 quality/decbench mine before.json after.json --metric local_types
```

`claim --metric structure` or `--metric local_types` leases a candidate in the
selected dimension. Structural findings and type evidence support a focused
review. Layout quality, names and general readability remain qualitative
judgments, and a structural finding does not establish a behavioral error.

### Inspect call bindings and field effects

Add a suite `call_bindings` configuration with the original `class_at` and
`class_member`. Set `call_bindings: true` on the methods to compare. For DEX
fixtures, the expected targets come from the original instructions, with the
original bytecode locations retained in the report.

An `effect_order` suite additionally names an independently authored Java
`reference`. Methods selected with `effect_order: true` use that source as the
oracle for ordered effects and data dependencies. The selected JVM class must
match the class rebuilt from the reference byte for byte. Set `variant` to
`stripped` for fixtures compiled without debug information. A DEX reference
requires a pinned input digest. For example,
[EffectSequences](../fixtures/jvm/source-recovery/EffectSequences.java) requires
saving the old value of `left` before writing the new one, then using that saved
value at the write to `right`.

```sh
python3 quality/decbench run --binary target/debug/diana \
  --suite dex-scalar-type-uses --suite dex-invoke-bindings --suite jvm-call-bindings
python3 quality/decbench run --binary target/debug/diana \
  --suite jvm-updated-field-effects --suite jvm-effect-sequences
python3 quality/decbench queue --metric call_bindings --status pending
python3 quality/decbench queue --metric effect_order --status pending
```

## Find, repair and verify

1. Run the corpus. Each independently failing method enters a persistent queue.
   A repeated observation retains its triage state. A later passing run clears
   the same input and oracle. A recurring failure reopens it. A changed oracle
   supersedes earlier queue evidence for the same original method.
2. Use `queue --status pending` and `claim` to take a case. `show ID` prints
   the expected and observed outcomes, original evidence paths, CLI output and a
   fresh reproduction command. A lease expires after one hour by default.
3. Confirm the cause and record triage with
   `resolve ID --claim-token TOKEN --status pending --group MECHANISM --note NOTE`.
   Group confirmed cases by the owning mechanism. Claims can target that group.
   Use `ignored` with a reason for reviewed non-actionable evidence.
4. Repair the general cause and run the affected Nextest regressions. Follow the
   fixture evolution loop for substantive changes to decompilation semantics or
   recovery. [Fixture evolution](../../docs/diana/EVOLUTION.md#primary-iteration)
5. Run the repaired executable on the same inputs. Compare reports to verify
   recovered checks and detect new failures, lost coverage and publication
   changes across the full scope.

For a public corpus case, replay the original method on a current Diana build
before changing the engine. The replay verifies the saved APK, source archive,
method identity and oracle first. The optional peer runs on the same evidence.
The receipt preserves the saved and fresh outcomes, including any checks that
can no longer be decided.

```sh
python3 quality/decbench triage --baseline .cache/decbench/antennapod-diana.json \
  --at e5:l0:c462:m3261 --binary target/release/diana \
  --output .cache/decbench/antennapod-triage.json
python3 quality/decbench report .cache/decbench/antennapod-triage.json \
  --output .cache/decbench/antennapod-triage.html
```

Supply the original generated manifest with `--cases` when it is outside the
run report directory. A campaign bundle keeps the manifest in the shard
directory with the producer reports.

```sh
python3 quality/decbench compare --baseline before.json \
  --candidate after.json --output changes.json --gate
python3 quality/decbench report changes.json --output changes.html
python3 quality/decbench index before.json after.json --output history.html
python3 quality/decbench mine before.json after.json --output findings.json
```

Report comparison aligns identical input bytes, oracle, original method
identity and expected check. Changes to the corpus or independent expectations
are reported as coverage changes. `--gate` rejects newly failing checks and
lost or incomparable evidence. `mine` uses reports in chronological order to
identify regressions, cases that a peer passes, and cases every measured
backend still fails. Supply `run --backend NAME` for a CLI-compatible adapter.
Every backend is independently evaluated before a peer difference becomes a
comparison signal.

### Run an independently evaluated peer

The optional [CFR adapter](https://www.benf.org/other/cfr/) evaluates the same
authored standalone JVM exception suites. Obtain CFR separately and create a
local launcher. The launcher records the JAR hash and requires a POSIX
environment and a JDK with `jdk.jdeps`.

```sh
python3 quality/decbench/cfr.py --jar /path/to/cfr-0.152.jar \
  --launcher .cache/decbench/cfr
python3 quality/decbench run --binary .cache/decbench/cfr --backend cfr \
  --suite jvm-exception-chains-debug --suite jvm-exception-chains-stripped \
  --output .cache/decbench/cfr.json
python3 quality/decbench mine .cache/decbench/current.json .cache/decbench/cfr.json
```

CFR supplies a full-class publication for the behavioral probes. Selected
method publication remains unchecked for CFR. Peer mining still compares the
scenarios executed with matching input and oracle.

### Automate a repair attempt

`cycle` leases a failure from a complete baseline report and hands it to an
explicitly configured local worker in a dedicated non-main worktree. The worker reads
JSON from stdin with the case, baseline report path and worktree. It returns an
object with `binary` pointing to its built Diana executable and optional
`revision` and `note` fields. Use a worker you trust to edit and build in that
worktree. The worker should treat recovered code as evidence only.

```sh
python3 quality/decbench cycle --baseline before.json \
  --worker /path/to/worker --worktree /path/to/repair-worktree
```

The cycle evaluates the proposed executable afresh over the baseline scope.
It accepts the benchmark attempt only if the leased checks recover, the
selected member passes and no other required check regresses or loses
evidence. It writes the candidate report, comparison and an attempt receipt
under the state directory. The next cycle can use the accepted report as its
baseline. Repository review, fixture challenges and integration follow the
normal Diana process.

Repeat `--guard-baseline PATH` to replay additional fixture or public corpus
reports before the attempt can be accepted. These reports must measure the
same original Diana executable as the primary baseline. Each guard reuses its
recorded selection and check settings. A guard can retain pre-existing failures
and must preserve the measured coverage. The attempt stores each guard report
and comparison.
Use `--prepared` when the public assets were moved. A copied campaign bundle
provides the adjacent manifest for each public report. Use `--cases` for a
separately stored primary manifest. Rerun a baseline with the current
evaluator when its fingerprint changes.

The [weekly workflow](../../.github/workflows/decbench.yml) evaluates the
default branch, compares against the previous complete evaluation, publishes
standalone HTML and raw evidence, and persists checkpoints and the triage queue
through the Actions cache. Local reports and generated corpora live under
ignored `.cache/` paths. Keep private inputs and reports in authorized local
storage. The repair worker follows Diana's normal integration policy.

## Reproduced comparisons

The guarded-tail fixture is a historical repair witness. On Diana
`2f7b1f13317b5b0df94bb848a92ebe53c2dcbea3`, the independent witness reports
224 mismatches across its 1,288 scenarios. Diana
`f4a8a477aecead75faf21bf7e4de991058aa86e5` passes all 1,288 on identical DEX
bytes. The comparison across the full DEX fixture selection records those
recoveries and no new failing checks or lost evidence. The executable witness
and its case table live with the [fixture](../fixtures/branches/mutable-tail-guards/).

The ordinary CLI can also evaluate the public APK documented in the
[AntennaPod source-aligned review](../reports/antennapod-3.12.3.md). With
`--inventory-only`, the benchmark establishes the original signatures and
physical identities of all 70,403 method definitions on that pinned input.

The [scalar-use fixture](../fixtures/atoms/scalar-use-views/) exercises overload
selection on the compiled reconstruction. Diana `6569b3a4` produces two concrete
mismatches. The carrier path returns 2,130 instead of 3,130, and the array path
returns `AA` instead of `A65`. The unmodified source oracle and both raw DEX
invoke targets explain the expected outcomes.

## Development checks

```sh
python3 -m unittest discover -s quality/decbench -v
```

The workflow follows the experiment → evaluation → per-method triage →
corpus sweep pattern from [DecBench](https://github.com/Noelo-Lab/decbench)
and [Kuna](https://github.com/Noelo-Lab/kuna/blob/main/docs/decbench-loop.md).
Diana evaluates original facts, recovery coverage and independently observed
behavior in the supported scenarios. Those results are useful for selecting
repairs and remain scoped to their actual evidence.
