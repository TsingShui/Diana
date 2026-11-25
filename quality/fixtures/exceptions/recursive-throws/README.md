# Checked exceptions in a recursive call graph

The native operation `zRead` can return or throw any `Throwable`. The DEX
declares this contract on the native operation only. The Java source explains
the caller contracts. `ReadFailure` is an input-defined subtype of `Throwable`.

| Entry | Effects and expected outcome |
| --- | --- |
| `c(false)` | Call `zRead()` once. A thrown value escapes unchanged. |
| `c(true)` | Call `b(false)`, which calls `c(false)`. A `ReadFailure` returns normally from the inner handler. Any other throwable escapes. |
| `a(flag)`, `b(flag)` | Call `b(flag)` or `c(flag)`, respectively. The reachable `b` handler accepts `ReadFailure`. Each caller can still propagate another throwable. |
| `d()` | Call `c(false)` twice if the first call returns or throws `ReadFailure`. Another throwable from the first call escapes immediately. The second handler accepts any throwable. |
| `e()` | Call `d()` once. Both normal completion and every throwable from that call lead to a void return. |

The complexity challenge couples a backward dependency (`a` calls `b`, which
calls `c`) with a cycle and two sites calling the same method under different
handlers. The caller exceptions must propagate to `a`, `b`, `c` and `d`.
The catch-all discharges the contract on `e`.

Smali 3.0.9 on OpenJDK 21.0.12.1 assembled these files at API 24.

```sh
fixture=quality/fixtures/exceptions/recursive-throws
work=.cache/fixtures/recursive-throws
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble \
  --api 24 -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
diana get-class "$fixture/classes.dex" 'Lfixture/RecursiveThrows;'
cargo nextest run --locked -p diana-decomp --test recursive_throws_test
```
