# Protected branch joins

The authored Java and Smali model a diamond with two observable calls, a shared
continuation, an exception exit and two unprotected calls. Native methods
denote distinct observable operations. Each can return an arbitrary int or
throw. An invocation that throws does not assign its destination register.

In `route`, `before(input)` runs once. Zero selects `left` and any other key
selects `right`. The selected operation consumes the result of `before`.
Normal completion calls `join` with the arm result and `after` with the result
of `join`. An `IllegalStateException` from either protected call returns
`recover` of the original `before` result. A failure of `before`, `after` or
`recover` escapes.

The challenge `loopRoute` moves the diamond into a loop with a bypass edge.
Even positive counts select `left`, odd positive counts select `right`.
A zero arm result skips `join` and reaches the shared decrement. A nonzero
arm result goes to `join` before the decrement. The next iteration reads the
last successful result. Nonpositive counts call `after` immediately. If an
arm throws an `IllegalStateException`, recovery reads the value from before
the arm. If `join` throws that exception, recovery reads the arm result. A
different exception escapes at its call site. Normal completion calls
`after` once and returns its result.

The wide challenge `wideRoute` unrolls six successive stages. Each has two
separate DEX protected ranges with the same handler. A nonthrowing integer
addition connects them. The six handlers differ in the constant subtracted
from their argument. On success, `left` consumes the value carried into the
stage and `right` receives its result plus the stage number. Outside this
handler, `after` receives the result of `right` and supplies the next stage.
If `left` throws a matching exception, `recover` receives the incoming
value minus the stage number. If `right` throws it, the argument to
`recover` is the completed `left` result. The caller observes the sixth
`after` result, any nonmatching exception, and any failure of the initial
`before`, of `after` or of the handlers themselves.

The fixture was assembled with Smali 3.0.9 on OpenJDK 21.0.12.1 at API 24.

```sh
fixture=quality/fixtures/branches/protected-branch-joins
work=.cache/fixtures/protected-branch-joins
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble \
  --api 24 -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
diana get-class "$fixture/classes.dex" 'Lfixture/ProtectedJoins;'
cargo nextest run --locked -p diana-decomp --test protected_branch_joins_test
```
