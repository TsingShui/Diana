# Shared switch entries and fallthrough

These Java/Smali sources are independently authored for this repository.
Original owner: `Lfixture/SwitchFallthrough;`. Java explains behavior; Smali fixes
the distinct case-entry and transfer structure. All subjects are static. The
unused Java default constructor has no Smali counterpart.

## Independent scenarios

Each invocation starts with `state = 0`, `observed = 0.0f` and an empty call trace. `mark(event)`
records its argument and updates `state = state * 10 + event`, using ordinary
32-bit integer arithmetic. No helper throws in these scenarios. The expected
results and calls below are derived from the authored sources before inspecting
Diana output. Trace order, not just call count, is material.

| Method / arguments | Result | Ordered mark arguments | Final state |
|---|---|---|---|
| `shared(1)` | 11 | 1 | 1 |
| `shared(2)` | 11 | 1 | 1 |
| `shared(3)` | 33 | 3 | 3 |
| `shared(99)` | -1 | none | 0 |
| `chain(10)` | 3 | 1, 2 | 12 |
| `chain(11)` | 2 | 2 | 2 |
| `chain(12)` | 3 | 3 | 3 |
| `chain(99)` | -1 | none | 0 |
| `parameterSeed(10, 7)` | 10 | none | 0 |
| `parameterSeed(11, 7)` | 9 | none | 0 |
| `parameterSeed(99, 7)` | -1 | none | 0 |
| `emptyDefault(1)` | 3 | 1 | 1 |
| `emptyDefault(2)` | 5 | 2 | 2 |
| `emptyDefault(99)` | 0 | none | 0 |
| `conditional(1, 0)` | 1 | 1 | 1 |
| `conditional(1, 1)` | 12 | 1, 2 | 12 |
| `conditional(2, 0)` | 2 | 2 | 2 |
| `conditional(99, 0)` | 9 | 9 | 9 |
| `sharedTail(1)` | 13 | 1, 3 | 13 |
| `sharedTail(2)` | 23 | 2, 3 | 23 |
| `sharedTail(3)` | 3 | 3 | 3 |
| `sharedTail(99)` | 9 | 9 | 9 |
| `wide(10)` | 3L | none | 0 |
| `wide(11)` | 2L | none | 0 |
| `wide(99)` | -1L | none | 0 |
| `mixedBits(10)` | 1065353219 | none | 0 |
| `mixedBits(11)` | 1065353218 | none | 0 |
| `mixedBits(99)` | -1 | none | 0 |

Every `mixedBits` invocation also calls `observeFloat(1.0f)` before selection and
sets `observed = 1.0f`. Smali reuses the single bit pattern `0x3f800000` as a float
argument and an integer accumulator; Java spells the two meanings separately.

## Trigger and guards

- `shared(I)I`: keys 1 and 2 target the same original body, which must execute once.
- `chain(I)I`: key 10 falls through to key 11. The original zero definition is
  before selection. Direct key 11 must use zero, but falling through from key 10
  must not reset the accumulator. The mark calls distinguish the entry paths.
- `parameterSeed(II)I`: the incoming parameter remains pinned storage. A surviving
  direct-entry copy must not execute again on fallthrough; explicit layout is
  acceptable when that distinction cannot be represented naturally.
- `emptyDefault(I)I`: default targets the external follow directly. An empty
  default body must not accidentally enter a later case.
- `conditional(II)I`: an explicit break and normal case fallthrough share one
  source case, preserving both branch outcomes and their selected transfers.
- `sharedTail(I)I`: two distinct case bodies reach case 3. Smali keeps the common
  mark(3) invocation once; Java spells its effect separately in case 1 to explain
  the behavior. No recovery may duplicate the original effect to force a chain.
- `wide(I)J`: the accumulator is two words; its original constant and Phi copy
  must retain width, bits and execution position.

- `mixedBits(I)I`: one constant has both float and integral uses. It cannot be
  materialized into one concrete numeric interpretation merely to eliminate an
  entry copy; the float write must retain the value 1.0 rather than 1065353216.0.

## Preparation and inspection

```sh
mkdir -p .cache/fixtures/switch-fallthrough
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/switch-fallthrough/classes.dex \
  quality/fixtures/branches/switch-fallthrough
cp .cache/fixtures/switch-fallthrough/classes.dex \
  quality/fixtures/branches/switch-fallthrough/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/branches/switch-fallthrough/classes.dex \
  --exec '{"cmd":"p5-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. These record the
actual environment, not required tool versions. The same prepared input was
inspected with the engine at `6d663d9` and the candidate based on that revision.
The baseline's shared labels and accumulator fallthrough require explicit graph
routing; the candidate supplies grouped case bodies and preserves the guarded
parameter/mixed-view/shared-tail layouts.

`crates/diana-decomp/tests/switch_fallthrough.rs` runs nine structural/source-anchor
checks through the ordinary Engine entry. Use
`cargo nextest run --locked -p diana-decomp --test switch_fallthrough`.
All nine pass on the candidate; the complete current engine set has 19 passing
structural checks. The LLM semantic comparison is recorded in `review.md`.
These observations are not execution of the independent scenario table or
EVOLUTION behavioral regression acceptance.
