# Constructor trees with effects and handler state

## Independent challenge expectations

Authored after the initial repair to `atoms/constructor-argument-chains`, before
candidate inspection of this DEX. The same allocation-before-argument trigger now
contains a branching expression tree (two reads feed an add and a finish call),
a later argument using the original parameter, a protected message-call chain,
and deliberate order, sharing, handler and snapshot refusals. Sources are
independently authored. Original owner `Lfixture/ArgumentEffects;`.

Subjects: tree(II)Lfixture/Point;, caughtError(I)V,
reversed(I)Lfixture/Point;, shared(I)Lfixture/Point;,
interleaved(I)Lfixture/Point;, differentProtection(I)Lfixture/Point;,
changingSnapshot(I)I. The prepared DEX includes Point's concrete constructor:
Object initialization, x store, y store, return.

Native helper model resets per scenario. read(stage,fault) records R(stage,fault),
throws a distinct E(stage) when fault & (1 << stage), otherwise returns stage+7.
finish(value) records F(value) and returns value*2 with wrapping int arithmetic.
message(value) records M(value), returns the exact string "value=" + value.
A scenario may select an independent finish/message failure E(F)/E(M).
note(value,error) records N(value,error) and returns, or throws a separately
selected E(N). No helper is assumed pure. External constructors are normal in
success scenarios; input allocation/initialization failures retain their original
positions. Every scenario starts with no prior calls or object publication.

Java explains natural subjects and the guards' original order. allocatePoint and
initializePoint in the explanation denote new-instance and invoke-direct only;
they are not runtime calls in Smali or extra helpers. Java cannot directly express
those separate uninitialized-object operations. differentProtection has per-site
handlers rather than one Java catch: allocation, first read and initialization
enter A (note10); only the second read enters B (note20). Smali is authoritative.

| Scenario | Expected result and effects |
|---|---|
| tree(5,0) | P(x30,y14); R0,R1,F(15),R2; initialization after all arguments |
| tree(5,1) | null; R0,N(5,E0); later reads/finish/initialization skipped |
| tree(5,2) | null; R0,R1,N(5,E1); finish/later read/initialization skipped |
| tree(5,4) | null; R0,R1,F(15),R2,N(5,E2) |
| tree finish failure | null; R0,R1,F(15),N(5,E(F)); R2 skipped |
| tree allocation failure | null; N(5,E); no argument evaluation |
| tree initialization failure | null; R0,R1,F(15),R2,N(5,E); no initialized publication |
| tree note failure | exact E(N), replacing the caught fault |
| caughtError(0) | void; R0,M(7),N(0,fresh IAE("value=7")) |
| caughtError(1) | void; R0,N(0,E0); message/initialization skipped |
| caughtError message failure | void; R0,M(7),N(0,E(M)); initialization skipped |
| reversed(0) | P(x7,y8); R1 then R0; must stay explicit |
| reversed(2) | E1 after R1; R0 skipped |
| reversed(1) | E0 after R1,R0 |
| shared(0) | P(x7,y7); exactly one R0; must stay explicit |
| interleaved(0) | P(x14,y8); R0,R1,F(7); must stay explicit |
| interleaved(2) | E1 after R0,R1; finish skipped |
| differentProtection(0) | P(x7,y8); R0,R1; must stay explicit |
| differentProtection(1) | null; R0,N(10,E0) |
| differentProtection(2) | null; R0,R1,N(20,E1) |
| differentProtection allocation/initialization fault | null; N(10,E), before/after arguments respectively |
| changingSnapshot(0) | return1; R0; original constructor still executes |
| changingSnapshot(1) | return1; R0,N(1,E0) |
| changingSnapshot allocation failure | return0; N(0,E); no read |
| changingSnapshot initialization failure | return1; R0,N(1,E) |

The snapshot guard changes a handler-observed literal between allocation and the
remaining throwing members. Their saves cannot become one common prelude even
though the constructor's normal arguments are available. Order guards preserve
allocation placement and call identity rather than moving a call or duplicating
one shared result. All selected errors prevent later work; note's own error
escapes without suppression or retry. These are independent expectations, not
observations obtained from Diana output.

## Preparation status

Baseline `e95b6a4` and the containing repair inspect the same prepared DEX through
ordinary dbg_shell. At expectation authorship, assembly, candidate inspection, semantic
review and fixture checks had not run. No whole-method behavioral executor or
VM/device result is claimed.

## Prepared input and current review

Prepared on 2026-11-09 with the locally available Smali 2.5.2 assembler,
API24 and Temurin OpenJDK 21.0.12.1+1. Java is an explanation; Smali is the
assembly input. The same prepared classes.dex is inspected on `e95b6a4` and the
containing repair, without installing an assembler for inspection or tests.

```sh
mkdir -p .cache/fixtures/constructor-argument-effects
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/constructor-argument-effects/classes.dex \
  quality/fixtures/atoms/constructor-argument-effects
cp .cache/fixtures/constructor-argument-effects/classes.dex quality/fixtures/atoms/constructor-argument-effects/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/atoms/constructor-argument-effects/classes.dex \
  --exec '{"cmd":"p5-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo nextest run --locked -p diana-decomp --test constructor_arguments
```

Fixture-local `review.md` records primary LLM semantic reasoning for every subject
and its helper/model boundary. Six grouped ordinary-Engine tests passed. The
argument evaluator actually checks the final numeric argument vector [30,14]
and the three read-fault prefixes on the effect challenge; it does not execute
whole methods, catches, resource helpers or external constructors.

The same recovery checks were run on the pre-fix engine for the Point chain and
protected exception construction: both rejected the baseline's absent selected
construction. That two-test sensitivity run is intentionally failing; the other
four groups were filtered out there. The complete candidate dev run has 132
passing tests and zero skips on Rust 1.98.0 with nextest 0.9.149. Format and
workspace Clippy passed. The workspace dev binary/example build passed.
The old protected-loop test keeps its independent ordering/control checks and
now locates the original executable throw span instead of a temporary name.

The required comparison uses all 101 prior Quality inputs plus the two new
fixtures: 103 DEX inputs. Both revisions retain 167 rendered classes and 797
methods (727 expressed, 70 declaration-only), with no class/member/render stop.
P9 text is exact for 93 inputs. The ten changed inputs are these two fixtures,
`algorithms/algo-string`, `branches/abrupt-continuation-tails`,
`branches/terminal-guard-arms`, `exceptions/finally-overrides-exit`,
`exceptions/finally-return-capture`, `exceptions/protected-loop-exits`,
`exceptions/sync-cleanup-rethrow`, and `exceptions/typed-catch-shared-cleanup`.
All changed paths receive renewed LLM review; unchanged input/output reuses prior
independent reviews. Original P1/P3/P4 publications remain exact; P5 construction
products intentionally change. These counts describe coverage, not quality scores.

No full-method behavioral executor, VM/device run or Java recompilation was run.
Release regression was not run, following the operator's dev-only instruction.

Dev CLI get-method additionally confirmed equal text/JSON result text and natural
construction on point, bodyError, tree and caughtError, using the original
selectors above. The final build and unchanged-path comparison use the same
source as the containing repair. The reviewed guard-method output stays exact
on the final candidate. No checker or expectation was weakened to admit them.
