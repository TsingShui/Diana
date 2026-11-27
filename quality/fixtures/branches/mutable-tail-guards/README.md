# Mutable tail guards

These independently authored Diana fixtures use the repository license.
`Lfixture/MutableGuardRoutes;->route(II)I` crosses five groups of choices.
`extended(II)I` adds a sixth group, changes which branches reach the next
group, and permutes the physical order of the first group. The Java
explanation expresses both routes as a loop. Smali unrolls the loop, with
an explicit branch and jump for each pair of successors.

Both methods start at node 0. Each visited node makes exactly one call to
`Lfixture/GuardTrace;->step(III)I` with the original seed, fault selector
and current node. Zero follows the jump and any nonzero result follows
the conditional branch. The group rules in `MutableGuardRoutes.java`
specify the next node. They include short paths to the terminal, joins
within the group and entries into both an adjacent group and a later one.
The terminal returns 40 or 48 without calling the helper. Any helper
exception escapes after the call and before another node can run.

`scenarios.tsv` records independent expected call arguments for 14 seed
and initial-trace pairs per method. For each call the helper records the
node and updates its signed 32-bit trace with `trace * 31 + node`. It throws
when the node equals the fault selector. Otherwise it returns
`((seed >>> (node % 27)) ^ trace ^ (node * 13)) & 1`. A fault on the
chosen path truncates the listed trace immediately after that node.
Selecting a node outside the path leaves the trace and terminal return
intact. These scenarios take both successors of every decision in each
method.

The prepared DEX was assembled at API 24 with Smali 3.0.9 and OpenJDK
21.0.12.1. The Engine regression uses the prepared input and executes the
emitted local assignments, short-circuit conditions, calls and returns
against the table.

```sh
fixture=quality/fixtures/branches/mutable-tail-guards
work=.cache/fixtures/mutable-tail-guards
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble \
  --api 24 -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
diana get-class "$fixture/classes.dex" 'Lfixture/MutableGuardRoutes;'
cargo nextest run --locked -p diana-decomp --test mutable_tail_guards_test
```
