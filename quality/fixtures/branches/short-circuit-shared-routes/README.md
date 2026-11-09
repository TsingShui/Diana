# Short-circuit tests with selected empty-goto routes

## Independent expectations

This challenge was authored after the P6 selected-route repair and before its
first candidate inspection. Discovery: `Lfixture/ExprCompare;->either(ZZ)Z` in
`atoms/expr-compare`. Its second true selection reaches the shared true value
through an original empty goto block. The challenge retains that route and adds
three effectful predicates, a longer route, ordered result-arm effects and fault
selection. Guard methods add an effectful route, a real Phi copy, a foreign
entrance and a divergent cycle.

The Java explanation and Smali are independently authored for this repository.
Smali is the assembly input; the Java file explains the same behavior without
specifying an identical instruction layout. Original owner: `Lfixture/OrRoutes;`.
Subjects: `ordered(II)I`, `effectful(I)I`, `blocked(I)I`, `copied(III)I`, `foreign(II)I` and
`cyclic(I)I`; concrete helpers are `probe(III)Z` and `mark(I)V`.

Each scenario starts with `trace = 0`. `mark(e)` appends the event using
32-bit wrapping `trace * 31 + e`. `probe(stage, flags, fault)` first marks
`10 + stage`, then throws `IllegalStateException("probe")` when `fault == stage`,
otherwise returns whether the stage's bit is set. The recorded scenarios use
stages 0, 1 and 2. `fault = -1` disables helper faults. Constructor failure,
asynchronous exceptions, linkage failure and concurrent trace changes are
excluded. There is no external helper model or external declaration input.

| Subject and arguments | Expected exit | Ordered mark arguments |
|---|---|---|
| `ordered(1, -1)` | return 7 | 10, 40, 60 |
| `ordered(2, -1)` | return 7 | 10, 11, 40, 60 |
| `ordered(4, -1)` | return 7 | 10, 11, 12, 40, 60 |
| `ordered(0, -1)` | return 9 | 10, 11, 12, 50, 60 |
| `ordered(1, 1)` or `ordered(1, 2)` | return 7; selected fault is skipped | 10, 40, 60 |
| `ordered(2, 2)` | return 7; stage 2 is skipped | 10, 11, 40, 60 |
| `ordered(0, 0)` | throw probe ISE | 10 |
| `ordered(0, 1)` | throw probe ISE | 10, 11 |
| `ordered(0, 2)` | throw probe ISE | 10, 11, 12 |
| `effectful(1)` | return 7 | 10, 40 |
| `effectful(2)` | return 7; selected path keeps mark 30 | 10, 11, 30, 40 |
| `effectful(0)` | return 9 | 10, 11, 50 |
| `blocked(1)` | return 7; later prelude is skipped | 10, 40 |
| `blocked(2)` | return 7 | 10, 25, 11, 40 |
| `blocked(0)` | return 9 | 10, 25, 11, 50 |
| `copied(1, 5, 8)` | return original value 5 | none |
| `copied(0, 5, 8)` | return replacement 8 | none |
| `copied(0, 5, 0)` | return -1 | none |
| `copied(0, 5, -3)` | return replacement -3 | none |
| `foreign(0, 1)` | return 7 without either probe | 40 |
| `foreign(1, 0)` | return 7 | 10, 40 |
| `foreign(2, 0)` | return 7 | 10, 11, 40 |
| `foreign(0, 0)` | return 9 | 10, 11, 50 |
| `cyclic(1)` | return 7 | 10 |
| `cyclic(0)` | return 9 | 10, 11 |
| `cyclic(2)` | diverge in the original empty-goto cycle | 10, 11 |

Acceptance: `ordered` retains one evaluation of each reached probe in stage
order and recovers one ordered condition despite both selected routing paths.
The true and false effects remain exclusive and precede the common mark 60.
Every original route transfer and terminal retains an origin. A route with work,
edge copies, an additional entrance or a cycle is not silently skipped.
The `blocked` guard retains the admitted selected route but requires P7's complete
explicit alternative because mark 25 cannot become a predicate operand. Its
expectations were authored before inspecting that newly added method.
Its first assembly had two independent returns and did not expose the intended
P6 condition product; the directed check rejected that preparation. The revised
Smali retains the same results and effects through one common result return,
so the selected-route and explicit-alternative trigger is independently testable.
The `copied` guard requires confirmation of an executable P5 copy on the empty
goto, rather than treating a source assignment alone as proof of that trigger.

