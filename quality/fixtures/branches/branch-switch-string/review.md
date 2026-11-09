# Fixture: branch-switch-string

String switches with grouped results and field effects.
`BranchSwitchString.java` is the authored reference; the committed DEX was prepared
with `javac --release 8 -g` and `d8 --min-api 24` through `quality/build.sh`.
No third-party program or data is used.

## Current verdict — acyclic case-entry review

Java and final describe/dispatch were reread. The original first integer switches
now have supplied case bodies rather than multi-entry wrappers. Hash selection
still precedes exactly the relevant equals call. A failed equality in a nonadjacent
case goes to the one default label; it does not evaluate another case's equality.
The last case can actually fall through to that default. A successful equality
stores its original discriminator and breaks to the unchanged second switch.
Null, collision/default behavior and add-input versus mul-literal identity remain.
The mul literal still resolves after hashCode and before its selected equality.

These gotos after conditional Ifs are required false-path continuations, unlike
the dead gotos after unconditional breaks removed earlier. No external string
protocol was inferred and no source target/copy/effect was duplicated. The fresh
[case-entry challenge](../case-entry-joins/review.md) adds three predecessors,
conditional shared work and actual copy/cycle guards. This is bounded LLM path
reading, not executed behavioral PASS; no device run or Java recompilation ran.

## Prior verdict — completed-tail review

Java and final describe/dispatch were reread. The first integer switches now omit
planned gotos that followed their already-emitted case breaks. Original hashCode,
equals, default/collision routes, discriminator stores and second switches are
unchanged. Null still fails on the initial call; add still writes the input
reference and mul the existing literal. No call, field effect or copy moved, and
no external string protocol was inferred by the engine. The explicit graphs and
available original members remain.

This is bounded direct LLM reading, not executed behavioral PASS. The fresh
[completed-tail challenge](../abrupt-continuation-tails/review.md) includes a
concrete two-stage discriminator, a later throwing read/cleanup and opaque
completion guards. Complete upstream P2/P3/P4/P5/P6 products agree. The original
source continuations remain inspectable; only their unreachable second display
actions disappear. No device run or Java recompilation ran.

## Prior verdict — round 2

The LLM review compared source with current ordinary Engine text for
`Lfixture/BranchSwitchString;`, using candidate source based on `6d663d9`:

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/branches/branch-switch-string/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

### Semantics

These are direct reading judgments, not automated behavioral execution results.

| Method | LLM judgment |
|---|---|
| `describe(Ljava/lang/String;)Ljava/lang/String;` | Equivalent: a selects discriminator 0 and returns first; b/c select 1/2 and share grouped; other strings select default and return other. No field is written. |
| `dispatch(Ljava/lang/String;)I` | Equivalent: add writes the input object to last and returns 1; sub returns 2 without a write; mul writes the literal mul and returns 3; default returns 0 without a write. |

Both first-stage hash switches still check equality before selecting a string
arm, so a matching hash alone is insufficient. A null input fails on the initial
`hashCode()` call before any result or field write, matching the source switch.
The add and mul assignments retain their different reference sources: the input
object is not replaced by an equal literal in the add case.

### Readability

The second switch in `describe` now keeps its grouped result as adjacent case 1/2
labels followed by one return, rather than dispatching into a graph. The original
hash/equality stage remains explicit: its shared default joins are not linear
case fallthrough chains.

## Ranked fixes

Improve the hash/equality protocol's readability while retaining collision checks,
null behavior, field reference identity and original member facts.

## Rounds

- R2: The grouped result switch is structured; direct LLM reading still finds
  agreement with source. The hash/equality protocol remains explicit.
- R1, 2026-11-08: Two-stage dispatch required following
  hash constants, equality calls, labels and a multi-entry graph.
