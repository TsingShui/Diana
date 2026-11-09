# Shared typed catches inside a multi-exit loop

## Independent expectations

Authored Java/Smali; no third-party input. Discovery fixture:
`../typed-catch-shared-cleanup/`. The challenged guarantee is one body for adjacent
original catch clauses sharing a target, with original priority and enclosing
cleanup retained. Added interactions are repeated execution in a loop, a value
Phi crossing cleanup, continue/break/return after cleanup, and cleanup failure.
The Java implicit constructor is not part of the Smali subjects.

Original owner: `Lfixture/LoopCatchFinally;`. Subjects are `scan(II)I` and
`priority(I)I`. All three helpers have concrete bodies in the DEX:

- `mark(I)V` stores `trace * 31 + event` using 32-bit integer arithmetic.
- `read(II)I` marks 10+row, then throws NFE/NPE/ISE for modes 1/2/3,
  AssertionError for modes 4/6, or returns row+1.
- `cleanup(II)V` marks 30+row, then throws IAE for modes 5/6, otherwise returns.

Each scenario resets trace to zero. Constructed exceptions have message `row`
(read) or `close` (cleanup). No allocation/linkage failure, concurrent writer or
asynchronous exception is assumed. The expected event sequence below is derived
from these bodies and the Java/Smali transfers before candidate inspection.
The trace field supplements the ordered sequence; its value alone is not an
order proof.

| Original subject | Arguments | Expected exit | Ordered mark events |
|---|---|---|---|
| `scan(II)I` | (3,0) | return 4; row 1 continues after cleanup | 10,30,11,31,12,32 |
| `scan(II)I` | (3,1) | return 200; break after second recovered value | 10,20,30,11,20,31 |
| `scan(II)I` | (3,2) | return 200, same shared handler | 10,20,30,11,20,31 |
| `scan(II)I` | (3,3) | return -2 after cleanup | 10,21,30 |
| `scan(II)I` | (3,4) | throw original AssertionError("row") | 10,30 |
| `scan(II)I` | (3,5) | throw IAE("close"), no catch/retry of cleanup | 10,30 |
| `scan(II)I` | (3,6) | throw IAE("close"), replacing AssertionError | 10,30 |
| `scan(II)I` | (0,6) | return 0, no acquisition of a loop iteration | none |
| `priority(I)I` | 0 | return 1 | 10 |
| `priority(I)I` | 1 | return 100 | 10 |
| `priority(I)I` | 2 or 3 | return -2 | 10 |
| `priority(I)I` | 4 or 6 | throw AssertionError("row") | 10 |

`priority` is an order guard, not another positive recovery witness. Its Smali
retains NFE -> first, RuntimeException -> second, NPE -> first. The nonadjacent
shared target cannot be grouped ahead of RuntimeException: doing so would change
mode 2 from -2 to 100. Java expresses the same behavior without an unreachable
NPE catch. This is the pair's explicit notation difference.

## Preparation and comparison

Assemble once and use that input on the pre-fix engine at `ebbc003` and candidate.

```sh
mkdir -p .cache/fixtures/typed-catch-loop-exits
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/typed-catch-loop-exits/classes.dex \
  quality/fixtures/exceptions/typed-catch-loop-exits
cp .cache/fixtures/typed-catch-loop-exits/classes.dex \
  quality/fixtures/exceptions/typed-catch-loop-exits/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/typed-catch-loop-exits/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. Before/after
inspection uses the same prepared DEX. The candidate recovers the compound catch
and enclosing cleanup inside scan; the baseline uses explicit protection graphs.
Nonadjacent aliases in priority remain explicit in both revisions. The outer
loop is still an explicit graph, a separately recorded discovery in `review.md`.
`crates/diana-decomp/tests/catch_groups.rs` checks ownership, ordered groups,
protection and original source anchors. No behavioral execution result is claimed.
