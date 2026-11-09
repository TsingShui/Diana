# Fixture: short-circuit-shared-routes

## Current verdict — selected-route recovery, 2026-11-09

The candidate generalizes the `ExprCompare.either(ZZ)Z` repair to three
effectful tests, two selected routing paths and exclusive result-arm effects.
It preserves guards for an effectful selected path, an executable Phi copy, a
foreign normal entrance, a cycle and an uncomposable later test prelude.
The initial iteration compared ordinary Engine `dbg_shell` on baseline
`29f1b82` and the selected-route candidate. Integration rechecks the same
prepared inputs with dev builds of main `7354242` and the combined change.

The path judgments below remain valid in the combined output. The `probe`
fault arm, `cyclic` early return, `effectful` false return and `copied` zero
replacement return now use main's terminal guards. Those guards preserve the
same early exits; later work remains on only the original surviving path.
The mark-before-fault order, skipped probe calls, copy placement and divergent
backedge are unchanged. The terminal-guard fixture itself has exact text against
main, and all 91 dev engine checks pass. These are supporting observations,
not final-plan behavioral execution.

### Semantics

These are LLM source/Smali/output judgments, with the helper assumptions and
independent scenarios in [README](README.md#independent-expectations).
They are not executed behavioral results.

| method | semantics |
|---|---|
| `mark` | ✔ Reads trace, multiplies by 31, adds the event and writes once, preserving int wrapping. |
| `probe` | ~ Marks before checking the fault; the fault throws the constructed ISE, otherwise the stage mask selects the same boolean. |
| `ordered` | ~ Reached probes execute once in stage order; selected paths reach one true arm and all false paths reach one false arm before the shared tail. |
| `blocked` | ~ Mark 25 remains before the second probe on only the first-false path; the complete explicit alternative preserves both original selections and the common result. |
| `effectful` | ~ Mark 30 remains exclusive to the first-false, second-true path. |
| `copied` | ~ First true returns the original value; first false stores the replacement before testing it, returning -1 only for replacement zero. |
| `foreign` | ~ The skip edge reaches true without either probe; normal probe paths retain their original stages and result arms. |
| `cyclic` | ~ First true returns 7, both false return 9 and first false/second true reaches the retained unbounded loop. |

### Path reasoning

`ordered` evaluates stage 0 before any branch selection. If it is true, the
two later calls and their possible faults are skipped. Stage 1 is reached only
after stage 0 is false; its two empty goto blocks carry no work or copies and
reach the same mark-40/result-7 arm. Stage 2 is reached only after both earlier
tests are false, and its separate empty route reaches that same arm. All false
inputs execute mark 50 and result 9. Both results execute mark 60 once before
returning. This gives every ordered trace recorded in the README. A reached
probe fault occurs after its own mark and propagates before either result-arm
effect or mark 60. Faults at skipped stages never execute. The candidate's
left-to-right `probe(0) || probe(1) || probe(2)` preserves those choices, calls,
argument values and exception positions.

`blocked` retains mark 25 between the first false selection and the second
probe, so it cannot become an ordinary boolean operand. P6 owns the selected
route; P7 selects the delivered explicit child/edge schedule rather than hiding
the work. The false result and true result both reach the common return with
their original copies. The first assembly's two independent returns did not
trigger this product; the directed test rejected that preparation. The revised
assembly has the same behavior and a confirmed common-result trigger.

`effectful` keeps mark 30 after the second true selection, before mark 40. It
does not execute mark 30 when the first probe is true or both are false. The
candidate refuses to treat the selected block containing that call as routing;
the output remains identical to the baseline. `copied` likewise remains
identical: its empty goto has a confirmed executable P5 copy for the changed
value entering the shared result. That copy is retained, not relocated to the
first selected edge or replaced by a boolean result.

`foreign` has a skip edge entering the route from outside the two-probe
interface. The route is therefore not made silent as part of those probes.
Its output is unchanged, retaining the early jump and both probe guards.
`cyclic` retains a two-goto cycle as `while (true)`. Neither graph traversal nor
condition selection turns that divergence into the true result. Its unchanged
output supports the source judgment; no execution of the divergent path ran.

The helpers retain their original names, arguments, mark-before-fault order,
int operations and throwable construction. Their output texts are exact against
the baseline. The scenarios exclude asynchronous/linkage/allocation faults and
concurrent trace mutation; behavior outside those assumptions is not reviewed.

### Recovery and evidence boundary

The baseline `ordered` prints separate predicates, route gotos and labels.
The candidate prints one compound condition with the same result arms and tail.
Every original normal edge is anchored once; compact routing also retains its
goto terminal and transfer at zero width. No operations or effects are hidden
on a routing path. The blocked prelude and floating regression use the existing
complete explicit alternative, preserving source child roles and destinations.

Eight directed structural tests cover the seed, challenge and guards in
`crates/diana-decomp/tests/short_circuit_routes.rs`. Actual broader execution
records are in the fixture README. Structural observations and this LLM reading
do not establish a machine final-plan behavioral PASS. No final-plan executor,
device run or Java recompilation ran.

## Rounds

- R1 2026-11-09: Authored the three-test routing challenge and guards after the owning P6 repair; reread normal, exceptional, copy, foreign-entry and divergent paths against the same baseline/candidate DEX.
