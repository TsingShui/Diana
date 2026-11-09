# Ordered constructor argument chains

## Independent expectations

Independently authored Java explanation and Smali assembly input. No third-party
code or external declarations are consumed. Original subject owner:
`Lfixture/ConstructorArguments;`. Subjects: `point()Lfixture/Point;`,
`caughtPoint()Lfixture/Point;`, `bodyError(I)V`.

The discovery retains allocation before two independent, three-operation
argument chains: getSystem, getDisplayMetrics, field read. Each intermediate
result has one later use; each complete chain supplies its own constructor
argument. The same calls are intentionally repeated. The caught subject also
places allocation, all argument operations and initialization in one catch-all
range. bodyError puts its string preparation and exception construction in that
same kind of protected corridor. The intended improvement is one natural new,
with the exception construction eligible for the existing throw composition.

Native helpers have this explicit model, reset for every scenario. The first
getSystem returns R0, the second R1, recording G0/G1 in order. R0.getDisplayMetrics
records D0 and returns M0; R1 records D1 and returns M1. M0 has width120, height200;
M1 has width360, height480. Field reads therefore provide120 and480. Resources
may instead be null (the following virtual call throws NPE), or metrics may be
null (the following original field read throws NPE). A selected G/D call can
throw a distinct E. No helper is pure and repeated calls must not be coalesced.
`note(E)` records N(E) and returns unless the scenario specifies its own failure.
Point's concrete constructor calls Object's constructor, stores x then y and
returns; there is no extra body effect. Object initialization is assumed normal
for the listed success cases. Allocation and constructor failure scenarios
retain the original exception positions without claiming to execute a VM fault.

| Subject/scenario | Expected exit and fields | Ordered calls |
|---|---|---|
| point success | new P with x120, y480 | G0,D0,G1,D1 |
| point G0 fault | exact E; no initialized Point publication | G0 |
| point D0 fault or R0 null | exact E or NPE | G0,D0 or G0 |
| point M0 null | NPE before the second G call | G0,D0 |
| point G1/D1 fault | exact E; constructor not called | G0,D0,G1 or G0,D0,G1,D1 |
| point M1 null | NPE; constructor not called | G0,D0,G1,D1 |
| caughtPoint success | P with x120,y480 | G0,D0,G1,D1 |
| caughtPoint any argument fault | null; note gets the exact fault | preceding calls,N(E) |
| caughtPoint allocation/constructor failure | null; note gets the original fault; publication requires constructor success | no argument calls on allocation failure; all argument calls on constructor failure; then N(E) |
| caughtPoint note fails | exact note failure replaces the caught error | preceding calls,N(E) |
| bodyError(0) | void, no allocation or note | empty |
| bodyError(1) | void after note of fresh ISE("body") | N(ISE) |
| bodyError construction/string fault | void after note of exact original fault | N(E) |

Source order is allocation, first argument chain, second chain, initialization,
then publication. Every failed step prevents subsequent steps. The catch applies
to exactly the original protected operations, including allocation and string
loading; note's own failure escapes. No suppression or retry occurs.

## Reproduction status

Expectations were recorded before preparing or inspecting the candidate DEX.
Baseline is main `e95b6a4`. Assemble once and compare that same DEX through
ordinary Engine dbg_shell. At expectation authorship, assembly, inspection, repair, semantic
review and tests had not run. No whole-method behavioral executor is claimed.

## Prepared input and current review

Prepared on 2026-11-09 with the locally available Smali 2.5.2 assembler,
API24 and Temurin OpenJDK 21.0.12.1+1. Java is an explanation; Smali is the
assembly input. The same prepared classes.dex is inspected on `e95b6a4` and the
containing repair, without installing an assembler for inspection or tests.

```sh
mkdir -p .cache/fixtures/constructor-argument-chains
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/constructor-argument-chains/classes.dex \
  quality/fixtures/atoms/constructor-argument-chains
cp .cache/fixtures/constructor-argument-chains/classes.dex quality/fixtures/atoms/constructor-argument-chains/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/atoms/constructor-argument-chains/classes.dex \
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
