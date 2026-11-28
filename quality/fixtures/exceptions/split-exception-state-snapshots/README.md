# State at a cleanup rethrow through a wide register move

The original method is `Lfixture/BackoffSplit;->run()V` at `e0:l0:c0:m6`.
The [Smali](BackoffSplit.smali) and [Java control](BackoffSplit.java) are
independently authored. This extends the [state-at-rethrow fixture](../exception-state-snapshots/README.md).
It keeps the same cleanup and adds a `move-wide` to a different register pair
at code unit 57, after the distinct normal and abrupt catch entries at 52
and 55 converge. The diagnostic at 58 observes the moved timestamp and the
selected exception. The earlier refresh still writes the live input before
its retained rethrow.

The [14 scenarios](scenarios.json) check the same observable behavior with
this additional dataflow. Old state before refresh is 4294967400. A successful
refresh commits 8589934600. Cancellation runs once on normal and fault paths
that reached scheduling. If it throws, the new exception replaces any pending
one without changing the timestamp.

The DEX was assembled with Apktool 2.11.1, OpenJDK 21.0.12.1 and API 24
using `SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`.
Its SHA-256 is
`e0433703590ac1146258850ecd5654883764ddf9610d576b9f75505a60c43965`.
The DEX path witness and Java control agree on all 14 traces. Diana 0.4.4
passes eleven and fails the three paths that refresh successfully before
rethrowing the pending work failure. Diana 0.4.5 passes all fourteen.

```sh
diana get-method classes.dex 'Lfixture/BackoffSplit;->run()V' --format json
python3 quality/decbench run --suite dex-split-exception-state-snapshots --binary /path/to/diana
```
