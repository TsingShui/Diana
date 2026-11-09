# Multi-site branch semantic review

This LLM review compares the Java/Kotlin explanation, assembled Smali, and
ordinary Engine output. Engine baseline is `64aaa04`; the candidate adds the
P7 dispatch-spelling repair. Structural tests support this written judgement;
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
