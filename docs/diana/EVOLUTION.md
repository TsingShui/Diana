# Fixture-driven evolution and regression

Use the fixture-driven loop for substantive changes to decompilation semantics
or recovery. These include changes to recovered control flow, values, effects,
exception handling, and the availability of readable code. For other work,
choose checks that exercise the behavior the change affects.

The root [AGENTS.md](../../AGENTS.md) owns contribution and integration rules.
[README](README.md#regression-workflow-status) records available tooling.

## Primary iteration

1. Reproduce the issue using an existing fixture or an authorized input. Record
   the original member, the command, observed output, and independent expected
   behavior. Reduce a real input to a focused fixture when needed.
2. Repair the mechanism in its owning module. Update affected producers and
   consumers together and preserve independently recoverable results.
3. Add a complexity challenge that exercises the changed mechanism under a
   relevant additional interaction.
4. Compare the original case and challenge on the baseline and candidate
   revisions. Review the input and output path by path and run the applicable
   independent checks.
5. Recheck prior cases based on the effect of the change. Record the outcome and
   the scope of the evidence.

Record fixture expectations before inspecting candidate output. A readability
change needs an independent criterion for improved recovery. When an existing
behavioral checker can observe the defect, show that it detects the pre-fix
failure and verify the repair on the candidate.

## Complexity challenge

For a substantive semantic or recovery change, authors MUST prepare a new case
that retains the trigger and adds a relevant interaction. For example, combine
a shared exception handler with an outer `finally`, or exercise a loop exit that
also carries a value. Include normal and exceptional paths where relevant and
record helper behavior, fault reachability, effect order, and recovery boundaries.

Follow [FIXTURES](FIXTURES.md) for the layout and records. Compare the same
prepared input on both revisions through the ordinary Engine entry when
reviewing Engine fixtures. Identify which result changed under the added
interaction and explain how it follows from the change. An existing case that
already behaves as expected is a compatibility check.

If the new case still exposes the same cause, revisit the repair and recheck
the original and affected prior cases. Track an independent issue already
present on the baseline separately. A regression caused by the candidate
blocks acceptance.

## Iteration decision

Review the source and results for the original, challenge, and affected prior
cases. Use LLM semantic review to compare authored input and helper behavior
with the final output path by path. Cover relevant results, effect order,
exceptions, original identities, recovery coverage, and any unresolved paths.
An existing independent review can be reused when the input and relevant output
are unchanged. Record which review was reused and the basis for reuse.

A substantive iteration is effective when the original issue is resolved, the
challenge shows that the mechanism handles the added interaction, and affected
prior cases preserve their established behavior and recovery. Resolve material
uncertainty on required paths and run the applicable implemented checks.

Put written reasoning in the fixture's `review.md`. Record provenance,
independent scenarios, revision and assembly details, and executed results in
its README. Identify written judgments, structural observations, and executed
behavioral checks separately. Record missing inputs and unsupported or unrun
checks with their actual status.

## Acceptance and regression

Select implemented behavioral checks by impact on the changed producers,
consumers, and invariants. Include affected admitted cases even when their
files are unchanged. Run the entire admitted set when a shared change can
affect it broadly, when its impact cannot be bounded, or when changing the
behavioral checker. For checker changes, rerun its negative examples as well.
For changes limited to CLI routing, GUI behavior, or documentation, choose
focused checks of that surface. Report the cases and paths the checks exercised.

A new behavioral regression case qualifies when the same scenarios expose the
defect on the baseline and pass on the candidate. Show that the checker detects
a relevant incorrect alternative, such as a changed branch target or reordered
effects. Preserve admitted cases and their independent expectations. Review
genuinely conflicting or incorrect expectations against the original input and
record the evidence before changing them.

| Outcome | Meaning |
|---|---|
| `PASS` | The completed observation matches the independent result and required ordered effects. |
| `FAIL` | The observation contradicts an expectation. Report the expected result and first divergence. |
| `UNCHECKED` | The checker lacks required semantics or facts, or the observation did not complete. |
| `SETUP_ERROR` | Input preparation, selection, build, or infrastructure prevented evaluation. |

For executed behavioral acceptance, the original repair scenario, its admitted
complexity challenges, and all required affected scenarios must pass without
reduced checker coverage. A failed required check blocks acceptance. A missing,
empty, or incomplete run leaves acceptance unverified. Preserve failures and
identify the scope of any focused run.

## Cost and evidence boundaries

Keep independent replay and correspondence checks test-only. Use ordinary
production entry points to obtain the products under test. Routine regression
uses focused fixtures and, when needed, authorized representative inputs.
Corpus coverage and written semantic review provide distinct evidence. Keep
unresolved behavior explicit and report failures from deterministic checks
even when the reviewed paths otherwise agree with expectations.
