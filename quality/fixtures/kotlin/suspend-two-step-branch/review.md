# Multi-site branch semantic review

## Acyclic source-body iteration

The new flow candidate compared with `6178bdd` keeps both calls and the existing
negative-return control in one source order. Each original result merge uses
its own normal/resume copy bundle; first is saved for the second reentry rather
than recomputed. The first record remains before the negative branch and the
second call, and the second record remains before addition/return. Immediate
marker branches and private resume code remain ABI attachments with original
targets. The call's one continuation occurrence is implicit; its binding and
other arguments remain original. Source local declarations now use Kotlin var
syntax, preserving nullable casts and Java-style ABI signature plus suspend.

The source body's results/effect order match the independent README paths,
including early negative return and failures at either load or record. Original
control/copy/exception facts are retained; no region recomposition or register
replay is used. The new three-step/shared-and-private-marker challenge extends
this same mechanism and rejects normal-only effects. This is LLM semantic
reading supported by structural tests, not executed helper/scheduler behavior.

## Prior ABI shared-suffix integration

State0 still checks failure, publishes label1 and calls load. Its marker branch
reaches only the physical return; normal completion copies the result inside
else before conversion/record. State1 checks failure before the same copy;
state2 restores first and checks failure before jumping past the first record,
negative branch and second call. The original invalid default still throws
before helper work. Seeds-1/0/2 retain the authored calls/results. An integration
draft wrongly qualified copy-only else as NoElse and was rejected. P7 now keeps
Both and the assignments within their original selected phases.

Compared the same prepared DEX on dd70d20 (engine-source identical to6178bdd)
and the shared-suffix candidate. Read authored Smali and existing independent
helper models against changed source paths; unchanged paths reuse the prior
review below. Original P2/P3/P4/P5 and copy/protection coordinates agree. The
[fresh copy-only challenge](../../branches/switch-shared-tails/review.md) covers
two real assignments, bypasses and wrapping. This is bounded LLM/product review,
not helper/scheduler or final-plan execution.

## Prior dispatch review

This LLM review compares the Java/Kotlin explanation, assembled Smali, and
ordinary Engine output. Discovery baseline is `64aaa04`; integrated engine
`dd70d20` supplies the shared P6/P7 raw-selector repair. Structural tests support this written judgement;
they do not execute native helpers or coroutine scheduling.

The baseline's default row printed no transfer before the case-zero label.
Reading that output as the displayed switch would enter initial helper work
for an invalid label, contradicting the input's invalid-state throw. The
candidate prints the original default target and its label before that throw.
Valid state-zero/one/two routes retain their original targets and work.

State zero checks the entry result, publishes one, and calls load(seed). A
marker returns before conversion or record. State one checks the received result
before reaching the same conversion/record. The negative first-result branch
returns boxed -1 after the first record and before any second call. Otherwise
the first int is saved in I$0, first + 1 is prepared, and label two precedes the
second load. State two restores first and checks second failure before the
second conversion/record and first + second return. Each original helper call
and its source identity remains present once in the owned result.

Under the README helper model, seed -1 gives load(-1), record(-2), then -1;
seed 0 gives 0 + 2 = 2; seed 2 gives 4 + 10 = 14. Either site's failure skips its
record/tail, and record failure skips later work. The provided callback's zero
seed is unused by both resumed paths. Marker returns do not execute the other
site's tail. No contradiction was found in these modeled paths.

The [shared-capture challenge](../suspend-default-shared-capture/review.md)
adds a live original seed and an effect before the default exception. Multisite
source folding is still unavailable; the complete explicit state machine is
retained. Unknown external constructor/resolution behavior remains outside the
helper model. Not run: runtime or final-plan behavioral execution.
