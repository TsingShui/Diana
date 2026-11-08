# Fixture: branch-switch-string

String switches with grouped results and field effects.
`BranchSwitchString.java` is the authored reference; the committed DEX was prepared
with `javac --release 8 -g` and `d8 --min-api 24` through `quality/build.sh`.
No third-party program or data is used.

## Current verdict — round 2

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

### Historical score

R1 only; not rerun for this candidate. Aggregate structure 18%, naming 71%.

| Method | Structure | Naming | First divergence |
|---|---|---|---|
| `dispatch` | 16% | 75% | `insert: ref=- ours=ID ID ; int ID` |
| `describe` | 21% | 67% | `insert: ref=- ours=int ID ; int ID` |

## Ranked fixes

Improve the hash/equality protocol's readability while retaining collision checks,
null behavior, field reference identity and original member facts.

## Rounds

- R2: The grouped result switch is structured; direct LLM reading still finds
  agreement with source. The hash/equality protocol remains explicit.
- R1, 2026-11-08: Structure 18%, naming 71%. Two-stage dispatch required following
  hash constants, equality calls, labels and a multi-entry graph.
