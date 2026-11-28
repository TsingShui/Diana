# Backoff cleanup with two independent wide states

The method is `Lfixture/BackoffTwoWide;->run()V` at `e0:l0:c0:m7`.
This independently authored challenge extends the [shared cleanup fixture](../shared-cleanup-state/README.md)
with a second live wide value and an observable normal completion.
The [Smali](BackoffTwoWide.smali) is the DEX input. The
[Java reference](BackoffTwoWide.java) supplies recording native-hook models.

The timestamp uses v0/v1 and the marker uses v10/v11. Both are live in
the outer diagnostic. Scheduling at code unit 9 precedes any update.
A successful save at 13 commits the middle marker before the wait at 21.
Normal refresh at 29 commits the new timestamp at 32. Reacquisition at
33 commits the late marker only when it returns. The exceptional refresh
copy at 39 has the same effect sequence. The normal cancellation at 48
precedes completion at 51, which records both committed values and can
throw. The abrupt cancellation at 63 precedes rethrow at 66. The distinct
protected ranges [48,54) and [63,67) target one physical outer catch at
67. Its diagnostic at 68 receives both values and the actual exception.

The [eleven scenarios](scenarios.json) specify exact call order, values
and fault identities. Before scheduling the timestamp and marker are
4294967400 and 12884901992. After save the marker becomes 17179869364.
Successful refresh changes the timestamp to 8589934600. Successful
reacquisition changes the marker to 21474836620. If completion throws
after cancellation, the outer diagnostic observes both final values and
no second cancellation occurs. Cancellation or refresh failures may
replace a pending exception while leaving the most recent committed
values intact.

Apktool 2.11.1 and OpenJDK 21.0.12.1 assembled the input through
`SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`.
The independent original DEX witness and the Java reference agree on
all eleven cases. Immutable Diana 0.4.5 passes both ordinary completion
cases. All nine diagnostic traces have a zero or stale timestamp. The
marker, cleanup count and exception identity agree in those traces.

Diana 0.4.6 passes all eleven behavioral scenarios.

```sh
diana get-method classes.dex 'Lfixture/BackoffTwoWide;->run()V' --format json
python3 quality/decbench run --suite dex-shared-cleanup-wide-state --binary /path/to/diana
```
