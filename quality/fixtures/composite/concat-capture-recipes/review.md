# Fixture: concat-capture-recipes

## Current LLM semantic review

The original Java explanation, actual Smali and same prepared DEX are compared
with ordinary Engine outputs on baseline `8785d2f` and the candidate.
The model assumes the named standard bootstrap contract. Recovery labels that
inference and retains the original Custom call, prototype, encoded events and
argument list. Runtime linkage, arbitrary library replacement, allocation
failure and asynchronous exceptions remain unproven.

The same paths were rechecked in dev after integration with main `5a6ba48`.
The latest coroutine, protected-loop and shared-tail behavior remains exact
against that baseline; only the two concat inputs change text. All 115 engine
checks pass and changed inputs retain exact upstream P1/P3/P4/P5/P6 products.
The primitive/recipe content judgments and helper/loop/exception reasoning below
remain valid under the same explicitly stated standard-bootstrap assumptions.

### Semantics

| method | judgment and scope |
|---|---|
| `mark`, `read` | ~ Same helper behavior: wrapping trace update before fault selection, then the same left/right result or constructed ISE. |
| `numeric` | ~ A leading empty String prevents 1+2 arithmetic; char, boolean and long conversions retain their established views and order. |
| `recipe` | ~ Dynamic values and static constants occupy the original tag positions; UTF-16 and a tag inside a static String remain literal. |
| `constants` | ~ Static long 7, boolean true and char A have their one-time standard String conversions, without dynamic conversion calls. |
| `single` | ~ The String-conversion prefix makes null become the String "null", retaining the original invocation. |
| `charAsInt` | ~ The original call parameter is int; the char header value is projected numerically as 65. |
| `moved` | ~ The two original word moves remain; the char cast and boolean value test recover the original ABI views. |
| `floating` | ~ Float/double operand values remain typed and ordered, including signed zero and nonfinite parameter values. Content execution is not modeled by the small checker. |
| `effectful` | ~ Read 0 then read 1 each execute once before concat; the captured values are consumed in 1/0 order. |
| `loop` | ~ Read/concat precede the original update; a reached ISE exits through the original handler with "caught" and no further update/read. |
| `object` | ? The explicit invocation and arguments stay intact; arbitrary object conversion behavior is not reviewed. |
| `nonfiniteLiteral` | ~ Original Float bits and explicit invocation remain. The unsupported literal is not changed into integer addition. |
| `badRecipe`, `badBootstrap` | ? Invalid linkage is not executed; original metadata/call are preserved with the unavailable-presentation reason. |
| `unknownFactory` | ? Original opaque bootstrap/call remain; absent external facts do not become a standard-factory assumption. |

For numeric(1,2,'A',true,3), the original argument sequence represents the text
"12Atrue3". The emitted prefix puts every operand in String-conversion context;
without it an initial numeric addition would produce the wrong prefix. The
char view stays char rather than integer 65. Conversely, charAsInt's prototype
is int, so its explicit numeric cast gives "65". The independent content cases
exercise both directions. The first draft missed the header's char display when
early local storage still looked integral; the content check rejected it before
acceptance. The owning P7 selection now uses actual parameter/local spelling
separately from P4's value-preserving range view.

The recipe begins with static "tag", then literal brackets and dynamic name,
count and mark at their original positions, ending with a static one-unit String.
Its final unit 0001 is not reinterpreted as another dynamic tag. Null name uses
standard String conversion; the lone surrogate stays one original UTF-16 unit.
Static constant conversion comes from decoded values and never calls an arbitrary
object toString repeatedly. Unsupported constants retain the explicit form.

Effectful(-1) marks 10 then 11, captures left then right, and returns right:left.
Fault 0 stops after mark 10; fault 1 stops after both marks before concat. The
baseline can inline the last helper at an argument read; the candidate leaves
both calls at their original statement positions. No call is duplicated and no
new conversion helper is inserted. The loop reads the current index, concatenates
the current result/index/value, then updates index and result at their original
positions. A failing read reaches the same ISE handler, assigns caught and exits;
limit 0 executes no read. The recovered enclosing try includes only nonthrowing
control/update portions around the original protected sites. Catch priority and
the explicit exception-chain facts remain. These are path readings, not full
method execution results.

The initially malformed assembly and the char/int draft were rejected. The
prepared input now has correct outgoing-word metadata without changing any
instruction or call-site content. Six dev-directed tests pass, including six
independent projection-content scenarios. Their coverage and exclusions are
recorded in the README; they do not establish VM/linkage or full-method behavior.

## Rounds

- R1 2026-11-09: Added captures, recipes, primitive views, loop/catch and explicit guards after the general P7 repair; corrected fixture metadata and a detected char/int interpretation error before acceptance.
