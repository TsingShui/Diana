# Semantic review

I compared the Smali of `NestedJoins.run` with the ordinary method source
from the DAG dominance and branch-variable changes based on `7be3f1e`.
For each of the five groups, I traced the value on entry, the operand of
each call, the bypass, and the effect that feeds the next group.

The output tests bit 0 before evaluating either arm. Under that guard a
clear bit 1 selects `left`, and a set bit 1 selects `right`. Both calls
use the value that entered the stage and store their return in the same
carried variable. The separate bypass guard includes the negation of bit
0, so a set bit 2 cannot skip the join after either selected helper. With
bit 0 clear, a set bit 2 returns the result of `finish` on the original
carried value. A clear bit 2 continues to the join without an arm call.

For each of the first four stages the join executes after the early return
guard, consumes the selected or carried value, and supplies the next
stage. In the fifth stage the output tests the complementary condition:
bit 0 set or bit 2 clear. Only this condition executes its join. The final
unconditional `finish` therefore receives the last join result on an
ordinary completion and the incoming value on a fifth-stage early exit.
All logical shifts use the original key and advance three bits per group.
They preserve the low fifteen decisions with either sign of the key.

There is no catching instruction in the input or catch in the output.
A failing selected helper escapes before its assignment, a failing join
escapes before the next group, and a failing finish escapes after the
same preceding effects on both forms. Unselected helpers have no effects.
The earlier protected-join and multiword-diamond fixture tests still pass.

The ordinary candidate class source also passed a local executable
comparison against the independent Java explanation. The harness replaced
the four native declarations in both classes with the same deterministic,
argument-dependent helper model. For helper index `h` (left 0, right 1,
join 2, finish 3), a normal invocation returned
`(h + 17) * 37 + argument * 43` with Java integer wraparound. The
initial value was `key * 994009 + 557`. It tested keys from -32768 through
32767 with normal completion and injection at each of the first eleven
call positions, including positions unreachable on an early exit.
All 786,432 cases had identical ordered helper identities and argument
values, and identical results or propagation. The checks exercise the
declared helper model; the Smali retains its native declarations.
