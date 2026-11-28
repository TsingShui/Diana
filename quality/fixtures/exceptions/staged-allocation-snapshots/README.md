# First-use class initialization and staged construction

The original subject is `Lfixture/AllocationStages;->run()V` at
`e0:l0:c1:m8`. The [caller Smali](AllocationStages.smali) and
[allocated class Smali](AllocationItem.smali) are independently authored
and determine the DEX input. The [Java reference](AllocationStages.java)
implements the native recording hooks and the same construction order.
This independently authored fixture extends the first-use ordering checked by
[Constructor preallocation order](../../atoms/constructor-allocation-order/README.md)
and the catch state in [Constructor trees with effects and handler state](../../atoms/constructor-argument-effects/README.md).
It adds a different live catch value at each stage and records object identity
through construction and publication.

The protected interval [2,24) reaches its catch-all at 25. It calls
`prepare()` at code unit 2, sets stage 15 at 6, and runs
`new-instance AllocationItem` at 8. On first use, `AllocationItem.<clinit>`
calls the recording hook `initialized()`. Only then does the caller
set stage 20 and evaluate the argument at 12. At 16 it sets stage 30.
The constructor at 18 receives the prepared value, the argument and
stage 30. Publication at 21 receives the initialized object. The catch
passes its live stage and captured throwable to `caught` at 26.

| Failing operation | Stage passed to the catch |
| --- | --- |
| Preparation before first use | 10 |
| First-use class initialization | 15 |
| Argument evaluation | 20 |
| Constructor or publication | 30 |

The [scenarios](scenarios.json) record exact events and throwable
identities from each modeled failing operation. The initialization
hook throws a unique `LinkageError` so it reaches the catch directly.
A second construction after successful initialization does not rerun
the initializer. A second use after failed initialization sees the
erroneous class and catches the VM's `NoClassDefFoundError` by type.
The model checks initialization and argument order before observing how
a stage fault prevents publication.

The DEX was prepared with Apktool 2.11.1 and OpenJDK 21.0.12.1 using
`SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`.
Its SHA-256 is
`ce6489ad2a4b6d7df23279fa9a6bef9daf3d1d2ef7ac7e75e241e39a6baae635`.
The independent DEX witness verifies the initializer and constructor bodies
and models class state on their first active use. The executable Java reference
agrees on all ten scenarios. The DecBench probe executes Java-shaped recovery
or interprets the selected method's explicit allocation and initialization
notation. The latter check compares the ordered effects and allocated receiver
against the original DEX witness with two sets of hook return values.

Diana 0.4.8 (`69e6cf11`) moves first-use initialization before preparation
and exposes stage 20 for all caught faults. DecBench executes that exact
published method and reproduces mismatches on all ten scenarios.
The 0.4.9 candidate passes all ten scenarios with the same prepared input.
Nine checker controls exercise the reference trace and deliberate changes
to the stage, receiver, initializer, fault type, and allocation count.
The ordinary Engine regression checks that the stage assignments and the
four operation boundaries remain in the protected body. [Semantic review](review.md)

```sh
fixture=quality/fixtures/exceptions/staged-allocation-snapshots
prepared=.cache/fixtures/staged-allocation-snapshots
mkdir -p "$prepared"
java -cp .cache/decbench-repair/smali/apktool.jar \
  .cache/decbench-repair/smali/Assemble.java "$fixture" "$prepared/classes.dex" 24
cmp "$prepared/classes.dex" "$fixture/classes.dex"
diana get-method "$fixture/classes.dex" 'Lfixture/AllocationStages;->run()V' --format json
python3 quality/decbench run --suite dex-allocation-first-use-order --binary /path/to/diana
PYTHONPATH=quality/decbench python3 -B "$fixture/test_explicit_recovery.py"
```
