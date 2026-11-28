# Diana DecBench

Diana DecBench evaluates a decompiler build against a reproducible corpus. It
records independent evidence for each original method, mines concrete failures,
and tracks a repair through a fresh evaluation and a regression sweep. The
[subproject](.) lives in the Diana repository and uses the ordinary Diana CLI.

## Run it

Python 3.11+, SQLite 3.35+, a JDK and a built Diana CLI are required. Build each
Diana revision in its own worktree. Run these commands from the repository root.

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
   the same input and oracle. A recurring failure reopens it. A changed oracle supersedes earlier queue
   evidence for the same original method.
2. Use `queue --status pending` and `claim` to take a case. `show ID` prints
   the expected and observed outcomes, original evidence paths, CLI output and a
   fresh reproduction command. A lease expires after one hour by default.
3. Confirm the cause and record triage with
   `resolve ID --claim-token TOKEN --status pending --group MECHANISM --note NOTE`.
   Group confirmed cases by the owning mechanism. Claims can target that group.
   Use `ignored` with a reason for reviewed non-actionable evidence.
4. Repair the general cause in a separate worktree. Apply the repository's
   [fixture evolution and review loop](../../docs/diana/EVOLUTION.md#primary-iteration),
   including a more complex challenge and applicable Nextest regressions.
5. Run the repaired executable on the same inputs. Compare reports to verify
   recovered checks and detect new failures, lost coverage and publication
   changes across the full scope.

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
explicitly configured local worker in a dedicated worktree. The worker reads
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
