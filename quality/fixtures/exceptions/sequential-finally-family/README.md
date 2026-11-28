# Sequential finally with forwarded exception entries

The original member is
`Lfixture/FinallyFamily;->run(Lfixture/FinallyFamily;I)V`. The retained fixture
locator is `e0:l0:c0:m1`. The Java model and Smali are independently authored.
The native declarations in the DEX stand for the recording methods in
[FinallyFamily.java](FinallyFamily.java). The Smali is the authoritative input.

Three ordered stage calls at code units 1, 5 and 9 each have a distinct
catch-all. Their protected ranges are [1,4), [5,8) and [9,12) and their
entries are at 16, 19 and 22. Each entry moves its captured exception
from a temporary register into v4. The entries converge on the cleanup at
24, then rethrow v4 at 27. The equivalent cleanup at 12 handles normal
completion. Both calls receive v5 and v6, the original receiver and token.

Each [scenario](scenarios.json) uses a new receiver, token 42 and unique
exception objects. A fault at stage 1, 2 or 3 prevents any later stage.
The cleanup runs once on the original receiver. It rethrows the exact
stage fault after cleanup succeeds. A cleanup fault replaces the pending
fault. Each scenario specifies the complete ordered trace, including the
modeled call arguments and throwable identity.

The DEX was assembled with Apktool 2.11.1, OpenJDK 21.0.12.1 and API 24.
The source-mode assembler calls `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo` with output under
`.cache/fixtures/sequential-finally-family/`. The stored input is
[classes.dex](classes.dex) and the authored input is
[FinallyFamily.smali](FinallyFamily.smali).

Diana 0.4.3 omits cleanup on normal completion and on stage 3 failure.
Failures in stages 1 and 2 complete normally after cleanup. DecBench
reproduces five mismatches across six scenarios. The original DEX witness
and the independent Java control both agree with all six expectations.
The cleanup-family candidate recovers one enclosing `finally` and passes
all six scenarios.

```sh
diana get-method classes.dex 'Lfixture/FinallyFamily;->run(Lfixture/FinallyFamily;I)V' --format json
python3 quality/decbench run --suite dex-sequential-finally-family --binary /path/to/diana
```