## Preparation and comparison

Assemble once with a locally available Smali assembler using API 24, publishing
only the successful `classes.dex` beside these sources. Use ordinary `dbg_shell`
on that same DEX for baseline `29f1b823a22db63e0ac656cbd383886b2a3e0d48` and the
candidate on `work/short-circuit-or`. The baseline is the committed source and
does not include unrelated uncommitted changes in the primary worktree.

The actual preparation used Smali 2.5.2, API 24 and Temurin OpenJDK 21.0.12.1.
The assembler was already available locally; no tool was downloaded. These are
observed environment versions, not per-fixture admission pins.

```sh
mkdir -p .cache/fixtures/short-circuit-shared-routes
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/short-circuit-shared-routes/classes.dex \
  quality/fixtures/branches/short-circuit-shared-routes
cp .cache/fixtures/short-circuit-shared-routes/classes.dex \
  quality/fixtures/branches/short-circuit-shared-routes/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/branches/short-circuit-shared-routes/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}' \
  --exec '{"cmd":"p6-all","details":true}'
```

## Execution and iteration decision

Original iteration base: `29f1b823a22db63e0ac656cbd383886b2a3e0d48`. The original engine-source snapshot
has SHA256 `00f4b58dfc900321bd6df06de4bca9cb8f6ab1f950dbfbf492b0e855dd445d4f`,
computed over sorted repository-relative `crates/diana-decomp/src/**/*.rs` names
and their contents. The baseline uses the committed source without this change.

The revised prepared DEX reproduces the selected-route obstruction on that
baseline and recovers the three-test condition on the candidate. Eight directed
structural checks passed after the corrected `blocked` preparation. The complete
engine run, `cargo nextest run --locked -p diana-decomp --all-features`, passed
82 tests across 19 binaries with no skips. The directed implementation is
[short_circuit_routes.rs](../../../../crates/diana-decomp/tests/short_circuit_routes.rs).
These checks inspect ordinary Engine products and output origins; they do not
execute a final-output behavioral plan.

The required old fixture set is every 89 prepared Quality fixture committed at
the baseline, including the prior repair/challenge and guard cases. Adding this
challenge gives 90 inputs. Both revisions processed the same prepared DEX for
each input: 141 classes, 631 expressed methods and 50 no-code declarations,
without stopped members, class errors, retention errors or rendering issues.
Only `expr-compare`, `expr-wide` and this challenge changed text. The other 87
old fixture texts are exact, supporting reuse of their existing path reviews;
every changed method is reread in this review or the two updated atom reviews.
The changed inputs' delivered P1/P3/P4/P5 products are exact between revisions.
P6 changes are reviewed as the repaired owning-stage product, not included in
that upstream equality observation.

`cargo fmt --all -- --check`, full-workspace Clippy with all targets/features
and warnings denied, and `cargo build --workspace --release --locked` passed on
Rust 1.98.0/macOS arm64. The release CLI's `get-class` text and JSON presentations
carry identical text for the seed, including the recovered disjunction.
The repository desensitization check passed; the newly untracked fixture and
test files were also checked against the same token list.

Integration validation uses dev builds against main
`73542421d671be2fc169b72c7233aa0b5b955fd5`, retaining its terminal-guard change.
The combined engine-source snapshot has SHA256
`b27c85412b2537fcd7c3240c5871a486fa004c558ab6d478d37fb9687cd99cfd`.
`cargo nextest run --locked -p diana-decomp --all-features` passed all 91 tests
across 20 binaries with no skips. Dev workspace build, workspace Clippy with
all targets/features and warnings denied, format and desensitization checks
passed. The seed, floating regression, this challenge and terminal-guard fixture
were inspected on the same DEX with baseline/combined dev `dbg_shell` builds.
P1/P3/P4/P5 products agree in all four inputs. The terminal-guard fixture text
is exact against main; the other three inputs retain the reviewed selected-route
changes. Dev CLI text/JSON carry the same seed text. Integration validation
uses the operator's dev-only verification policy; it does not repeat the earlier
full release corpus run.

The LLM path review finds no contradiction under the recorded helper assumptions.
The new challenge demonstrates the same repair with added predicate/effect
interactions, and earlier required recovery/availability is retained. The
iteration is effective within this reviewed scope. No final-plan behavioral
executor, device run or Java recompilation ran; this decision is separate from
machine behavioral acceptance and does not authorize a commit or integration.
