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
