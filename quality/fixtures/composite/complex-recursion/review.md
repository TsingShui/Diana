# Fixture: complex-recursion

## Current verdict — terminal guard follow-up

Read parity/base accumulator and Fibonacci memo guards. Recursive arguments/call
order stay original; small Fibonacci returns before memo access, hit conversion
returns before recursive/cache work, and misses retain both recursive calls and
put order.

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

Read the changed iterative portion alongside unchanged recursive calls. Initial
values, recursive argument order, accumulator dependencies and exit tests
remain. For syntax schedules only the original pure tail, without claiming
progress or changing recursive semantics.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


mutual recursion, accumulator recursion and memoisation. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `isEven` | ✔ The inverted test still selects the original recursive branch. |
| `isOdd` | ✔ |
| `sumTo` | ✔ The entry delegates to the private overload. |
| `fibonacci` | ✔ Memo lookup precedes recursive computation and write-back. |
| `factorial` | ✔ Iterative multiplication agrees. |

### Findings

- Mutual and accumulator recursion agree; fibonacci retains memo read/write order.
- The memo field retains Map<Integer, Long>, but its initializer uses a raw HashMap expression. Recorded forms: `Map<Integer, Long> memo`, `this.memo = hashMap`.
- Intermediate locals and while-loop spelling account for remaining readability differences.

### Ranked fixes

Improve consistency between generic field declarations and initialization expressions; reduce redundant locals.

## Rounds

- R1 2026-11-08: Mutual and accumulator recursion agree; fibonacci retains memo read/write order.
