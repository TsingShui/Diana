# Monitor execution and declaration flags

This independently authored DEX distinguishes physical monitor operations
from the DEX `declared-synchronized` modifier. It also has a native
execution-synchronization control and two pairs of explicitly nested
monitor regions.

| Original method | DEX flags | Monitor-enter sites | Monitor-exit sites |
| --- | --- | ---: | ---: |
| `e0:l0:c0:m1` `metadataOnly()Z` | `0x20001` | 0 | 0 |
| `e0:l0:c0:m2` `nativeSync()V` | `0x121` | 0 | 0 |
| `e0:l0:c0:m3` `nested(Z)Z` | `0x1` | 2 | 4 |
| `e0:l0:c0:m4` `nestedNamed(Ljava/lang/Object;Z)V` | `0x1` | 2 | 4 |
| `e0:l0:c0:m5` `physical(Z)Z` | `0x20001` | 1 | 2 |

All five methods belong to `Lfixture/MonitorContract;`.
The two `0x20001` declarations carry `declared-synchronized` without an
implicit monitor. The metadata-only method reads
`Thread.holdsLock(this)` on a fresh, unlocked receiver and returns
false. The physical method enters the receiver monitor and releases it
once on normal return or on rethrow. The nested methods enter the same
monitor twice on each path and release it twice. One uses `this`;
the other uses an object parameter distinct from the receiver. Each
fault case rethrows the exact supplied exception.

The [DEX oracle](raw_proof.py) resolves the original method declarations,
checks access flags and monitor operands, decodes catch-all dispatch,
and replays normal and fault paths. The [Java observer](CheckMonitor.java)
records actual `Thread.holdsLock` results before, in and after the
recovered body. It also checks exact throwable identity. These runtime
observations cannot distinguish one reentrant acquisition from two.
The probe separately reads JVM `ACC_SYNCHRONIZED` and monitor sites with
Javap. The native control checks classfile flags without calling a
platform native implementation.

The [default suite](decbench.json) recompiles the exact complete class
publication with the unchanged observer and modeled hooks. The authored
Java and original DEX must agree before the probe can report a recovery
difference. The native model and Java disassembly are scoped to this
fixture's selected operations.

The [prepared DEX](classes.dex) was assembled with APKTool 2.11.1 and
API 24 using `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`. The SHA-256 is
`2f45fa2b44eb56719461c91fbf2fa8cc52bb1baf0dbc54334afd94dce9cd0055`.

Run the original controls and candidate suite from the repository root.

```sh
python3 -B quality/decbench run --suite dex-monitor-execution-contract --binary /path/to/diana
cargo nextest run -p diana -E 'test(recovered_jvm_monitor_scopes_match_execution_and_bytecode)'
```

The frozen 0.4.14 baseline has 23 passing and four
failing checks. The metadata-only method adds a synchronized modifier
and returns true. The physical method adds an implicit method monitor
while retaining its original explicit body monitor. The named nested
method publishes one explicit region for two original acquisitions.
The nested receiver and native controls match the authored expectations.
The 0.4.15 candidate passes all 27 checks. Its method headers and physical
monitor instructions agree with the independent observations.

The [JVM companion](jvm/README.md) covers implicit method locks, explicit
locks, reentry and exception release on complete class-file publications.
