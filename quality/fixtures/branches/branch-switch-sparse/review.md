# Fixture: branch-switch-sparse

Noncontiguous and large switch keys. `BranchSwitchSparse.java` is the authored
reference; the committed DEX was prepared with `javac --release 8 -g` and
`d8 --min-api 24` through `quality/build.sh`. No third-party program or data is used.

## Current verdict — round 2

The LLM review compared source, original case keys/targets and current ordinary
Engine output for `Lfixture/BranchSwitchSparse;`, using candidate source based on
`6d663d9`. This is semantic reading, not an executed behavioral check.

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/branches/branch-switch-sparse/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

### Semantics

| Method | LLM judgment |
|---|---|
| `map(I)I` | Equivalent: -1000 returns 1; 7 returns 2; 100 writes state=1 then returns 3; 65535 writes state=2 then returns 4; all other keys return 0 without a write. |
| `table(I)I` | Equivalent: odd keys 1 through 13 return 11,33,55,77,99,111,133 respectively; every other key returns -1. No field is changed. |

The DEX represents `table` with a packed payload. Its even holes 2 through 12
share the original default target. The current output groups these original
labels with default and emits their return body once. Out-of-range keys also
select that body. Their extra labels are original DEX facts, not added behavior.
The normal bodies contain no discarded call, write or exception handling.

### Readability

Both methods now have directly readable case bodies, without graph/goto dispatch.
The even-hole labels remain visible but no longer repeat separate gotos. Key
identities and the two state writes remain explicit.

## Ranked fixes

Keep the payload's original key/default facts available while improving their
presentation. Source-style brevity does not justify deleting original keys.

## Rounds

- R2: Shared targets are grouped, with equivalent results and field effects by
  direct LLM reading. Automated behavioral execution is unrun.
- R1, 2026-11-08: Payload holes and explicit dispatch obscured the
  otherwise understood key mapping.
