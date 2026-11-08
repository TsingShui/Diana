# Fixture: branch-switch-packed

Packed switch with grouped labels, fallthrough, negative keys and default.
`BranchSwitchPacked.java` is the authored reference; the committed DEX was
prepared with `javac --release 8 -g` and `d8 --min-api 24` through
`quality/build.sh`. No third-party program or data is used.

## Current verdict — round 2

The LLM review compared source, original case targets and current ordinary Engine
text for `Lfixture/BranchSwitchPacked;`. Candidate source is based on `6d663d9`.
This is direct semantic reading, not an executed behavioral check.

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/branches/branch-switch-packed/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

### Semantics

| Method | LLM judgment |
|---|---|
| `classify(I)I` | Equivalent: -1 returns 0 without writing state; 0/1 write 1 and return 10; 2 writes 2 and reaches the shared return of 32; 3 reads existing state and returns state+30 without resetting it; default writes 9 and returns -1. |
| `clustered(I)I` | Equivalent: key 10 returns 3, key 11 returns 2, key 12 returns 3, default returns -1. It does not write the static state. |

In `classify`, the current plan places the common return after the switch.
Cases 2 and 3 break to that original shared target; other cases return inside
the switch. This differs from source fallthrough spelling but preserves its
field write/read and return order. For example, after `classify(99)` sets state
to 9, `classify(3)` returns 39 and keeps state at 9.

In `clustered`, the actual original zero definition materializes before the
selector. Key 10 assigns 0+1 and then enters key 11's addition. Direct key 11
keeps the initialized zero. There is no entry reset on the fallthrough path.
The independent results above follow directly from the authored arithmetic.

### Readability

Both methods now use case bodies rather than `graph<MultipleEntry>` and dispatch
gotos. Keys 0/1 share a single body. The accumulator path from key 10 to key 11
is a natural fallthrough. Default order is not itself a correctness issue;
placement follows the delivered control relationships.

The nine switch structural checks include these two original methods and verify
that their code remains available without graph/goto dispatch. Their source
initializer and call-origin checks supplement this LLM judgment; they are not
independent execution of these methods.

## Ranked fixes

Improve local names and expression concision while preserving original member
identities, the shared return and selected-entry assignment behavior.

## Rounds

- R2: Grouped labels and accumulator fallthrough are structured; direct LLM path
  comparison finds agreement with source. Automated behavioral execution is unrun.
- R1, 2026-11-08: Behavior was judged equivalent, but every case
  dispatched into a multi-entry graph and required following labels.
