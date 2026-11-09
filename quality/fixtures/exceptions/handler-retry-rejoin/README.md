# Retry after an ignored fast-path exception

Independently authored from the general R1 structure in
[the APK review](../../../reports/antennapod-3.12.3.md), without copied code.
The repository license applies. Java explains the assembly input Case.smali.
Original subject: `Lfixture/RetryRejoin;->run(III)I`.

## Independent obligations

Recorded before first inspection. The trigger is a fast call returning before
fallback, an ignored fast-path catch, then a retry loop whose captureless inner
handler is the same increment instruction reached by normal failed attempts.
Neither ignored handler has a fabricated move-exception. Original APK P1 shows
the corresponding ignored entries begin at an invoke and an add-int/lit8,
respectively. Java does not use either caught throwable, so omitting their
captures preserves the explanation's behavior.

A preliminary reduction with unused captures recovered a while on the baseline;
it did not reproduce R1. The captureless/shared-entry revision was recorded
before its first inspection and reproduces the explicit-loop refusal. The P6
recovery below preserves these source expectations rather than changing them.

Native helper model: fast records `(fast,mode)` and returns70 for mode0, otherwise
throws IllegalArgumentException F. reset records `(reset,row)` and returns.
attempt records `(attempt,row,mode)`, throws IllegalStateException A when
row=mode-1, otherwise returns10+row. Selected faults have their own identities;
other library/runtime behavior is unmodeled. The native implementations have not
been executed.

| Input/fault | Independent result and order |
|---|---|
| `run(0,12,3)` | `70`, only fast(0); reset/attempt/fallback are skipped. |
| `run(2,12,3)` | `12`; fast(2), reset0, attempt(0,2), reset1, attempt(1,2) throws, reset2, attempt(2,2) returns12. Both failed routes increment exactly once. |
| `run(2,99,3)` | `-1`, same three reset/attempt pairs; no invented result from the caught attempt. |
| `run(2,12,0)` | `-1` after fast(2) throws; no reset or attempt. |
| fast throws Error E instead of F | E propagates before fallback; only the selected IllegalArgumentException catches. |
| reset1 throws A | A propagates outside the attempt interval; there is no later attempt/reset. |
| attempt0 throws Error E | E propagates, no increment/retry. |

Preserve fast success, exception priority, the reset/attempt boundary, original
shared update, no false result commit and full method availability. Explicit
transfers are permitted; no source-level for is an acceptance requirement.

## Preparation

```sh
mkdir -p .cache/fixtures/handler-retry-rejoin
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/exceptions/handler-retry-rejoin/Case.smali \
  -o .cache/fixtures/handler-retry-rejoin/classes.dex
cp .cache/fixtures/handler-retry-rejoin/classes.dex \
  quality/fixtures/exceptions/handler-retry-rejoin/classes.dex
```

Baseline df10b744985e3407f902442b8252fab100700aec. Observed assembly used
Smali2.5.2, OpenJDK21.0.12.1 and API24, not an exact-version admission gate.
The original subject is class0/method3. Baseline and scalar-rendering candidate
both print a graph<Irreducible> retry with the same two captureless entries and
shared update. Every P2–P8 product and this method's text are unchanged.

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/handler-retry-rejoin/classes.dex \
  --exec '{"cmd":"p1","class":0,"method":3,"details":true}' \
  --exec '{"cmd":"p6","class":0,"method":3,"details":true}' \
  --exec '{"cmd":"p9","class":0,"method":3}'
```

The captureless/shared-normal-entry preservation check in
`crates/diana-decomp/tests/scalar_use_views.rs` passes on both revisions; it does
not require the graph shape forever. [review.md](review.md) records source/model
reasoning. Those scalar-repair observations did not claim natural recovery.

## P6 recovery

Against73fc8793a91d766d021d117aa83ddfd644f7ac82 on the same prepared bytes,
P6 now accepts the original captureless shared update inside one while. Fast
success still returns before fallback; reset remains outside the attempt chain.
No throwable capture, result or second increment is invented. Complete P2–P5
and original-coordinate P6 facts are unchanged. The new ordinary-Engine recovery
check and existing preservation check pass. Fresh
[nested/latch/foreign cases](../captureless-retry-nesting/README.md) challenge
the admission. Both full isolated configurations pass 155 checks, without
executing fixture/native behavior.
