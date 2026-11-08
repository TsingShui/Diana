# Fixture-driven evolution and regression

The primary improvement process is a fixture-driven iteration: discover a problem,
repair its general cause, challenge the repair with newly authored, more complex
fixtures, then confirm that earlier effects remain intact. Repairing the discovery
fixture alone does not complete an iteration. LLM semantic-equivalence review is
the primary review method; executed checks supply separate supporting evidence.

The root [AGENTS.md](../../AGENTS.md) owns execution authorization;
[README](README.md#regression-workflow-status) records implemented tooling.
A defined procedure is not an executed or passing check.

```text
Existing fixture or authorized real input
  → confirmed problem + independent expectations + pre-fix observation
  → owning-stage repair
  → newly authored, more complex same-mechanism fixtures
  → pre-fix/candidate comparison + LLM path-by-path semantic review
  → original fixture + prior effects + required regression checks
  → iteration decision → separately authorized integration
  → further discovery within the authorized task
```

## Primary iteration

| Step | Action | Required evidence before advancing |
|---|---|---|
| Discover | Inspect an existing fixture or an authorized real input. | Original member identity, command, actual output and the concrete obstruction. |
| Confirm | Distinguish a semantic defect, readability limitation, absent fact, wrong selector or faulty expectation. Reduce a real-input discovery to a focused fixture. | Original DEX evidence, applicable contracts and independently recorded results/effect order or targeted recovery criteria. |
| Repair | Change the general mechanism in its owning stage. | Producer/consumer obligations remain consistent; the original symptom improves without a case-name patch, alternate method chain or hidden fragments. |
| Challenge | After the repair, author more complex fixtures exercising the same cause and its interactions. | Prepared Java/Smali inputs, independent scenarios, the retained trigger and the added complexity dimension. |
| Review | Compare the same prepared challenge inputs on pre-fix and candidate revisions. Perform LLM path-by-path review of the original and challenge outputs. | Results, effects, exceptions, identities and targeted recovery agree with the input; improvement and uncertainty are explained, not inferred from surface resemblance. |
| Regress | Revisit earlier effects and run applicable implemented checks, including every admitted behavioral case. | The required old fixture set, compared paths, actual check outcomes and preserved recovery/coverage are explicit. |
| Decide | Apply the iteration decision below. | The repair generalizes to the challenge and preserves earlier effects, with evidence limits stated. |
| Integrate | Run applicable repository checks and obtain the required integration authorization. | The reviewed candidate and its fixture/review/check records; a completed iteration is not commit or merge permission. |

These responsibilities can be performed in separate sessions; they do not imply
permission to launch agents or run indefinitely. A Skill or role prompt SHOULD
link here rather than maintain another acceptance policy. The operator retains
integration authority.

Authors MUST record independent expectations before inspecting each fixture's
candidate output. A readability repair records the concrete recovery improvement;
it does not manufacture a behavioral mismatch to qualify the case. If an
implemented behavioral checker applies, contributors MUST demonstrate its
sensitivity on the pre-fix defect and run it on the candidate. The primary review
loop does not require an implemented behavioral executor; missing execution
remains unrun, not a pass. An input that does not reproduce the recorded
trigger cannot serve as a successful repair witness.

## Complexity challenge

Contributors MUST generate a new, more complex same-mechanism challenge after
each repair before declaring the iteration effective. Re-running only existing
fixtures or renaming/enlarging an input does not satisfy this step. Authors MUST
retain the causal trigger and add a relevant interaction, not unrelated bulk.
Examples include shared handlers combined with outer finally, fallthrough inside
a loop with selected-entry copies, or cleanup combined with return capture,
throwing close and monitor release.

Each challenge MUST identify its original discovery fixture, repaired invariant,
added interaction, original selectors and independent scenarios. Scenarios MUST
cover relevant normal and exceptional paths, helper arguments/results, effect
order and recovery boundaries. Authors MUST establish that a selected fault is
reachable, distinguish acquisition IDs from returned handles, and state helper
bodies or explicit models. Unmodeled behavior remains uncertain.

Use the [shared fixture layout](FIXTURES.md). Prepare each input once and inspect
the same DEX on both revisions through the ordinary Engine entry. A challenge
MUST demonstrate the repaired guarantee under its added interaction. A baseline
that already handles a case is a useful compatibility guard, but that case alone
does not demonstrate that the repair generalized. The comparison MUST explain
which targeted behavior or recovery changed and why it follows from the repair.

A same-cause failure keeps the current iteration open: contributors MUST revisit
the owning mechanism and recheck the original, challenge and prior cases. An
independent problem already present in the baseline can become a separately
recorded next discovery, with the different cause/invariant explained. A
candidate-induced regression blocks the current iteration even if its cause is
different. Contributors MUST NOT relabel a same-cause failure or a new regression
as unrelated to finish an iteration.

## Iteration decision

Contributors MUST record the required original, challenge and old fixture set.
It includes prior completed iterations' repair/challenge cases and all admitted
behavioral regression cases. Broader producer/consumer effects require a
correspondingly broader inspection set. Contributors MUST compare semantics,
available readable fragments, original identities, uncertainty and the previously
established recovery effects; absence of diagnostics alone is insufficient.

LLM review MUST compare the authored input/Smali and helper behavior against final
output path by path. Reviewers MUST separate semantic agreement, the targeted
readability/recovery improvement and unresolved paths. An explicit graph or
exception chain does not by itself establish a semantic defect. Existing
independently reasoned reviews can be reused for unchanged inputs and output;
reports MUST identify that reuse and its basis. Changed paths require renewed
review. Applicable automated checks still run on the candidate.

An iteration is effective only when:

- The original symptom is resolved and a newly generated complexity challenge
  demonstrates that the same repair remains effective under the added interaction.
- LLM semantic review finds no contradiction on the required paths; material
  uncertainty about those paths is resolved rather than counted as agreement.
- Earlier required effects, member/code availability and recovery coverage remain
  intact; expectations and case selection have not been weakened.
- Required implemented checks pass, no known deterministic failure is overridden,
  and applicable format/lint/build and architecture checks hold.

An unmet condition keeps the iteration open. Missing inputs, unsupported checking
semantics or unrun required checks have explicit evidence status, not a successful
skip. An effective iteration is a review decision with a stated scope, not proof
for all programs or a machine behavioral PASS. The executed behavioral-regression
criteria below remain separate; contributors MUST NOT promote a written judgment
to an execution result. Reviews go in each fixture's `review.md`; provenance,
independent scenarios, challenge relationships and actual execution records stay
in its README. Record original and candidate revisions and distinguish LLM,
structural and actually executed behavioral evidence.

## What the machine checks

The following sections define supplementary, test-owned behavioral checking.
They do not replace the primary complexity-challenge and LLM-review loop.

The subject is the owned `MethodCode` and its final
[`EmissionPlan`](../../crates/diana-decomp/src/p7/emission.rs), obtained through
the shared ordinary Engine entry. This is the schedule consumed by P9, not an
earlier CFG that still contains decisions the final output has omitted.

The independent expectation is a fixture scenario: inputs, initial state,
explicit environment assumptions, expected result and ordered observable effects.
The checker MUST compute the candidate's actual behavior before comparing it with
that expectation. It MUST NOT obtain expected values from candidate products.

The test-side executor MUST:

- Follow final control layout, statement placement, selected copy phases and
  expression evaluation order. `Fallthrough` follows the next action in that
  layout, not the original transfer's hidden destination.
- Honor omitted conditions and other final spelling decisions. It MUST NOT repair
  a missing branch by replaying retained P1/P2/P6 facts instead of the final plan.
- Preserve actual runtime casts and checks, including inline check carriers.
  Owner disclosure such as `via` adds no runtime check. A retained origin alone
  does not imply that its operation executes in the output.
- Record scenario-relevant values, calls with arguments, writes, exceptions and
  lock events in order. Matching a final return or a call count alone is
  insufficient when intermediate effects are required.
- Model fixture helper calls from declared inputs and state, never by returning
  the expected final answer. Unmodeled calls, operations or execution forms
  produce an unchecked result; they are not silently ignored or treated as no-ops.
- Keep interpretation independent of stage reconstruction decisions. Product
  access MUST NOT widen analysis access or construction authority, and the
  checker MUST NOT invoke a private stage or introduce another analysis route.

The executor and correspondence checks MUST compile only under `cfg(test)`.
Normal debug/release requests MUST NOT run them or expose a switch to enable them.
A check of an existing final plan is not a second decompilation implementation.

The executor's supported semantics are explicit and grow only with concrete
cases. The initial scope is integer values, local storage/copies, conditions,
sequences, labels/jumps and fixture-defined helper calls. Reports MUST identify
unsupported constructs and unexercised paths separately; a scenario pass does
not cover an unvisited path. A terminating or resource-bounded test observation
is not a proof for all inputs; a timeout or interrupted execution MUST NOT count
as a pass.

P7 plan acceptance does not establish P9 textual fidelity. A rendering change
requires separate evidence that the emitted text preserves the plan. Reports
MUST distinguish plan behavior from text rendering and MUST NOT label an
unchecked renderer as verified. No step requires Java legality or recompilation
of Diana's Java-like output.

## Checker implementation

The implementation is Rust test-only code under `crates/diana-decomp/tests/`,
not a CLI feature or a JSON interpreter over inspection-formatted Debug strings.
Inputs and independent scenario expectations stay in
`quality/fixtures/<theme>/<slug>/`. The planned file responsibilities are:

| File | Responsibility |
|---|---|
| `<case>.rs` | Resolve fixture inputs and original selectors, call the ordinary Engine entry, declare scenarios/helper behavior/independent observations, run checks and report outcomes. |
| `support/schedule.rs` | Translate final emission placement into a private executable instruction sequence with actual label locations. |
| `support/execute.rs` | Interpret that sequence and its expressions/copies, tracking values, helper calls and ordered effects. |

Cargo integration-test targets compile this code only for tests. Production
stage modules contain no fixture test implementations or checker entry points.
Shared helpers are added only for actual case needs; the executor files above
are planned responsibilities, not implemented capabilities. Fixture preparation uses the
[Smali procedure](FIXTURES.md#dex-preparation); the runner MUST NOT silently use a
stale DEX when assembly fails. A fresh ordinary Engine request provides the owned
`MethodResult`. `MethodResult::code()` and `MethodCode::emission()` are the checker
seam; existing read-only product queries provide supporting expressions, locals,
references and copies. No private stage constructor or new production export is
required by this design.

### Final-layout adapter

`schedule.rs` uses an explicit traversal of selected emission children and
continuations to produce evaluation, copy, branch, jump and exit instructions.
This is a test execution representation, not a replacement production IR. It
MUST retain a location back to the original member and relevant control/operation
for diagnostics. The adapter MUST use these rules:

- Materialize only the final executable placement. Composed expressions execute
  at their selected uses, not again at their retained original statement sites.
- Resolve jumps to labels at their displayed locations. A referenced but missing
  label is a check failure; do not invent its placement from an earlier graph.
- Lower `Fallthrough` to sequential execution. Lower an ordinary branch according
  to its actual emission: an omitted If display contributes no predicate or hidden route.
- Preserve selected-edge copies and before/effect/commit order. Do not execute
  both arms' copies or use the destination of an omitted transfer to repair it.
- Reject an execution form whose semantics are not implemented. Unknown forms
  do not become empty instructions. Cast/exception/lock support is admitted only
  with corresponding cases and explicit semantics.

The adapter interprets the final product independently; it MUST NOT call P6
selection or P7 composition helpers to decide what should have been emitted.

### Execution and comparison

`execute.rs` accepts the executable sequence, scenario arguments/initial state
and a fixture helper implementation. It returns an observation or an unchecked
reason. The executor MUST NOT receive the expected observation; `<case>.rs` compares
actual and expected only after execution. A scenario starts with fresh locals and
helper state, with explicit machine-width numeric behavior rather than host-sized
arithmetic or fabricated values for unread storage.

A helper such as `readCode()` returns the scenario's supplied value and records
its call. A helper such as `reportError(error, detail)` records the arguments
computed by the candidate. Neither helper decides which error the candidate
should select. Calls are bound by original owner/name/prototype, not generated
names, and an unmodeled call is unchecked. A test-host execution limit bounds
nontermination without adding an engine quota; exhaustion is not a pass.

An observation contains the normal/exceptional exit and its value, plus the
ordered effects relevant to the scenario. The report identifies the first
expected/actual divergence and its source location. A summary count does not
replace that trace. The required case list and scenarios are explicit, so missing
input, failed selection or an empty run cannot silently reduce the denominator.

### First implementation milestone

1. Prepare the shared-tail branch Java/Smali pair and confirm the pre-fix defect.
2. Record its input scenarios, result selection and ordered helper calls independently.
3. Implement the smallest complete execution subset needed by those scenarios.
4. Run the same checks on the pre-fix and repaired Engine revisions using one assembled input.
5. Demonstrate rejection of a missing/wrong branch and an extra/reordered helper call.
6. Admit the case only after all required scenarios pass on the repaired revision.

The first milestone exercises the integer/control/helper subset, not general
Android execution, arbitrary APKs or a Java VM. Qualification includes both the
real pre-fix plan (to exercise the adapter) and deliberately wrong test-owned
instruction sequences (to exercise the executor/comparator). Those negative
sequences do not grant authority to construct production stage products.

## Acceptance and regression

Each scenario receives one of four check outcomes. These are regression-report
terms, not new production failure kinds.

| Outcome | Meaning |
|---|---|
| `PASS` | The completed observation matches its independent result and required ordered effects. |
| `FAIL` | An observed result or effect contradicts an expectation; report expected and actual behavior and the first divergence. |
| `UNCHECKED` | The interpreter lacks required semantics, facts are unavailable, or the observation did not complete. |
| `SETUP_ERROR` | Input preparation, selection, build or check infrastructure prevented evaluation. |

A new behavioral regression case qualifies only when the same scenarios expose
the original behavioral defect before the repair and pass after it. The checker qualification also requires a
negative example that deletes or retargets the relevant branch, or changes the
required effect order, to be detected. A checker that merely accepts the intended
patch has not established this sensitivity.

After admission with its repair, a case remains in the required regression set.
Open, still-failing discoveries remain separate backlog items. Contributors MUST
run the entire admitted fixture set for integration, not only cases selected by
changed filenames or text differences. Local development MAY run a focused subset,
but its report MUST identify that subset and cannot authorize integration.

Acceptance for executed behavioral regression: the qualified behavioral repair
case, its admitted complexity challenges and every required scenario in the
admitted set return `PASS`, checker coverage has not decreased, and the discovery
symptom is resolved. The primary iteration decision and applicable repository
checks are additional requirements.
`FAIL`, `UNCHECKED`, `SETUP_ERROR`, an empty run or a missing required case prevents
regression acceptance. Aggregate percentages MUST NOT hide any such result.

A candidate that repairs A while breaking B is rejected; so is one that repairs B
while breaking A. The repair must satisfy their combined requirements. Authors
MUST NOT delete, skip, rename out of selection or weaken a case to obtain green.
If expectations genuinely conflict, record the conflict and independent evidence
for review instead of alternating accepted regressions. Correcting a faulty
expectation is an explicit reviewed specification change, not a baseline refresh.

When changing the executor, contributors MUST rerun its known negative examples
and the admitted set. Interpreter changes MUST NOT reset expectations or conceal
previously reported gaps. Reports identify the candidate revision, fixture
revision, checker revision, actual assembly environment, exercised scenarios and
outcomes; use original identities, not unstable local names or table indexes.

## Cost and evidence boundaries

Every iteration includes the complexity challenge and LLM semantic review.
Implemented deterministic checks run independently of the LLM, including the
entire admitted behavioral set for integration. Existing fixtures supply the main
discovery loop; authorized real inputs supply additional discovery and targeted
confirmation. Routine regression does not require a full APK corpus rerun.
Broader corpus runs remain useful for behavior not yet represented by fixtures.
Contributors MUST report that coverage limit rather than infer whole-program
correctness from the reviewed set.

LLM judgments are primary semantic-review evidence, not substitutes for executed
checks. Other decompilers assist investigation; neither their output nor an LLM judgment
can override a failed deterministic case. Output diffs are diagnostic aids, not the acceptance
standard and not a reason to replace fixed behavioral expectations.
