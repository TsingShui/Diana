# Sequential finally around a nested typed catch

The original member is
`Lfixture/FinallyTyped;->run(Lfixture/FinallyTyped;I)V`. The retained fixture
locator is `e0:l0:c0:m2`. The Java model and Smali are independently authored.
The native declarations in the DEX stand for the recording methods in
[FinallyTyped.java](FinallyTyped.java). The Smali is the authoritative input.

This extends the sequential cleanup case with a typed catch covering
stage 2 alone. The ordered handlers for [5,8) are
`IllegalArgumentException` at 9 and an outer catch-all at 24. The action of
the typed catch is at 10 and has its own catch-all entry at 30. Stage 1 and
stage 3 begin at 1 and 14 and have outer catch-all entries at 21 and 27.
Every outer entry forwards its captured exception into v4 and reaches
cleanup at 32 and rethrow at 35. The normal cleanup is at 17. Both receive
v7 and v8, the original receiver and token.

The [scenarios](scenarios.json) use a fresh receiver, token 42 and distinct
stage, handler and cleanup exception objects. A matching stage 2 failure
delivers the exact fault to the typed handler and, if the handler finishes,
resumes at stage 3. A handler failure escapes through the outer cleanup.
A nonmatching exception from stage 2 and a same-type exception from stage
1 both reach the outer cleanup directly. Every path runs cleanup once.
A cleanup fault takes precedence over the pending exception.

The DEX was assembled with Apktool 2.11.1, OpenJDK 21.0.12.1 and API 24.
The source-mode assembler calls `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo` with output under
`.cache/fixtures/sequential-finally-family-typed/`. The stored input is
[classes.dex](classes.dex) and the authored input is
[FinallyTyped.smali](FinallyTyped.smali).

Diana 0.4.3 retains the inner catch but skips outer cleanup when stage 3
returns or throws. Stage 1 and 2 exceptions and failures from the typed
handler can complete normally after cleanup. DecBench reproduces six
mismatches across seven scenarios. Both the original DEX witness and
the independent Java control agree with all seven expectations.
The cleanup-family candidate preserves the inner typed catch inside one
enclosing `finally` and passes all seven scenarios.

```sh
diana get-method classes.dex 'Lfixture/FinallyTyped;->run(Lfixture/FinallyTyped;I)V' --format json
python3 quality/decbench run --suite dex-sequential-finally-family-typed --binary /path/to/diana
```
