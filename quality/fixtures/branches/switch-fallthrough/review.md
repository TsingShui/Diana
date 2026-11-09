# Fixture: switch-fallthrough

## Current verdict — acyclic case-entry review

Java, Smali, helpers and final sharedTail(I)I were reread. Case1 marks1 then keeps
its original goto to L6, skipping case2; case2 marks2 then falls into the one case3
body; direct case3 skips both prefixes. Each path still marks3 once, returning
state13,23,3 respectively; default marks9 and returns9. The shared body is now a
natural case entry, not duplicated work. Other method text is unchanged, including
the nonidentity parameter and mixed-view copy guards.

The old requirement for a graph with two predecessors was a source-layout
qualification, not a semantic obligation. Its package check now asserts one shared
body and the exact retained nonadjacent target. The new
[case-entry challenge](../case-entry-joins/review.md) adds three predecessors,
conditional work, executable selector copies and a copy-free case cycle.
This is bounded direct LLM reading, not executed behavioral PASS. No device run
or Java recompilation ran.

## Prior LLM semantic review

The Java explanation, actual Smali transfers and current ordinary Engine output
were read together, including `mark` and `observeFloat`. The candidate is based
on `6d663d9`; this is a manual semantic-equivalence judgment, not execution of
the scenario table or EVOLUTION behavioral acceptance. The README owns the
independently authored expected results and ordered effects.

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/branches/switch-fallthrough/classes.dex \
  --exec '{"cmd":"p5-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

### Semantics

Original owner: `Lfixture/SwitchFallthrough;`.

| Method | LLM judgment and reasoning |
|---|---|
| `shared(I)I` | Equivalent: keys 1/2 share one mark(1) and return 11; key 3 marks 3 and returns 33; default returns -1 without a mark. |
| `chain(I)I` | Equivalent: the original zero is stored before selection. Key 10 marks 1, assigns 1, then falls into key 11's mark(2) and addition. Direct key 11 starts at zero and skips mark(1). Results are 3 and 2 respectively, with ordered traces [1,2] and [2]. |
| `parameterSeed(II)I` | Equivalent but explicit: direct key 11 copies the original seed into the shared accumulator before its goto; key 10 first computes seed+1 and copies that value. The shared addition therefore gives seed+3 or seed+2, never a reset on fallthrough. |
| `emptyDefault(I)I` | Equivalent: default is an empty final group followed by break, returning the initialized zero without a mark. Keys 1/2 assign 3/5, mark 1/2 and break. |
| `conditional(II)I` | Equivalent: key 1 marks 1, then flag==0 breaks the switch; other flags enter key 2 and mark 2. Direct key 2 skips mark(1). The conditional break is still present. |
| `sharedTail(I)I` | Equivalent but explicit: keys 1/2 mark their own digit and jump/fall into the single mark(3) body. Key 3 enters it directly. Default marks 9. No shared call is duplicated. |
| `wide(I)J` | Equivalent: the two-word zero definition remains before selection; key 10 adds 1L then 2L, direct key 11 adds only 2L, and default returns -1L. |
| `mixedBits(I)I` | Equivalent but explicit: the float use remains observeFloat(1.0f), while the shared integer addition starts from 1065353216 or 1065353217. The same bits are not turned into an integer-to-float numeric conversion. |

`mark(I)V` still multiplies the old state by ten and adds its argument before
writing the field. `observeFloat(F)V` still writes its argument to `observed`.
All reviewed normal paths therefore preserve the README's call order, field
writes and results. The helpers contain no deliberate throwing path; unrelated
VM failures and concurrent callers are outside the scenarios reviewed here.

### Producer and layout findings

- P5 can materialize an existing numeric constant at its original definition
  when one interference-checked storage group can also carry its normal Phi
  input. Ordinary constant reads stay folded. In `chain`, this removes the
  selected-only reset at key 11 without executing it on key 10's fallthrough.
- P6 groups original links sharing an entry and orders private bodies into
  linear fallthrough chains. P7/P9 consume the groups; they do not rediscover
  membership or duplicate bodies.
- Empty If arms retain their actual original boundary target. Treating both
  arms as the containing If's common follow would wrongly omit the flag test
  in `conditional`; its delivered break now prevents that omission.
- Pinned parameter entry copies, mixed literal interpretations and a tail with
  two fallthrough predecessors retain explicit layout. These refusals preserve
  readable effects and original transfers rather than manufacture a dispatcher.

### Supporting checks

`crates/diana-decomp/tests/switch_fallthrough.rs` supplies nine structural checks
through the ordinary Engine entry, including the original packed-switch examples.
They check grouping, original case coverage, source/PC anchors, initializer
placement, conditional-break retention, width and explicit-layout guards.
Run with `cargo nextest run --locked -p diana-decomp --test switch_fallthrough`.
They do not independently execute the scenario table. The semantic verdict comes from direct path comparison.

## Ranked fixes

Future improvements can address surviving selected-only entry copies and
nonlinear shared tails, while retaining each original effect once. No semantic
mismatch was found in the paths reviewed here.
