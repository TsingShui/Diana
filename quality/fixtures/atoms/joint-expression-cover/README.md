# Joint expression ownership

Independently authored Java and Smali, under the repository's Apache-2.0 license.
Smali determines the actual input. Original owner: `Lfixture/JointCover;`.
No external declarations are supplied to the engine.

## Independent expectations

The subjects `ordered(I)I` and `blocked(I)I` share a successful result arm between
an initial flag test and a later test with two effectful argument producers.
`blocked` adds a void effect between the numeric call and its branch. That effect
prevents a compact short-circuit spelling; admission of an earlier argument
forest MUST NOT hide any operations when the whole condition is refused.
`ordinary(I)I` uses the same producer forest without a branch. `reversed(I)I`
reverses the pair's operand order while preserving original call order.

The native helper model is explicit: `take(x)` records T(x), returns x + 10;
`pair(x,y)` records P(x,y), returns 100*x+y; `mark(x)` records M(x), returns void.
Any selected helper can instead throw a distinct E. A fault stops the current
subject immediately; there is no catch, retry or later helper evaluation.

| Subject/scenario | Expected result | Ordered effects |
|---|---|---|
| ordered(flag != 0) | 7 | empty |
| ordered(0) | 7 | T(1), T(2), P(11,12) |
| blocked(flag != 0) | 7 | empty |
| blocked(0) | 7 | T(1), T(2), P(11,12), M(1112) |
| ordinary(seed) | 101*seed+1011 | T(seed), T(seed+1), P(seed+10,seed+11) |
| reversed(seed) | 101*seed+1110 | T(seed), T(seed+1), P(seed+11,seed+10) |
| either first take fault | exact E | first T only |
| either second take fault | exact E | both T calls; no P |
| pair fault | exact E | both T calls, P |
| blocked mark fault | exact E | both T calls, P, M |

Arithmetic wraps as DEX signed 32-bit arithmetic. For ordered/blocked, flag != 0
makes every helper fault unreachable. Native models are expectations, not an
executed helper implementation.

`constructed(I)Lfixture/CoverBox;` adds allocation before the same ordered
argument producers and a concrete constructor that initializes Object, writes
left, writes right, then returns. On success, left is seed+10 and right is
seed+11; effects are T(seed), T(seed+1). Allocation failure skips both calls;
argument failure skips initialization/publication; constructor failure prevents
publication. Object initialization is assumed normal except in an explicitly
selected constructor-fault scenario. No VM fault was executed.

The added `tree(I)I` challenge places two independent pair subtrees in an outer
pair and reuses registers for successive result values. Its source effects are
T(seed), T(seed+1), P(seed+10,seed+11), T(seed+2), T(seed+3),
P(seed+12,seed+13), P(101*seed+1011,101*seed+1213); its result is
10201*seed+102313, with wrapping arithmetic throughout. Each of its seven call
faults stops at the corresponding exact effect prefix. `barrier(I)I` puts mark
between the independent takes: T(seed), M(seed+10), T(seed+1),
P(seed+10,seed+11), with the ordinary result. No cover can move the first take
across mark. These expectations were recorded before inspecting the added DEX.

## Structural acceptance

Every installed Evaluation owns one exact expression read occurrence and its
original operation names that occurrence through nested_at. The original read's
ValueId/StateId and sites remain inspectable. P9 retains exactly one statement,
operation and result-commit anchor for each nested producer. The blocked subject
keeps all effects in source order after refusing compact condition ownership.

Baseline: `93580c4`. This input challenges the shared expression-ownership
migration in [the recovery-core design](../../../../docs/diana/UNIFIED_RECOVERY_IR_DESIGN.md).
It adds a joint operand forest, short-circuit routing, a refusal after speculative
argument composition, and reversed effectful arguments. Architecture compatibility
and new recovery capability are separate observations. Historical fixtures are
deferred during the structural rewrite at the operator's request.

## Preparation

At expectation authorship, preparation and inspection were not run. Use the
locally available assembler, without treating its version as an admission gate:

```sh
mkdir -p .cache/fixtures/joint-expression-cover
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/joint-expression-cover/classes.dex \
  quality/fixtures/atoms/joint-expression-cover
cp .cache/fixtures/joint-expression-cover/classes.dex quality/fixtures/atoms/joint-expression-cover/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/atoms/joint-expression-cover/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared on 2026-11-10 with Smali 2.5.2, API 24 and Temurin OpenJDK
21.0.12.1+1. The two original classes and five executable JointCover subjects
were resolved through ordinary Engine inspection. Baseline `1f49236` and the
containing ownership migration deliver identical P9 text, issues and member/span
counts on this new input. Four targeted `cargo nextest run --locked -p diana-decomp
--test joint_expression_cover` cases pass. Engine Clippy (library, dbg_shell and
that test, all features) and the dev dbg_shell build pass. No release duplicate,
historical fixture suite or behavioral helper executor was run.

The extended tree/barrier challenge uses the same assembler environment. Compare
`c2e1ac5` with the containing shared-suffix refactor on that newly prepared input.
Five nextest cases pass, including sixteen normal scalar-plan scenarios and
sixty-eight reachable helper-fault scenarios with exact event-prefix checks.
Original identities, eleven members and 533 source spans remain available;
ordinary/tree return forests improve without a new recognition rule. No old
fixture run, condition/constructor executor or VM run is claimed.
