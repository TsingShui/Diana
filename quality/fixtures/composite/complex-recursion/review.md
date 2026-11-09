# Fixture: complex-recursion

## Current verdict — whole-operation follow-up

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
