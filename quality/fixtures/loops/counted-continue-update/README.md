# Counted updates before a loop continuation

This independently authored fixture extends the [`stopAt` case](../loop-for/review.md).
In the original case, a threshold hit writes the current index and breaks.
The other branch advances the index and returns to the loop test. The
challenge adds a conditional, observable call before the induction update.
That call can throw the exact supplied exception. Two controls exercise
transfers that require the induction update to stay inside the loop body.

| Original member in `Lfixture/CountedContinueUpdate;` | Locator | Original update path |
| --- | --- | --- |
| `advanceThenStop([IILjava/lang/ArithmeticException;)I` | `e0:l0:c0:m0` | A threshold hit writes `state` and breaks. Otherwise, call the recorder for nonzero values before the update at code unit 16 and the backedge at 18. |
| `holdOnZero([I)I` | `e0:l0:c0:m1` | The zero branch writes `1` into the current element and takes the backedge at 11 with the same index. A positive value records and advances at 21. A negative value writes `state` and breaks. |
| `observeAfterAdvance([ILjava/lang/ArithmeticException;)I` | `e0:l0:c0:m2` | After the negative-value exit, the update at 11 precedes the zero test at 13. The call at 15 receives the newly incremented index and can throw before the backedge at 18. |

The recorder `record(IILjava/lang/ArithmeticException;)V` at `e0:l0:c0:m3`
writes `events = (events * 31 + index + 1) * 31 + value` with signed 32-bit
arithmetic, then writes `lastValue = value`. At index 1 it throws the same
supplied exception if present. Each scenario starts with `state = 17`,
`events = 3`, and `lastValue = -99`. The observer records the return or
exception, the final fields, and the final array. The independent
[scenarios](scenarios.json) include exhaustion, an empty array, a null fault,
a break that bypasses a possible fault, and a fault after the recorder's
writes. The controls require the same-index revisit and an observer that
receives the incremented index.

## Preparation and checks

Apktool 2.11.1's embedded Smali assembler generated the DEX at API 24 through
a local Java adapter accepting a Smali directory, output file and API. The
prepared DEX has SHA-256
`13b1515ec38c13e2bd098d0ff8bb2b8fcda1626ea9976c757f8d0b1eb414e315`.

```sh
fixture=quality/fixtures/loops/counted-continue-update
work=.cache/fixtures/counted-continue-update
mkdir -p "$work"
java -cp "$APKTOOL_JAR" "$SMALI_ADAPTER" "$fixture" "$work/classes.dex" 24
cp "$work/classes.dex" "$fixture/classes.dex"
```

The bounded probe decodes the prepared DEX and replays the selected methods
and the original helper. It compares that result with the authored Java and
the exact Diana class publication, each compiled and observed separately.
DecBench also compares loop graphs for `advanceThenStop` and `holdOnZero`.
The `observeAfterAdvance` gate exercises update timing and exception behavior.

```sh
python3 -B quality/decbench run --binary /path/to/diana \
  --suite dex-structure-loop-for --suite dex-counted-continue-update
```

Frozen Diana 0.4.23 passes the twelve behavior cases and emits `while` for
both `stopAt` and `advanceThenStop`, which fail their structural checks. The
repaired build passes all 35 checks across the two suites. It recovers both
counted loops as `for` and retains both controls as `while`. [Semantic review](review.md)
