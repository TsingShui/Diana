# Repeated backoff cleanup with a live wide timestamp

The original member is `Lfixture/BackoffRepeated;->run()V` at
`e0:l0:c0:m8`. It extends the independently authored
[shared cleanup fixture](../shared-cleanup-state/README.md) with one loop that reuses the same wide timestamp registers
and physical handler entries. The [Smali](BackoffRepeated.smali) defines
the DEX and the [Java reference](BackoffRepeated.java) supplies modeled
native hooks.

Each iteration enters at code unit 0 and commits its own initial
timestamp after the clock call at 6. Normal and abrupt cancellation
at 42 and 54 both reach the same outer capture at 58 on a fault or
rethrow. The diagnostic observes the live timestamp and the exact
exception before rethrowing. A separate per-iteration catcher at 63
records the delivered throwable and goes to the same loop header.
Normal cancellation also resumes there. An initial clock failure
goes directly to the per-iteration catcher before scheduling.

The [scenarios](scenarios.json) use two rounds with different clock
values and distinct fault objects. The late-fault first round observes
8589934600. The next early-fault round observes its new initial
4294967656. The first round's value and fallback zero would both be
incorrect for the second round. A cancellation fault may replace the
pending throwable without changing the timestamp. The ordinary path
control verifies both rounds enter and cancel once.

The DEX was assembled with Apktool 2.11.1, OpenJDK 21.0.12.1 and API 24
through `SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`.
The independent DEX witness and Java reference agree on all seven
scenarios. Immutable Diana 0.4.4 and 0.4.5 both pass the ordinary two-round
control and fail the six diagnostic scenarios. They preserve the
reported throwable identities and the iteration boundary.

Diana 0.4.6 passes all seven behavioral scenarios.

```sh
diana get-method classes.dex 'Lfixture/BackoffRepeated;->run()V' --format json
python3 quality/decbench run --suite dex-shared-cleanup-loop-state --binary /path/to/diana
```
