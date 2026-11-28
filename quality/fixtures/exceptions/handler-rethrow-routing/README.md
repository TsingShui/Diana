# Cleanup dispatch after a failure inside a catch

These methods model three separate exception scopes. A setup failure can
retry. Scheduling and waiting use a cleanup that cancels the outstanding
operation before passing failures to the session handler. The interrupted
wait has its own typed catch. A failure inside that catch still reaches
the same cleanup and can never reenter the setup retry.

`runWithSecondaryCatch` adds a locally handled exception in the
interruption handler. A further fault in the secondary handler runs the
same cleanup exactly once and reaches the session handler. The Smali
helpers are native declarations. The Java control records calls and
checks that the session handler receives the exception object actually
thrown, including a replacement exception from cancellation.

The previous candidate runs cancellation twice when an interruption
handler throws, once in its own catch and once when its rethrow reaches
an enclosing copy of the same cleanup. Four of ten model scenarios expose
that extra effect. The candidate with the shared cleanup placed once
passes all ten scenarios.

The input was assembled into `.cache/fixtures/handler-rethrow-routing/full.dex`
with the Smali builder in [Apktool 2.11.1](https://github.com/iBotPeaches/Apktool/releases/tag/v2.11.1)
on OpenJDK 21.0.12.1, API 24. It used `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`.

[Scenarios](scenarios.json) · [Semantic review](review.md)

```sh
python3 quality/decbench prepare
python3 quality/decbench run --suite dex-handler-rethrow-routing --binary /path/to/diana
```
