# Protected scalar-join review

## Current verdict

The fresh challenge retains C1 with a word Phi from a char-array load or char
call, inside a loop with recovery, result snapshots and cleanup. The baseline
already discloses the char method signature, but drops the supplied use view.
The candidate states that view without changing the Phi, snapshot, call or
handler products. A composed producer is also enclosed once. Numeric-only and
unproved-range guards remain unchanged.

### Semantics

Bounded LLM reading under the README helper/fault model; no execution result is
claimed. P2–P8 products agree exactly between baseline and this rendering change.

| Method | Judgment and reasoning |
|---|---|
| `route([CIZ)I` | ~ Nonpositive limit returns0 before either producer. The selected array/unit path runs before try, so producer failure skips cleanup. Char and int record calls consume the same selected word, with only the char use viewed as char. The char result commits before the int call. Snapshots save the old total before char failure and the partially updated total before int failure. Recovery receives that saved value; normal cleanup or recovery cleanup precedes the original row/carry update. |
| `composed(I)I` | ~ unit executes once before record; mark11 precedes mark1066 for input1. The view surrounds the composed call but not another execution. The unit Operation span is inside the enclosing read-expression span, without consuming the view suffix. A throwing unit skips record. |
| `raw([CZ)I` | ~ Two original selected array reads remain; the chosen numeric result is65/66 for A/B. The branch Phi coalesces, so no executable copy is required here. No char view leaks into the int return. Actual executable join/snapshot copies are covered by route. |
| `unproved(I)I` | ? Out-of-range argument ABI behavior is intentionally unmodeled. The int source has no proved char range: its read remains a word, with original char signature disclosure, no new view/conversion. For modeled input65, char/int helper calls still give3130 in order. |
| `unit(I)C` | ~ Mark(10+index) precedes add65 and the one original int-to-char operation. The extra result view preserves that code unit without introducing another conversion instruction. |
| `record(C)I` | ~ Adds1000, marks that result, then returns it. A throwing mark prevents the result commit in its caller. |
| `record(I)I` | ~ Adds2000 and retains the same mark-before-return behavior on the int path. |
| `mark(I)V` | ? Native implementation is not executed; its ordered trace/fault behavior is explicitly modeled, not inferred by Diana. |

For the two-row normal cases, row0 contributes3130 and row1 contributes3132,
with cleanup70/71 before each increment, giving6262. Failure1065 reaches recovery
with total0; failure2065 reaches it with total1065, hence3132/4197 after row1.
A recovery failure reaches cleanup and then rethrows that failure; a cleanup
failure replaces it and bypasses row advancement. A nonmatching Error skips the
typed recovery but still executes original catch-all cleanup once. The normal
cleanup calls are outside that catch-all, so their failure does not repeat them.

The first test draft wrongly required an executable copy for raw's coalesced
Phi. That assertion was replaced by the actual two-load/numeric-return obligation;
route's executable-copy and snapshot requirements remain. This was a test
assumption correction, not a source expectation or production repair change.

The final baseline run has six passes/seven failures across the full13 tests;
this challenge's view and composition checks are among the failures. Candidate
checks pass. This is product/text sensitivity, not behavioral execution.

## Rounds

- Fresh challenge after the parent repair: loop, producer merge, typed recovery,
  partial commits, finally, composition and unproved-view guards.
