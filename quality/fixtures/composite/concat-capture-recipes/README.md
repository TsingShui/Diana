# Captured arguments and StringConcatFactory recipes

## Independent expectations

This fixture was authored after the first concat presentation repair and before
its first candidate inspection. Discovery: `ComplexIndyConcat.simple`, `many`
and `insideLoop` in `composite/complex-indy-concat`. The retained mechanism is
an original Custom call to StringConcatFactory. Added interactions are leading
numeric operands, char/boolean/wide views, static recipe constants, captured
effectful values in a different use order, loop copies and a typed catch.

The Java explanation and Smali are independently authored for this repository.
Smali is the assembly input. Java explains the successful standard-bootstrap
paths; the three native explanations correspond to invalid or opaque linkage
that Java expressions cannot state. Original owner: `Lfixture/ConcatCapture;`.
Subjects and full prototypes are declared in the assembly beside this file.

The reference model assumes the named standard StringConcatFactory contracts
described by the [JDK 21 API](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/lang/invoke/StringConcatFactory.html).
That specification guides protocol interpretation; it supplies no external
declaration or runtime input to Diana. Runtime linkage, allocation failure,
asynchronous exceptions and arbitrary library replacement are not executed or
proved here. Recovery retains the original Custom call and marks its inference.
The name of the generated call is arbitrary; only the exact bootstrap identity
and signature select the interpretation.

Each scenario starts with trace 0. Concrete `mark(event)` appends the event as
wrapping int `trace * 31 + event`. `read(stage, fault)` marks `10 + stage`, throws
ISE("read") when stage equals fault, otherwise returns "left" at stage 0 and
"right" at other stages. Only reached helper calls can fault.

| Subject and inputs | Expected result or exit | Ordered mark arguments |
|---|---|---|
| `numeric(1, 2, 'A', true, 3L)` | "12Atrue3", not "3Atrue3" | none |
| `numeric(-1, 0, '\u0000', false, -2L)` | "-10\u0000false-2" | none |
| `recipe("x", 2, 'A')` | "tag[x:2]A\u0001" | none |
| `recipe(null, -3, '\ud800')` | "tag[null:-3]\ud800\u0001", retaining both UTF-16 units | none |
| `constants("x")` | "x7trueA" | none |
| `constants(null)` | "null7trueA" | none |
| `floating(0.5f, 2.0d)` | "0.5:2.0" | none |
| `floating(-0.0f, positive infinity)` | "-0.0:Infinity" | none |
| `single("x")` | "x" as a concatenation result | none |
| `single(null)` | "null", not a null reference | none |
| `moved('A', true)` | "Atrue" through original machine-word temporaries | none |
| `charAsInt('A')` | "65", not "A"; the original call parameter is int | none |
| `effectful(-1)` | "right:left" | 10, 11 |
| `effectful(0)` | throw original helper ISE before stage 1 | 10 |
| `effectful(1)` | throw original helper ISE before concat | 10, 11 |
| `loop(0, 0)` | ""; no read is reached | none |
| `loop(2, -1)` | "<0:left><1:right>" | 10, 11 |
| `loop(2, 0)` | "caught" at the first read | 10 |
| `loop(2, 1)` | "caught" after the first successful concat/update | 10, 11 |
| `loop(1, 1)` | "<0:left>"; selected later fault is unreachable | 10 |
| `object(null, 2)` | standard-model content "null2"; keep the original explicit call | none |
| `nonfiniteLiteral()` | standard-model content "Infinity"; retain the original exact Float bits | none |
| `badRecipe("x", 2)` | standard linkage fails because a dynamic placeholder is missing; no fabricated result | none |
| `badBootstrap("x")` | keep the invalid bootstrap return signature and original call | none |
| `unknownFactory("x", 2)` | Unknown: the external alias bootstrap is not in this input | none |

Recipe tags consume dynamic arguments and static constants in their original
orders. A tag inside a static String constant is literal content, not another
recipe tag. Static long, boolean and char constants need one-time standard String
conversion; arbitrary object or class constants have no assumed conversion here.
The effectful method first captures stage 0, then stage 1, and consumes their
values in reverse order. Recovery must not relocate either helper invocation
into a concat operand. The loop's String result, original update and catch
remain at their supplied positions. The nonfinite literal cannot become an
ordinary hexadecimal numeric addition, and the invalid recipe cannot drop its
second original argument.

## Preparation and review status

Use a locally available Smali assembler with API 28; prepare once under
`.cache/fixtures/concat-capture-recipes/`, and publish only the successful
`classes.dex` here. Inspect the same DEX through ordinary Engine `dbg_shell`
on baseline `8785d2f` and the candidate on `work/indy-concat`. No assembler is
downloaded implicitly and no per-fixture tool version is pinned.

