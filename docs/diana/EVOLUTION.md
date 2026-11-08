# Improvement and regression workflow

This document fixes the agreed discovery, reduction, repair and acceptance
procedure. The root [AGENTS.md](../../AGENTS.md) owns execution authorization;
[README](README.md#regression-workflow-status) records what is implemented and
which steps are active. A defined procedure is not an executed or passing check.

```text
Authorized APK + concrete task
  → recorded symptom → confirmed input evidence
  → Java explanation + Smali reproduction + independent expectations
  → qualified behavioral check → owning-stage repair
  → new case and all admitted regression cases pass
  → authorized integration → further discovery
```

## One problem through the workflow

| Step | Action | Required evidence before advancing |
|---|---|---|
| Discover | Use Diana for a concrete analysis task on an authorized input. | Original member identity, command, actual output and the obstruction it caused. |
| Confirm | Distinguish a Diana defect from a wrong selector, absent input fact or different notation. | Original DEX evidence and the relevant product contract; reference-tool agreement alone is insufficient. |
| Reduce | Author the [Java/Smali pair](FIXTURES.md) and independent scenarios. | The small DEX retains the triggering structure and exhibits the same defect. |
| Qualify the check | Evaluate the pre-fix final output plan against those scenarios. | A behavioral mismatch, not a build error or missing-input failure, catches the known defect. |
| Repair | Change the general mechanism in its owning stage. | Producer/consumer obligations remain consistent; no case-name patch or alternate method chain. |
| Regress | Evaluate the candidate against the new case and every admitted case. | Required scenarios pass without weakening expectations or losing coverage. |
| Integrate | Revisit the discovery case and run applicable repository checks. | A reviewed candidate with explicit evidence limits and separate commit/merge authorization. |

These responsibilities can be performed in separate sessions; they do not imply
permission to launch agents. A Skill or role prompt SHOULD link to this procedure,
not maintain a second acceptance policy. The operator retains integration authority.

Authors MUST record the expectation before using the repaired output to judge it.
A non-reproducing reduction stays a discovery item, not an accepted regression
case. Authors MUST report unavailable steps rather than substitute a manual
observation for a machine pass.

## What the machine checks

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

A new case qualifies only when the same scenarios expose the original defect
before the repair and pass after it. The checker qualification also requires a
negative example that deletes or retargets the relevant branch, or changes the
required effect order, to be detected. A checker that merely accepts the intended
patch has not established this sensitivity.

After admission with its repair, a case remains in the required regression set.
Open, still-failing discoveries remain separate backlog items. Contributors MUST
run the entire admitted fixture set for integration, not only cases selected by
changed filenames or text differences. Local development MAY run a focused subset,
but its report MUST identify that subset and cannot authorize integration.

Acceptance: the new repair case and every required scenario in the admitted set
return `PASS`, checker coverage has not decreased, the discovery symptom is
resolved, and the applicable format/lint/build and architecture checks hold.
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

Every integration candidate runs the small admitted fixture set without an LLM.
Real APKs supply discovery and targeted repair confirmation; routine regression
does not require a full APK corpus rerun. Broader corpus runs remain useful for
finding behavior not yet represented by fixtures. Contributors MUST report that
coverage limit rather than infer whole-program correctness from the small set.

LLMs and other decompilers can assist discovery, reduction and investigation of
uncovered changes. Their similarity scores or judgments MUST NOT override a
failed deterministic case. Output diffs are diagnostic aids, not the acceptance
standard and not a reason to replace fixed behavioral expectations.
