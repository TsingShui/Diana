# State observed after a shared finally

The original member is `Lfixture/SharedCleanupState;->run()V` at
`e0:l0:c0:m6`. The independently authored [Smali](SharedCleanupState.smali)
and [Java control](SharedCleanupState.java) isolate the state carried from
normal and abrupt cleanup into a single physical handler. The preceding
[split-handler fixture](../exception-state-snapshots/README.md) has distinct
outer entries and remains a compatibility check.

Scheduling at code unit 4 can fail before the wait or a refresh. A typed
interruption handler at 15 either rejoins the normal refresh or enters the
exceptional refresh. Normal refresh writes the new timestamp at 22 on
successful return. Exceptional refresh commits at 31. Reacquisition can then
fail with the fresh value already live. The normal cancellation at 36 and
abrupt cancellation and rethrow at 48 and 51 all reach the outer handler at
52 if they propagate a fault. The diagnostic at 53 receives the wide
value and the exact throwable before rethrowing it.

The [14 scenarios](scenarios.json) use initial timestamp 4294967400,
refreshed timestamp 8589934600, and diagnostic clock 8589934732. A failed
refresh preserves elapsed time 4294967332. A successful refresh changes it
to 132. A cleanup fault replaces a pending fault and leaves the committed
state intact. Each route reaching scheduling cancels once.

The prepared DEX was assembled with Apktool 2.11.1, OpenJDK 21.0.12.1 and
API 24 through `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`. Its SHA-256 is
`3f106fd354722d43dcce6693464dcc0d791e0dc2cda366c14ab045a8174ebc14`.
The original DEX path witness and Java control match all fourteen traces.
Diana 0.4.5 passes four and fails ten. Diana 0.4.6 passes all fourteen.
The [two-wide-value](../shared-cleanup-wide-state/README.md) and
[loop](../shared-cleanup-loop-state/README.md) challenges add independent
value commits and a repeated cleanup entry, respectively.

```sh
diana get-method classes.dex 'Lfixture/SharedCleanupState;->run()V' --format json
python3 quality/decbench run --suite dex-shared-cleanup-state --binary /path/to/diana
```
