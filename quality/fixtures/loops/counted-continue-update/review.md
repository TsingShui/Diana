# Counted continuation review

In the original `stopAt`, a successful threshold check skips the update after
writing the index. The continuing branch increments once and repeats the
bounds test. The frozen `while` has those paths. The repaired `for` retains the
same schedule because `break` skips the update clause. Exact recompilation of
the original fixture's authored Java and both Diana publications agrees on
seven observations, including the initial null fault and a miss after a
previous successful state write.

`advanceThenStop` adds a second decision on the repeating arm. A zero value
skips the recorder, then advances. A nonzero value at or below the threshold
calls the recorder with the old index before advancing. The recorder writes
its two fields and may throw the exact supplied exception. An escaping
exception or a successful threshold break bypasses the update. The frozen
publication expresses the backedge with an explicit `continue` after the
increment. The repaired counted `for` runs the update only when the body
finishes normally. Both publications give the original results, state and
exception identity.

In `holdOnZero`, the write and backedge re-read the same element. Advancing
on that path would skip a call to the recorder at the changed element.
The positive path records then advances, and a negative value stops before
either operation. Both publications keep the separate continuation behavior.

In `observeAfterAdvance`, the conditional recorder receives the already
incremented index. On the first visited nonzero element, the exact supplied
fault can therefore escape after both writes. Both publications preserve
these effects and retain a `while`. They express the next index with a
separate value and copy it on the two repeating paths. The behavior gate
covers the original value and fault timing.

The bounded original replay, authored Java and exact Diana publications
agree on all twelve challenge observations. The structural graph check
confirms the counted recovery and the unchanged `holdOnZero` control.
