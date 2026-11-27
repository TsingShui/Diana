# Mutable tail guard review

The first divergence in `route` occurs after the calls to nodes 0, 1 and
3 with seed and initial trace both set to −131074. Node 3 returns zero.
Its Smali jump leads to node 7. The older output set the same bypass flag
on both sides of the choice, ran nodes 9 and 11, and eventually returned
40. A test of the terminal integer alone therefore misses the error.
Selecting node 7 as the fault must throw after recording 0, 1, 3, 7.
Selecting node 9 instead must let this path continue.

In the repaired output the zero arm updates an exit flag. The following
guard reads that updated flag before it can write the node-7 bypass flag.
Consequently node 7 runs on the zero path and the nonzero path can enter
node 9 without first calling node 7. The same dependency appears at node
19. A result leading to node 23 must preserve its helper call and its
possible exception. The other arm can reach node 25 with no intervening
call to node 23. The drawing of these choices as nested flag assignments
does not change the order of the calls or their arguments.

The `extended` layout moves node 4 before node 1, adds a sixth group,
and changes which exits enter the next group. In the scenario starting
with seed −446630381 and trace 293007920, the route visits 0, 4, 6, 7,
8, 12, 13, 15, 24, 25, 27, 33, 35, 39, 40, 44, 45 and 47. The helper
result at node 35 selects node 39 on this path. Node 39 sends execution
to 40, then the selected arm reaches 47 and returns 48. The alternative
arm at 35 returns 48 immediately. On that arm a fault at 39 cannot
prevent the return. On the longer path a fault at 39 prevents the call at 40.
All calls still receive the original seed and fault selector. Neither
route catches an exception.

An independent DEX inspection checked the 176 conditional and jump
destinations, call operands and terminal return against the Java model.
The 28 scenarios collectively exercised both outcomes at all 88 decisions.
For each scenario the ordinary Engine regression tried every fault selector
from −1 through the terminal. The 1,288 resulting call sequences and exits
matched. Separately, executing the authored Java and the recovered Java on
OpenJDK gave the expected observations for all 1,288 cases. The pre-repair
recovery disagreed in 224 cases. These executions cover the listed inputs;
arbitrary combinations of helper results across all nodes remain unexercised.
