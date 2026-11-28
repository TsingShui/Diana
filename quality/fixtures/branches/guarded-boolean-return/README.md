# Guarded boolean return

This independently authored fixture uses the Diana repository license. It
isolates a branch structure observed in the public AntennaPod 3.12.3 corpus
in `NetworkUtils.isEpisodeDownloadAllowed()Z`. The fixture supplies its own
predicates and effects. [Public corpus review](../../../reports/antennapod-3.12.3.md)

The selected method is `Lfixture/GuardedBoolean;->allowed()Z` at
`e0:l0:c1:m6`. In the DEX, a true result from `first()` jumps directly to
the true return. A false result reaches `second()`, whose true lane
returns false and whose false lane joins the first true return. The Java
explanation expresses those edges as `first() || !second()`.

The authored helper preserves the original effect order. Every predicate first
appends its event digit to the trace, then throws the configured exception
object when its digit equals the fault selector. Otherwise the helper returns
the bit indexed by its digit minus one. The observer initializes the trace to
zero, supplies the flags and fault selector, and checks the result and the
exact thrown object. Event digits 1 and 2 identify `first()` and `second()`.
The five [scenarios](scenarios.json) cover both second-call values, the
short-circuit path that skips a configured second-call failure, and both
reachable failures.

Diana 0.4.30 publishes two separate guarded constant returns and a final
true return. Bounded replay of the original DEX and execution of the authored
Java and exact frozen Diana publication each match all five scenarios.
The DecBench manifest checks the short-circuit return expression separately
from those behavior checks. The [nested challenge](../nested-guarded-boolean-return/)
adds mixed predicates and an independently observable branch effect.

Diana 0.4.31 emits one short-circuit return and passes the DecBench
publication check that fails on 0.4.30. All five executed scenarios pass on
both releases. The behavioral probe rejects a mutation that reverses the
second predicate. See the [semantic review](review.md).

The DEX was assembled with Apktool 2.11.1's embedded Smali, API 24, on
OpenJDK 21.0.12.1. The launcher used `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`. An equivalent local Smali recipe and
the focused checks can run from the repository root.

```sh
fixture=quality/fixtures/branches/guarded-boolean-return
work=.cache/fixtures/guarded-boolean-return
mkdir -p "$work/smali"
cp "$fixture/"*.smali "$work/smali/"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble --api 24 \
  -o "$work/classes.dex" "$work/smali"
PYTHONPATH=quality/decbench python3 -B "$fixture/verify_raw.py"
python3 -B quality/decbench run --binary /path/to/diana \
  --suite dex-guarded-boolean-return --output "$work/decbench.json"
```
