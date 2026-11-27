# Semantic review

In `run()`, `begin()` first stores `1` in `trace`. The original helper appends
`2`, multiplies its returned value by `2`, appends `3`, and adds that returned
value. The replacement evaluates those operations in the same order. Both
callers then pass `7` to `end()` and return it. The trace is `1237`.

In `forwarded(4, 10)`, `begin()` first stores `1`. The original JVM caller
loads `start` from local 1 and `base` from locals 2–3. The static helper reads
those values from local 0 and locals 1–2 respectively. The replacement maps
the reads to the caller's original slots and preserves the int-to-long
conversion, multiplication and addition. Both forms append `4`, then pass
`24` to `end()` and return `24L`. The trace is `164`.

`changedArguments(3, 8)` demonstrates why equal opcode sequences and matching
argument types are insufficient. The original appends `3` and computes
`3 + 8 * 2 = 19`. The replacement appends `8` and computes `8 + 3 * 2 = 14`.
The traces are `149` and `194`. `unrelated()` acquires the sequence from
`noArguments()` without any old invocation or surrounding effect matches.
It changes its result from `12` to `7` and its trace from `0` to `23`.

On the normal `protectedCall()` path, both forms append `4`, append `6`, then
pass `18` to `end()`. On the forced fault path, `effect(4)` appends `4` and
throws. Both handlers return `-1`, the trace stops at `14`, and neither form
calls `effect(6)` or `end()`. These execution checks cover the fixture handler.
The conservative fragment comparison leaves that covered region without an
inline correspondence. Calls with handlers, branches, helper-private locals,
unproven argument origins, and unknown operation facts require further
correspondence evidence. The fixture observes the bodies' arithmetic, effect
order and control transfers. It leaves stack inspection and external linkage
behavior unreviewed.
