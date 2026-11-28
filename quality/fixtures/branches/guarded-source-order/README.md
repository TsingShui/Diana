# Guarded sources evaluated in order

These independently authored methods keep a first guarded effect, a later
guarded effect, and their shared value consumer under an interruption handler.
`collectAfterOpening` adds a state change. Its first effect can enable the
later guard during the same method call. The repository license applies.
[Motivating source-aligned case](../../../reports/antennapod-3.12.3.md#source-first-review)

| Locator | Original member |
| --- | --- |
| `e0:l0:c2:m13` | `Lfixture/GuardedSourceOrder;->collect(Ljava/lang/Object;I)Ljava/lang/Object;` |
| `e0:l0:c2:m14` | `Lfixture/GuardedSourceOrder;->collectAfterOpening(Ljava/lang/Object;I)Ljava/lang/Object;` |

`GuardedSourceOrder.java` describes the behavior and provides the independent
execution controls. The Smali files define the tested DEX. The fixture records
effects in `Effects.log`, the returned value and the final `remote` flag.
Each scenario starts with an empty trace and fresh input. The selected subtype
can enable the first guard, the second guard, both guards, or neither.
[Scenario inputs and expectations](scenarios.json)

`P` records `prepare`, `O` records `open`, `C` records `confirm`, and `F`
records the final source. Each merge emits `M`. The catch path emits `X` and
returns null. `open` throws for negative mode before mutating state. Otherwise
it enables the second source. `confirm` throws for positive mode.

| Selected scenario | Authored trace | Diana 0.4.2 trace |
| --- | --- | --- |
| Both sources enabled | `PCFMM` | `CPFMM` |
| Second source interrupted | `PCX` | `CX` |
| First source enables second | `OCFMM` | `OFMM` |
| Both sources already enabled in the challenge | `OCFMM` | `COFMM` |
| First source interrupted in the challenge | `OX` | `COX` |
| Newly enabled second source interrupted | `OCX` | `OFMM` |

The other five scenarios pass on Diana 0.4.2. Diana 0.4.1 gives identical
observations across all eleven cases. The opt-in DecBench suite
compiles each selected publication with the authored helpers, whose behavior
defines the trace. An unavailable or uncompilable publication remains
unchecked for execution.

## Preparation and bytecode evidence

OpenJDK 21.0.12.1 compiled the source as Java 8. D8 9.5.33 produced the initial
DEX at API 24. Baksmali 3.0.10 disassembled it without debug directives into
the three Smali files. Smali 3.0.10 assembled the checked-in `classes.dex`.

```sh
fixture=quality/fixtures/branches/guarded-source-order
work=.cache/fixtures/guarded-source-order
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble \
  --api 24 -j 1 -o "$work/rebuilt.dex" "$fixture"
cmp "$work/rebuilt.dex" "$fixture/classes.dex"
python3 quality/decbench run --suite dex-guarded-source-order --binary target/debug/diana
```

Baksmali `--code-offsets` reported these code-unit positions for both methods.
The recovered column comes from compiling Diana 0.4.2's class publication with
the same OpenJDK and D8 settings into a separate scratch DEX.

| Operation | Original PC | Recompiled recovery PC |
| --- | --- | --- |
| First effect `prepare` or `open` | `0x0e` | `0x2a` |
| First read of the second guard's key | `0x14` | `0x0c` |
| Second guard's emptiness check | `0x18` | `0x10` |
| Guarded `confirm` | `0x22` | `0x1a` |
| Final source `finish` | `0x2b` | `0x30` |

The executed traces and bytecode positions record the open regression on the
stock executable. [Semantic review](review.md)
