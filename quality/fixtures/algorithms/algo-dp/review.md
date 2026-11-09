# Fixture: algo-dp

## Current verdict — terminal guard follow-up

Read the zero-length LIS guard and remaining recurrence. Empty input returns0
before allocation; nonempty input retains table initialization, nested
dependencies, accesses and both body updates. Allocation/access failure still
precedes the same later effects.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — while/do-while architecture correction

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

Read the changed dynamic-programming loops: initial state/table work remains
before the first test; cell dependencies and array accesses retain their order.
Supplied scalar counter tails execute after the same completed body, not after
an exiting test or fault. No arithmetic, recurrence or result is invented by the
new for headers.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


edit distance, longest increasing subsequence and knapsack. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — iteration-tail review

Java and final text were reread for editDistance(String,String), knapsack(int[],int[],int)
and longestIncreasing(int[]). The recurrence's outer/inner counters, outer item
counter and LIS counters now use delivered for updates. Initialization loops and
descending room iteration keep their previous layout. Length/charAt/array reads,
Math.min ordering and conditional stores still precede the updates; a failure
skips that update. LIS's final longest selection stays before its outer update,
and empty input still returns zero. No result or original declaration changed.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 2, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `editDistance` | ✔ Boundary initialization, nested Math.min and diagonal recurrence agree; int[][] declarations are restored; 64 executed comparisons agreed. |
| `longestIncreasing` | ✔ The nested loops and best[i] = best[j] + 1 agree. |
| `knapsack` | ✔ Reverse capacity traversal and candidate comparison agree. |

### Findings

- `new int[m][n]` becomes `Array.newInstance(Integer.TYPE, new int[]{m,n})` plus an Object local; most reads have `/* Object load */` and lack casts.
- Intermediate locals and loop spelling account for the remaining readability differences.
- The nesting order of `Math.min(v32, v35)` and `Math.min(v29, v37)` agrees.

### Ranked fixes

Recover array construction and loop presentation; reduce intermediate array-read locals. Recorded forms: `new int[m][n]`.

## Rounds

- R2 2026-11-08: Two-dimensional array declarations were restored to `int[][]`; the three methods agreed in 1646 executed comparisons.
- R1 2026-11-08: `new int[m][n]` becomes `Array.newInstance(Integer.TYPE, new int[]{m,n})` plus an Object local; most reads have `/* Object load */` and lack casts.
