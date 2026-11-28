# State at an explicit cleanup rethrow

The original method is `Lfixture/BackoffNested;->run()V` at `e0:l0:c0:m6`.
The [Smali](BackoffNested.smali) and [Java control](BackoffNested.java)
are independently authored. They isolate the live-state transfer observed
around alarm cancellation in FairEmail 1.2338.

The initial clock at code unit 0 establishes the timestamp before scheduling.
Scheduling at 4 can fail before entering the refresh scope. The typed
interruption handler at 15 rejoins normal refresh. Faults from save, wait, or
interruption reporting can enter the exceptional refresh at 28. That copy
writes the timestamp at 31, reacquires at 32, then explicitly rethrows at 35
through a separate capture at 46. Those writes precede exceptional cancellation
at 48. Normal cancellation at 36 can also fail. Its catch at 52 and the abrupt
catch at 55 forward the selected exception to the diagnostic at 57.

The [14 scenarios](scenarios.json) check ordered effects and exception
identity, including cancellation replacement. The initial timestamp is
4294967400. Once refresh returns successfully, the timestamp is 8589934600.
The reporter at fixed clock 8589934732 observes elapsed time 4294967332
before a successful refresh, or 132 after it.

The DEX was assembled with Apktool 2.11.1, OpenJDK 21.0.12.1 and API 24
using `SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`.
Its SHA-256 is
`9649aaf23a0adb3f070ceebdbc0d56a2f64c458d08474c7d157b4b0a33738102`.
The DEX path witness and Java control agree on all 14 traces. Diana 0.4.4
fails three scenarios after the exceptional refresh and passes eleven.
Diana 0.4.5 passes all fourteen. The [complexity challenge](../split-exception-state-snapshots/README.md)
adds a wide register transfer at the joined outer handler.

```sh
diana get-method classes.dex 'Lfixture/BackoffNested;->run()V' --format json
python3 quality/decbench run --suite dex-exception-state-snapshots --binary /path/to/diana
```