Prepared with Smali 2.5.2/API 28 and Temurin OpenJDK 21.0.12.1. The first assembly
underallocated `outs_size` for Custom calls, so the original input checks exposed
`InsufficientOutgoingWords`. D8 preserved that metadata and was not the solution.
The shared fixture preparation tool now reads actual invoke-register counts with
the assembler's existing dexlib2 reader and corrects only deficient code-item
metadata plus required DEX integrity fields. No dummy invocation, instruction
rewrite, reference rewrite or weakened Engine check is used. This prepared DEX
has no input issues in the ordinary Engine checks. These are observed tool
versions, not per-fixture pins or production dependencies.

```sh
mkdir -p .cache/fixtures/concat-capture-recipes .cache/quality-tools
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 28 \
  -o .cache/fixtures/concat-capture-recipes/assembled.dex \
  quality/fixtures/composite/concat-capture-recipes
javac -cp "$SMALI_CLASSPATH" -d .cache/quality-tools quality/NormalizeSmali.java
java -cp ".cache/quality-tools:$SMALI_CLASSPATH" NormalizeSmali \
  .cache/fixtures/concat-capture-recipes/assembled.dex \
  .cache/fixtures/concat-capture-recipes/classes.dex
cp .cache/fixtures/concat-capture-recipes/classes.dex \
  quality/fixtures/composite/concat-capture-recipes/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/composite/concat-capture-recipes/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

## Executed checks and evidence boundary

Six directed tests in `crates/diana-decomp/tests/concat_factory.rs` pass in dev.
The independent content checks evaluate six cases across five parameter/constant
projections: numeric, recipe, constants, single and charAsInt. They use supplied
parameters and the published concat parts, respecting numeric addition, char
conversion and a String-valued result. Negative test-owned values distinguish
numeric addition, character conversion and null from its converted String.
The evaluator does not execute stores, helper calls, loops, exceptions or full
method scheduling. Those paths receive structural observations and the source
review in `review.md`, not executed behavioral PASS claims. The checks also
require input issue absence, preserve every original call/capture anchor and
refuse to relocate helper producers into a concat operand.

The complete dev engine run currently passes 97 tests across 21 binaries with
no skips. No final-plan method executor, device run or Java recompilation ran.

## Iteration and regression record

The original discovery and this new challenge are inspected on baseline
`8785d2f7d504ceed9afdd76066d27284f52259c9` and the candidate in the independent
`work/indy-concat` branch. Candidate engine source snapshot:
`0e14a62042828d7007970ae7ab992792f2b93f1c767ccf474d485c3856743783` (SHA256 over
sorted repository-relative Rust source names and contents). This identifies the
reviewed uncommitted source; it is not an input-verification gate.

The required old set is all 91 prepared Quality inputs at that baseline, including
earlier repair/challenge and guard cases. Adding this challenge gives 92 inputs:
143 classes, 659 expressed methods and 50 no-code declarations. Same-input dev
inspection reports no stopped member, class error, retention error or render
issue. Only `complex-indy-concat` and this new challenge changed text; the other
90 old inputs are exact, supporting reuse of their existing path reviews. All
changed paths are reread in this review and the updated discovery review.
The changed inputs' P1/P3/P4/P5/P6 products remain exact between revisions.
The release profile is not part of this iteration's acceptance.

Dev workspace build, full workspace Clippy with all targets/features and warnings
denied, format, shell syntax and desensitization checks pass. The dev CLI text and
JSON presentations carry identical inferred concat text. The preparation byte
comparison confirms that normalization changes only 13 code-item headers and
required integrity fields, with instruction and reference bytes identical.
The original obstruction improves, the newly authored challenge generalizes the
same mechanism, the recorded path assumptions are explicit and prior effects and
availability remain. This is the scoped LLM iteration decision plus actual
projection/structural checks, not machine acceptance for arbitrary VM behavior.

Integration rechecks the complete combination with main
`5a6ba4821e7297085fc168c3f3604dae95d43282`, preserving the concurrent coroutine,
protected-loop and shared-tail repairs. The combined engine snapshot is
`680406777b796ce6afd5c0c94866e150b906a7e67dc3f5f65d69cf7eae518cc0`.
All 115 dev nextest checks across 25 binaries pass without skips. Dev workspace
build, Clippy, format and desensitization pass. Same-input comparison covers 99
fixtures, 155 classes, 691 expressed methods and 61 no-code declarations; only
the original concat fixture and this challenge change text. The other 97 texts
are exact against that main, with no member loss or rendering errors. Changed
inputs retain identical P1/P3/P4/P5/P6 products and CLI text/JSON agree. The
recorded concat path reasoning and explicit unsupported boundaries remain valid
in that combination; no full-method VM or linkage execution is inferred.
