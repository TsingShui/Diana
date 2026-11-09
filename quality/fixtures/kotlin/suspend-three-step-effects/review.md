# Three-step source-body review

This LLM review compares the Java/Kotlin explanations, the same prepared Smali
input, and baseline `6178bdd` versus the acyclic-flow candidate. Written path
review is primary; origin/placement tests support it. No helper/scheduler or
final-output behavior interpreter is claimed as executed.

Both positive methods retain first loading, first record and the negative return
before second loading. State one checks the first result before conversion or
record. The nonnegative first is saved for state two; first + 1 remains the
second argument. State two checks the second result before conversion and record,
then retains the zero-result return before third loading. A modeled second zero
is reachable under the explicit helper override and returns the captured first.

The third route saves first/second in I$0/I$1, prepares their sum, and publishes
three before load. State three restores both original values and checks the
received result before conversion/record. The final addition uses these values.
Seed zero yields 0, 2, 4 and returns 6; seed two yields 4, 10, 28 and returns 42.
Earlier records remain before following calls. Load failure skips its own record
and later work, while record failure skips later calls.

The candidate retains original ABI dispatch/resume code as marked attachments
and delivers the source sequence, bindings, scalar copies and early returns.
It selects no other callee/argument. Guard comments retain both destinations,
with normal copies at the original guard position. privateReturns keeps three
distinct original marker returns; walk keeps its shared return. Selected P6
guards are consumed without recomposition or duplicate dominance. Nullable
Number casts retain original null/cast/dereference behavior.

normalOnly cannot share immediate-completion record(777) with the resumed tail.
The normal corridor contains that effect and fails the source-body proof. Its
complete explicit state machine, record and corresponding callback remain, so
the immediate/resumed effect difference stays visible.

No contradiction was found on the modeled paths. Original call/capture,
terminal and normal-transfer origins remain anchored. External field/constructor
resolution and actual scheduling remain unmodeled, not passing checks.
