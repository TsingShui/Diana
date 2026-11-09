# Effectful default complexity review

## Current shared-suffix integration

The actual default now owns record(-99) then construction/throw, preserving
that order and excluding logical load work. A record failure still precedes
allocation. Both resumed cases restore I$1 before failure checking; state2 also
restores I$0 before its later-tail goto. The live seed remains in normal/resumed
copies and final addition, retaining seed2 result16. Physical marker returns
skip conversions, records and later work. An integration draft emitted a false
copy at the if header and was rejected; the P7 late display guard now keeps
that assignment in its else arm.

Compared the same prepared DEX on dd70d20 (engine-source identical to6178bdd)
and the shared-suffix candidate. Read authored Smali and existing independent
helper models against changed source paths; unchanged paths reuse the prior
review below. Original P2/P3/P4/P5 and copy/protection coordinates agree. The
[fresh copy-only challenge](../../branches/switch-shared-tails/review.md) covers
two real assignments, bypasses and wrapping. This is bounded LLM/product review,
not helper/scheduler or final-plan execution.

## Prior dispatch review

This challenge was authored after the P7 dispatch repair. The same prepared DEX
is inspected on `64aaa04` and integrated engine `dd70d20`. The baseline prints an empty default
row before case zero; the candidate prints the original default goto and its
label. This LLM review compares both outputs with the authored sources, without
claiming helper/runtime execution.

For adjusted label seven, the input reaches record(-99), then the original
invalid-state exception. The baseline's displayed fallthrough would bypass that
effect and enter initial loading, so it fails this concrete expectation. The
candidate's explicit default transfer reaches record(-99) before construction
and throw. If that record throws, the later construction is skipped. No valid
state route reaches that default effect.

The initial route stores seed in I$1 before first loading. Both resumes recover
it; state two also recovers first from I$0. First completion is checked and
recorded before the negative branch or second load. Second completion is checked,
converted and recorded before first + second + seed. The callback's zero seed
does not replace the captured original. Seed 2 therefore yields 4 + 10 + 2 = 16
on immediate/resumed combinations; seed 0 yields 2, while seed -1 takes the
recorded negative branch and returns -1. Failures and marker returns skip exactly
the later work identified by README.

The repair generalizes from a pure throwing default to an effectful default and
shared capture. Existing state/copy/binding facts are preserved. No contradiction
was found under the helper model. This is supporting semantic review of an
explicit state machine, not a claim of full source-body recovery. Four fixture
checks support dispatch/origin availability. Not run: behavioral execution of
helpers, scheduling, or a final-plan interpreter.
