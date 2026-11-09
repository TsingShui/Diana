# Fixture: obf-dp-r8

## Current verdict — while/do-while architecture correction

Read every changed loop's entry test, ordinary body and original backedge. The
same body work precedes the now-visible update; an exiting test or abrupt effect
skips exactly the original remaining work. Initial values, loop nesting,
call/access/argument order and selected results remain. Some body-only
temporaries can now be declared at their existing assignments because no header
reads them. This changes no original value or effect.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Read the changed obfuscated scalar loops using original member identities and
retained source mapping. Recurrence/array dependency order and exit values
remain; supplied counters stay at their original body tails. No original name or
external declaration is substituted into the input facts.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


R8-obfuscated edit distance, LIS and knapsack. Built from the same Java reference as its source fixture with an
obfuscator, so the decompiled bodies can be compared against a known-good
reference.

- Provenance: authored for this repository. No third-party code or data; the
  obfuscated DEX is generated from our own source.
- Toolchain: R8, or ProGuard when the rules file starts with `# obfuscator:
  proguard`; `-dontshrink -dontoptimize` unless the rules say otherwise.
  `quality/build.sh` runs whichever tool the fixture's own rules file names.
- `mapping.txt` records original and obfuscated identities. Pair methods by
  original owner, name and full prototype; reused short names alone are ambiguous.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — iteration-tail review

Mapping and complete final class were reread; the Java pair is identical to
algo-dp. Subjects are La;->a(Ljava/lang/String;Ljava/lang/String;)I,
La;->b([I[II)I and La;->c([I)I, not an ambiguous short-name match. R8's original
cached loads/indices remain intact. Recurrence stores, descending room work and
LIS conditional/max writes precede the same extracted counters. Failure skips
the pending update; empty LIS returns zero. No original short identity changed.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `editDistance` | ✔ The body matches algo-dp.editDistance; two-dimensional creation still uses Array.newInstance. |
| `longestIncreasing` | ✔ The body matches its source fixture. |
| `knapsack` | ✔ The body matches its source fixture. |

### Findings

- The recorded body comparison found no additional structure loss from renaming.
- Methods become a/b/c; Array.newInstance and array reads lacking casts remain visible.
- Use mapping.txt and original parameter types to pair obfuscated members with their source identities.

### Ranked fixes

As for algo-dp, improve two-dimensional declarations/reads and loop presentation.

## Rounds

- R1 2026-11-08: The recorded body comparison found no additional structure loss from renaming.
