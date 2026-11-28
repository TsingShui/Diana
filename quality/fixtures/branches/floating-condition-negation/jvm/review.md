# JVM floating condition negation review

The strict ordered comparisons proceed into the long arm only on their
specified order. `fcmpg` and `dcmpg` use +1 for unordered input, so `ifge`
takes the short arm for any NaN. `fcmpl` and `dcmpl` use -1, so `ifle` takes
it for the greater-than forms. The same edges select the short arm for
finite equality. Each short arm records 9 before return.

In the joined void methods, frozen Diana 0.4.19 rotates the smaller arm
into an early return. Its recovered early guards use `>=` or `<=`. This
keeps every finite trace and changes unordered inputs from 9 to 12345.
`gated` has an extra observable `open` call on the long comparison's
fallthrough. The baseline reaches that call on unordered inputs, recording
`812345` if it returns true or `89` if it returns false. The original skips
that call for either gate argument. All three NaN placements reproduce.

The baseline's separate physical return control keeps the Boolean
negation and all its traces agree with the original. Its integral joined
control agrees on both orderings and equality, where the complement is
represented by an ordinary integral `>=`.

`selfEqual` loads the same double twice and compares it with `dcmpl`. A
finite double yields zero and returns 9. NaN yields -1 and `ifne` goes to
the result 1. The baseline recovers an unconditional 9 because it treats
repeated operands as necessarily equal.

The repaired publication keeps an explicit Boolean negation on each
floating guard and a compact relational complement on the integral control.
The double self-equality result again distinguishes unordered inputs with
`value != value` as the early-return condition. Recompilation and execution
of the exact repaired class match every original trace, including the skipped
`open` call for every unordered input.
