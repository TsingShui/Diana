# Sequential finally with outer catch state

The original member is
`Lfixture/FinallyOutwardState;->run(Lfixture/FinallyOutwardState;II)V` at
`e0:l0:c0:m2`. The fixture sources are independently authored. The
[Smali](FinallyOutwardState.smali) defines the retained DEX and the
[Java control](FinallyOutwardState.java) defines the native hook stubs.

This extends the sequential finally family with one outer catcher that
observes the live scalar state and original throwable identity. State is
3 before stage 1. Once stage 1 succeeds, the route argument determines
whether the joined state is 1 or 2. The protected ranges [2,5), [11,14)
and [15,18) lead to distinct catches at 22, 25 and 28. Each capture
forwards into v4 and enters the shared cleanup at 30. The normal cleanup
is at 18. Both invoke sites pass the original receiver and token in
v7 and v8. The original rethrow is at 33. Both cleanup copies and the
rethrow are protected by ordered DEX entries [18,21) and [30,34) that
target the same outer catch at 34. The outer catch calls `outward(v5, v6)`
at 35.

The [ten scenarios](scenarios.json) use an original receiver, token
42, a route, and modeled stage, cleanup and reporter faults. Both
nonthrowing routes run cleanup once. A cleanup failure on normal
completion reaches the outer handler with timestamp 1 or 2. A failure
in stage 1 reaches it with timestamp 3. Later stage failures reach it
with their branch timestamp. A cleanup failure replaces the pending
fault. The catcher observes both the timestamp and exact exception
object before a modeled reporter failure can escape.

The DEX was assembled with Apktool 2.11.1, OpenJDK 21.0.12.1 and API 24.
The source-mode assembler calls `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`. The retained DEX has SHA-256
`07044810dc77f35fb5c99328be619471611d2479ad8730230db84c089ea66bd8`.

The original DEX path witness and Java control agree on all ten
scenarios. The 0.4.2 release fails eight scenarios. In particular,
both early failure traces incorrectly report timestamp 0. Recovery at
`c59c4879` passes all ten in DecBench. The earlier 0.4.3 executable
has no verified result for this input.

```sh
diana get-method classes.dex 'Lfixture/FinallyOutwardState;->run(Lfixture/FinallyOutwardState;II)V' --format json
python3 quality/decbench run --suite dex-sequential-finally-outward-state --binary /path/to/diana
```
