# Nested handler rejoins and a foreign-entry guard

## Input and independent expectations

Authored Java/Smali with no third-party input. Parent discovery:
`../typed-catch-loop-exits/`. The repaired guarantee is that an internally entered
catch corridor rejoining a normal cycle does not create a fictitious external
loop entrance. Added interactions are two nested loops, repeated monitor capture,
shared typed catches, nested finally cleanup and monitor release on abrupt exit.
The guard adds an actual pre-loop exceptional arrival to the shared rejoin.

The helper Java/Smali pair is copied from the parent and included in this DEX,
not supplied as external declarations. Its read/mark/cleanup bodies and modes
are unchanged. `LoopCatchFinally.trace` starts at zero. Standard exception
construction succeeds; no asynchronous exception, linkage failure, allocation
failure or concurrent trace writer is modeled. The Java implicit constructors
are not Smali subjects.

Original subjects: `Lfixture/NestedCatchLoops;->run(Ljava/lang/Object;II)I` and
`Lfixture/NestedCatchLoops;->foreign(II)I`. Java expresses the foreign guard with
two catches; Smali shares their actual handler/capture and normal join.

`run` computes code=row*3+column. Each iteration acquires the supplied non-null
lock, reads, closes while still held, adds the selected value, then releases.
A typed failure marks 20 and selects 100. Every exit through read or cleanup
failure releases before propagating. A null lock fails before read/cleanup.

| Subject | Arguments | Expected exit | Ordered mark events / monitor effects |
|---|---|---|---|
| `run` | (lock,2,0) | return 21 | For code 0..5: LOCK, mark(10+code), mark(30+code), UNLOCK |
| `run` | (lock,2,1) or (lock,2,2) | return 600 | For code 0..5: LOCK, mark(10+code), mark(20), mark(30+code), UNLOCK |
| `run` | (lock,2,3) | throw ISE("row") | LOCK, 10,30,UNLOCK |
| `run` | (lock,2,4) | throw AssertionError("row") | LOCK,10,30,UNLOCK |
| `run` | (lock,2,5) | throw IAE("close") | LOCK,10,30,UNLOCK; no retry |
| `run` | (lock,2,6) | throw IAE("close"), replacing Error | LOCK,10,30,UNLOCK; no suppression |
| `run` | (null,1,0) | throw NPE at acquisition | none |
| `run` | (null,0,6) | return 0 | none |
| `foreign` | (2,0) | return 3 | 10,10,11 |
| `foreign` | (2,1) or (2,2) | return 200 | 10,20,11,20 |
| `foreign` | (0,1) | return 100 | 10,20; recovery precedes the first loop test |
| `foreign` | (2,3) | throw ISE("row") | 10 |

The guard's shared handler can run before the loop condition, even for zero rows.
It therefore cannot become a private loop catch without preserving that entry.
A graph is an acceptable guard result; a fabricated while that skips the first
recovery is not. Expectations are recorded before candidate inspection.

## Preparation

```sh
mkdir -p .cache/fixtures/nested-catch-rejoins
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/nested-catch-rejoins/classes.dex \
  quality/fixtures/exceptions/nested-catch-rejoins
cp .cache/fixtures/nested-catch-rejoins/classes.dex \
  quality/fixtures/exceptions/nested-catch-rejoins/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/nested-catch-rejoins/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Assembled with Smali 2.5.2, API 24 and OpenJDK 21.0.12.1. The baseline is the
catch-group working tree based on `ebbc003`, before the private-handler loop-entry
repair. Both comparisons use the same prepared DEX. An initial maximal-SCC
candidate recovered only the outer loop; the completed candidate establishes
both headers without changing normal dominance. See review.md. No behavioral
execution result is claimed.
