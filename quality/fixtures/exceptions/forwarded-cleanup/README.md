# Shared cleanup with two exception captures

`Lfixture/ForwardedCleanup;->run(Lfixture/ForwardedCleanup;Lfixture/ForwardedCleanup;Z)V`
selects one of two calls. Their distinct catch-all entries capture the
exception and supply a receiver, an integer, a wide value, and a tag to
one cleanup block over normal edges. A third handler covers the cleanup
call and the rethrow. This independently authored fixture checks that
changes to the shared cleanup do not change either ingress.

The original DEX table maps protected ranges `[4,7)` and `[10,13)` to
`0x0e` and `0x1a` in code units. They jump or fall through to the
cleanup at `0x25`. Its call and rethrow are covered by `[37,41)`, which
dispatches to `0x29`. The Java control implements the native hooks.
Normal calls return directly. An original or replacement exception is
reported by identity. A failure in the reporter itself escapes.

The input was assembled into `.cache/fixtures/forwarded-cleanup/classes.dex`
using `SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`
from [Apktool 2.11.1](https://github.com/iBotPeaches/Apktool/releases/tag/v2.11.1)
with OpenJDK 21.0.12.1. The input source is in [source](source).

[Expected scenarios](scenarios.json) and [semantic review](review.md)

```sh
python3 quality/decbench prepare
python3 quality/decbench run --suite dex-forwarded-cleanup --binary /path/to/diana
```
