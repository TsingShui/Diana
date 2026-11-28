# Shared cleanup with staggered state joins

`Lfixture/ForwardedCleanupStaged;->run(Lfixture/ForwardedCleanupStaged;Lfixture/ForwardedCleanupStaged;Lfixture/ForwardedCleanupStaged;I)V`
has three independent catch-all entries. The left capture moves the
Throwable through a separate bridge. The right capture joins after that
move. Both enter the tag block on normal edges and pass their original
receiver, integer, and wide date to the cleanup. A third capture sets
the shared cleanup's arguments and enters it directly. This independently
authored fixture extends the two-entry case in [forwarded-cleanup](../forwarded-cleanup).

The DEX table maps protected ranges `[7,10)`, `[13,16)`, and `[19,22)`
to the direct, left, and right captures at code-unit offsets `47`, `23`,
and `33`. The bridge starts at `43`, the shared tag block at `44`, and
the shared cleanup at `58`. The fourth entry, `[58,62) → 62`, covers
both the cleanup call and the rethrow. The native hooks have Java models
that record argument values, call order and exception identity.

The Diana 0.4.2 baseline supplies zero for the integer and date on
both forwarded entries. That breaks four of the eight modeled scenarios.
The direct entry retains its scalar values in the baseline.

The input was assembled into `.cache/fixtures/forwarded-cleanup-staged/classes.dex`
using `SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`
from [Apktool 2.11.1](https://github.com/iBotPeaches/Apktool/releases/tag/v2.11.1)
with OpenJDK 21.0.12.1. The input source is in [source](source).

[Expected scenarios](scenarios.json) and [semantic review](review.md)

```sh
python3 quality/decbench prepare
python3 quality/decbench run --suite dex-forwarded-cleanup-staged --binary /path/to/diana
```
