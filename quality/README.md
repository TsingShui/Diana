# Quality fixtures and review

Diana's shared fixture directory is `quality/fixtures/<theme>/<slug>/`.
It holds output-quality references, Java/Smali reproductions and regression
inputs, with prepared DEX beside their sources. Reuse the corresponding theme
or create it when absent; do not create a separate fixture tree.
[FIXTURES](../docs/diana/FIXTURES.md) defines Java/Smali preparation and independent
expectations. Executable engine checks live in `crates/diana-decomp/tests/` and
refer to these inputs. The written-review workflow here answers *did the readable output
get better*, separately from executed behavioral checks.

Fixture reviews MUST primarily use LLM semantic-equivalence review: directly
compare original input/source behavior and helper bodies or explicit helper
models with current decompiled output, path by path. Structural assertions and executed test results support
this semantic reading; they do not replace it. Reviews record qualitative
verdicts and reasoning rather than numerical structure or naming ratings.

Missing effects or case bodies can be semantic
errors; an explicit handler chain or graph can instead be a readability obstacle
that preserves execution facts. The notation alone does not establish a semantic
error. Reviewers explain the concrete effect, identity or recovery difference
and keep their judgment separate from independently executed behavioral checks.

```text
quality/
├── build.sh                     regenerate DEX from Smali or Java/Kotlin sources
├── NormalizeSmali.java          fixture-only outgoing-word metadata correction
├── dump.py                      reference + Diana output side by side, per method
├── lib.py                       fixture lookup and method extraction
├── .gitignore                   keeps the committed fixture DEX in the tree
└── fixtures/<theme>/<slug>/     one focused hypothesis per directory
    ├── <Slug>.java              readable explanation; compile input without Smali
    ├── <Slug>.smali             optional assembly input; takes precedence over Java
    ├── classes.dex              prepared input, committed for reproducibility
    ├── README.md                reproduction setup and independent scenarios, if applicable
    └── review.md                written quality verdict, if applicable
```

A fixture tests **one focused mechanism or interaction**. Atomic inputs isolate
constants, branches or catches; complexity challenges combine relevant structures
while keeping the repaired guarantee and independent expectations identifiable.
The themes are `atoms/`, `branches/`, `loops/`, `exceptions/`, `sync/`,
`classes/`, `generics/`, `algorithms/`, `composite/`, `obfuscated/` and
`kotlin/`; the two catalog tables below use the same grouping, and a slug stays
unique across themes so `--slug` needs no path.

The fixture DEX is committed **on purpose**: inspection and tests then need no
JDK, Android SDK, assembler or network. The generated-output exception covers
`quality/fixtures/**/classes.dex` only.

## Workflow

The primary improvement loop is [EVOLUTION](../docs/diana/EVOLUTION.md#primary-iteration):
discover a fixture problem, repair its general cause, author new more-complex
same-mechanism fixtures, then review the challenge and preserve earlier effects.
A successful review of only the discovery case does not finish an iteration.
The commands and review steps below supply evidence for that loop.

```sh
quality/build.sh                       # after editing assembly/compile inputs
python3 quality/dump.py                # writes .cache/review/<slug>.md
python3 quality/dump.py --slug branch-if2
```

For a fixture with Smali, set `SMALI_CLASSPATH` to a locally available assembler
classpath before running `build.sh`. Its Java file is an explanation, not an
alternative compile input. No tool is downloaded implicitly, and a failed
assembly does not publish a replacement DEX.
For Smali custom invocations, the shared preparation helper corrects `outs_size`
from the assembled invoke operands using the assembler's existing dexlib2 reader.
Only deficient code-item metadata and required DEX integrity fields change;
instructions, declarations and call-site references remain intact. This handles
the Smali writer's missing Custom-call word count without adding dummy calls or
relaxing Diana's input checks.

For Kotlin, compare the Kotlin reference directly with the output. Review `?.`,
`?:`, `!!`, `when`, safe-call chains and lowered coroutine/property protocols by
behavior. Do not invent a Java reference to make the syntax resemble the output.

1. `dump.py` prints every fixture method twice: the Java reference and Diana's
   output for the same method, plus the checks to apply.
2. An LLM reviewer judges semantic equivalence first: results and exception
   identities; evaluation, argument, call and write order; initialization,
   acquisition, close and lock order; catch priority; primary and suppressed
   exceptions. Read helper bodies and compare each relevant normal and exceptional
   path. Record original member identities, concrete expected/observed behavior
   and the reasoning. Mark uncertain and unreviewed paths explicitly rather than
   inferring agreement from similar structure. Then record **facts** (original
   calls, constants, identities), **shape** (reference constructs recovered) and
   **notation** (only-a-decompiler forms in the body) as separate findings.
3. The judgement is written into the fixture itself, `quality/fixtures/<theme>/<slug>/review.md`.
   **Every written quality review carries a per-method `### Semantics` table**:
   `✔` equivalent, `~` equivalent but different shape, `✘` wrong, `?` not established.
   Each judgment includes path reasoning and its helper/environment assumptions.
   A missing or partially reviewed path remains uncertain. Record concrete
   readability and naming findings as prose, not percentages or aggregate ratings.
   Reproduction scenarios and executed check evidence stay in the fixture's README.
4. After a repair, complete EVOLUTION's new complexity challenge and original/prior
   effect review, rather than only re-dumping the seed. The **current verdict is
   rewritten in place** while `## Rounds` gains one line. The file stays bounded:
   it always opens with what is true now, and older rounds remain readable in
   `git log -p`.

A fixture review records the LLM's semantic judgment and its evidence separately
from readability. It supports EVOLUTION's primary iteration decision, but is not
an automated behavioral execution result or executed behavioral-regression acceptance. An LLM judgment MUST NOT override a
failed deterministic check; repair acceptance still follows
[EVOLUTION](../docs/diana/EVOLUTION.md#acceptance-and-regression).

## Source-aligned APK reviews

Public application reviews record pinned APK/source provenance, retrieval coverage,
method-level source/DEX comparisons and evidence limits. They are discovery
records, not whole-application behavioral acceptance or text-similarity ratings.
Downloaded inputs and generated output remain outside version control.

- [AntennaPod 3.12.3](reports/antennapod-3.12.3.md): character-use text disclosure,
  explicit date retries and resource cleanup.

## Fixtures

### Atoms

| slug | covers |
|---|---|
| `expr-constants` | every constant form: int, long, float, double, char, boolean, String, null, class literal |
| `expr-arith` | add, subtract, multiply, modulo, bits, shifts, negation |
| `expr-compare` | `<`, `>=`, `==`, `!=`, `&&`, `\|\|`, `!` |
| `field-basic` | static/instance reads and writes, compound assignment |
| `invoke-basic` | static, virtual, interface, constructor, `invoke-range` args, generic call |
| `array-basic` | sized and literal arrays, byte/boolean arrays, read, write, `.length`, 2D |
| `cast-instanceof` | `instanceof`, `check-cast`, array cast |
| `expr-wide` | `long` shifts/arithmetic, `float`/`double`, NaN comparison, narrowing casts |

### Branches

| slug | covers |
|---|---|
| `branch-if2` | two-register branches with effects on both sides |
| `branch-ifz` | single-register branches, `null` check |
| `branch-if-inverted` | empty-arm inversion, side effects, copies, float comparisons and protected branches |
| `branch-switch-string` | switch over `String`, grouped `case` labels, effects in branches |
| `branch-switch-enum` | switch over an enum with a `default` arm |
| `branch-switch-packed` | packed switch: fallthrough, grouped cases, negative and default arms |
| `branch-switch-sparse` | sparse switch: non-contiguous and large case values |
| `switch-fallthrough` | Java/Smali cases for grouped entries, linear fallthrough, selected-entry copies and boundary guards |

### Loops

| slug | covers |
|---|---|
| `loop-for` | classic `for` with `break` and `continue` in the body |
| `loop-for-multi` | several initialisers/updaters, `for(;;)`, empty update/condition |
| `loop-shared-exit` | nested loops leaving through a flag or a labelled break |
| `loop-map-views` | `values()`, `keySet()`, custom `Iterable`, iterator removal |
| `loop-do-while-sideeffect` | loop conditions that call something on every test |
| `loop-do-while` | `do-while`, `do-while` with `break`, nested `do-while` |
| `loop-tail-shapes` | every tail-test shape next to the head tests that must stay `while` |
| `nested-loop-exits` | Java/Smali reproduction of private nested-loop exit ownership |

### Exceptions

| slug | covers |
|---|---|
| `exception-one-catch` | one catch that returns, one that rethrows, effect order inside `try` |
| `exception-multi-catch` | one multi-catch clause, one caught rethrow |
| `exception-finally` | `finally` on the normal path and on the throwing path |
| `exception-two-resources` | try-with-resources over two resources, `close` + `addSuppressed` |
| `exception-finally-transfer` | `finally` with `return`/`break`/`continue`, try inside catch |
| `resource-cleanup` | Java/Smali reproduction of loop cleanup, nested resources and a shared-handler guard |
| `typed-catch-loop-exits` | Shared catch entries and enclosing cleanup in a multi-exit loop; nonadjacent-alias priority guard |
| `nested-catch-rejoins` | Two nested loops with held cleanup and shared catch rejoin; foreign pre-loop exception guard |
| `branching-finally-suffix` | Whole-loop branching cleanup, unequal-argument guard and a shared outside pending-rethrow route |
| `protected-loop-exits` | Nested protected abrupt corridors, literal cleanup order, cleanup-skipping returns, shared/recovering refusals and raw selector targets |
| `iteration-tail-cleanup` | Nested shared body updates through cleanup, update-skipping backedges and original monitor routing |
| `while-tail-continuations` | Three-level private exits, shared-entry refusal, cleanup before body updates and first-body do-while |
| `monitor-iteration-exits` | Held cleanup before supplied update continuations, mixed release exits and a backedge skipping the update |
| `monitor-split-tails` | Nested captures, three distinct unlocked tails and an executable post-release Phi-copy barrier |
| `abrupt-continuation-tails` | Completed case exits, a shared-default dispatch with later cleanup, and opaque completion guards |
| `switch-shared-tails` | One external shared suffix with downstream bypass entries, case-local loop/catch, outer finally and selected-entry copies |
| `case-entry-joins` | Three predecessors of one conditional body, executable selector-copy and copy-free cycle guards |
| `protected-prefix-frontiers` | Branched guarded cores, optional predecessor frontiers and a genuine multi-entry recovery guard |
| `mutable-iteration-step` | Body-mutated update operands, cleanup and distinct no-body nonlocal condition completions |
| `quiet-condition-copies` | Origin-only condition edges beside actual call-result and parameter Phi assignments |
| `terminal-guard-arms` | Return/throw guards with exact other-edge copies, parent scope, recovering-catch refusal and cleanup |
| `short-circuit-shared-routes` | Ordered effectful disjunctions through empty selected routes; copy, effect, entry, cycle and explicit-prelude guards |
| `concat-capture-recipes` | Inferred Custom concatenation, recipe constants, char/int views, captured effects, loop/catch and explicit unsupported guards |
| `pure-tail-finally` | Whole-operation normal preparation, literal cleanup prefixes, scoped bypasses and late cleanup failure |
| `abrupt-finally-completion` | Saved body returns, cleanup return/throw replacement, always-abrupt/void cleanup, multiple rethrows and unequal/foreign-entry guards |
| `refined-carriers` | Adjacent cast/array carriers and checks that must precede calls and early returns |

### Synchronization

| slug | covers |
|---|---|
| `sync-block` | `synchronized` block, early return inside it, nested blocks |

### Classes and inner types

| slug | covers |
|---|---|
| `class-inner-member` | member inner class, synthetic accessor to a private field |
| `class-inner-static` | static nested class with its own state |
| `class-anonymous` | anonymous class capturing a local and a field |
| `class-enum-body` | enum with constructor state, method and a constant body |
| `class-interface-default` | interface `default` and `static` methods |
| `class-init-order` | field initializers, instance block, static init, `this(...)` |
| `class-bridge` | bounded generics, a real bridge method, wildcard signatures |

### Generic, lambda, stream and iteration

| slug | covers |
|---|---|
| `complex-generics` | generic class, bounded generic method, bridge method, generic call |
| `complex-lambda` | lambdas, capturing lambda, anonymous supplier, method reference, comparator |
| `complex-iteration` | iterator loops, enhanced `for`, `Map.Entry`, `iterator.remove()` |
| `complex-stream` | stream pipelines, method references, `Optional`, `IntStream` |

### Algorithms

| slug | covers |
|---|---|
| `algo-sort` | insertion, quick and merge sort |
| `algo-graph` | BFS, DFS and connected components on an adjacency matrix |
| `algo-dp` | edit distance, longest increasing subsequence, knapsack |
| `algo-tree` | binary search tree: recursive insert/height/inOrder, iterative contains |
| `algo-string` | KMP prefix/search, word frequency over `HashMap`, reversal |
| `algo-matrix` | matrix multiply, transpose, spiral traversal |

### Composite flows

| slug | covers |
|---|---|
| `complex-flow` | enum state machine: switch, loop with break/continue, try/finally, ternary |
| `complex-exceptions` | nested `try`/`finally`, caught rethrow with cause, TWR in a loop, multi-catch |
| `complex-inner` | anonymous class, member inner, static nested, generic bag |
| `complex-switch-nested` | `switch` inside a loop inside `try`, labelled `break`/`continue` |
| `complex-indy-concat` | `invoke-custom` string concatenation (`javac.args` + `d8.args`) |
| `complex-indy-lambda` | lambdas and every method-reference kind kept as `invoke-custom` |
| `complex-map-nested` | `Map<String, List<Integer>>`, `computeIfAbsent`, `getOrDefault`, entry loops |
| `complex-recursion` | mutual recursion, accumulator recursion, memoised Fibonacci |
| `complex-annotation` | runtime annotations with array, enum, class and default values |

### Obfuscated

| slug | covers |
|---|---|
| `obf-flow-r8` | R8-obfuscated `complex-flow` (enum switch, loop, try/finally, ternary) |
| `obf-dp-r8` | R8-obfuscated `algo-dp` (2D arrays, nested loops, `Math.min`) |
| `obf-exceptions-proguard` | ProGuard-obfuscated `complex-exceptions` (nested try/finally, TWR in a loop, multi-catch) |

### Kotlin

| slug | covers |
|---|---|
| `kotlin-null-safety` | `?.`, `?:`, `!!`, safe-call chain, `@Metadata` |
| `kotlin-data-class` | data class, `componentN`, `copy`, `copy$default`, `equals`/`hashCode`/`toString` |
| `kotlin-sealed-when` | sealed hierarchy, exhaustive `when`, smart casts |
| `kotlin-object-singleton` | `object` with `INSTANCE`, companion object, synthetic accessors |
| `kotlin-property-delegate` | `by lazy`, custom `ReadWriteProperty`, custom setter, `lateinit` |
| `kotlin-suspend-state` | suspend functions, continuation state machine, suspend inside try/loop |
| `suspend-loop-captured-effects` | one suspend loop with captured arithmetic, mutable checksum, effects, and divergent reentry boundaries |
| `suspend-two-step-branch` | two suspend sites, a conditional second call, early return, and distinct resume values |
| `suspend-loop-two-sites` | two resume entries inside a loop, between-site effects, early loop exit, and repeated captures |
| `suspend-finally-two-sites` | two suspend sites, resumed failures, physical marker returns, and normal/exceptional cleanup |
| `suspend-default-shared-capture` | effectful dispatch default and a live seed captured across both suspend sites |
| `suspend-three-step-effects` | three suspension sites, simultaneous captures, two early returns, shared/private marker returns, and normal-only effect boundary |
| `kotlin-range-loops` | `until`, `downTo`, `step`, `repeat`, `forEach`, labelled `break@` |

Each fixture's `review.md` records its qualitative LLM verdict and evidence.

## Reviews

### Atoms

| fixture | headline |
| --- | --- |
| `expr-constants` | `char` prints as `120`, class literal becomes a temp assignment |
| `invoke-basic` | all four call forms exact; generics and constructor temporaries differ |
| `field-basic` | `this.`/`X.` qualification and `+=` expansion |
| `cast-instanceof` | correct; casts go through an extra `Object v1 = p0;` |
| `expr-arith` | correct; each expression becomes `int v = (…); return v;` |
| `array-basic` | correct; array creation/literals go through a temporary |
| `expr-compare` | predicates expanded to temporaries plus `if` |
| `expr-wide` | all correct; `cmpl/cmpg` print as `compare<CmplDouble>(…)` |

### Branches

| fixture | headline |
| --- | --- |
| `branch-ifz` | branches kept; losses are naming and field qualification only |
| `branch-if2` | correct but inverted (`<` → `>=`) and split into early returns |
| `branch-if-inverted` | inverted empty-arm controls keep origin anchors; float and exception notation remains explicit |
| `branch-switch-string` | two-level hash/equals switch plus `graph<MultipleEntry>` |
| `branch-switch-enum` | `ordinal()` switch correct, temporaries and `default` order differ |
| `branch-switch-packed` | fallthrough and grouped cases correct; case bodies move into `graph<MultipleEntry>` |
| `branch-switch-sparse` | correct; d8 lowers it to a packed table and every hole is listed |

### Loops

| fixture | headline |
| --- | --- |
| `loop-for` | no `for`; `break`/`continue` are labels and gotos |
| `loop-for-multi` | multi-init/update and empty segments all become independent assignments |
| `loop-map-views` | map views and custom `Iterable` become `Iterator` + `Object` + cast |
| `loop-do-while-sideeffect` | side-effecting conditions print in the loop body with an inverted `break`; the checked value now prints as a cast on its use, so the whole class is plain Java |
| `loop-shared-exit` | private exit work stays in the branch; nested and labelled breaks replace the gotos |
| `loop-do-while` | `do-while` recovered; `v1 = (v1 + 1)` against `index++` dominates the diff |
| `loop-tail-shapes` | tail tests correct both ways; remaining `nop` and goto-based exits |

### Loop semantics evidence

Executed differential check, 2026-11-08: original fixture methods against Diana's rendered methods over generated inputs (equal return values, exception classes, mutated arrays, matrices, deque contents and static state). 3054 comparisons across `loop-for`, `loop-for-multi`, `loop-do-while`, `loop-do-while-sideeffect`, `loop-shared-exit`, `algo-sort`, `algo-matrix` and `algo-dp`, 0 mismatches; `knapsack` contributes 1568, `quick` 896, `multiply` 64 and `editDistance` 64. All eight classes render as plain Java. The checker is sensitive: a changed loop bound in `LoopDoWhile.countUp` produced 5 mismatches, a broken swap in `quick` produced array mismatches, an inverted null test in `consume` produced mismatches, one changed store in `editDistance` produced 33, and a re-inverted `drain` predicate produced 7. Not covered: loops that render with `goto`. This is a bounded behavioral comparison, not a proof for all inputs.

### Exceptions

| fixture | headline |
| --- | --- |
| `exception-one-catch` | semantics correct; `[exn c0]`, `exception_chain`, `nop` dominate the diff |
| `exception-finally` | `finally` becomes a duplicated normal path plus `catch (all)` rethrow |
| `exception-multi-catch` | `\|` becomes two catches plus `goto`; declarations hoisted |
| `exception-two-resources` | handlers printed outside their protected range; no `try (` form |
| `exception-finally-transfer` | correct, including break/return inside `finally`; no `finally` keyword survives |

### Synchronization

| fixture | headline |
| --- | --- |
| `sync-block` | `synchronized` kept, but every block carries `catch (all)`, `release`, `goto` |

### Classes and inner types

| fixture | headline |
| --- | --- |
| `class-enum-body` | methods correct; enum constants become `static final dex_flags<enum>` plus a static block |
| `class-inner-static` | nested type referenced by name; constructor result goes through a temporary |
| `class-inner-member` | inner class rendered separately; `this.` qualification added |
| `class-interface-default` | `default` keyword lost; `plus()` condition inverted |
| `class-anonymous` | capture via `new Outer$1(this, p0)` is right; result goes through a temporary |
| `class-init-order` | static block, initializer and instance block order correct; field reads go through temporaries |
| `class-bridge` | bridge method rendered and delegating; bounds only survive as `@Signature` fragments |

### Generic, lambda, stream and iteration

| fixture | headline |
| --- | --- |
| `complex-generics` | `repeat` recovers `<T> List<T>`, `largest` only keeps `@Signature`; annotations enter the body |
| `complex-iteration` | enhanced `for` and `Map.Entry` become `Iterator` + `Object` + cast |
| `complex-lambda` | captures correct; lambda body name and `$r8$lambda$…` leak into the class body |
| `complex-stream` | pipelines and method references correct; lambda bodies and `$r8$lambda$…` names enter the class body |

### Algorithms

| fixture | headline |
| --- | --- |
| `algo-graph` | BFS/DFS values correct; `Deque`/`ArrayDeque` generics and element casts lose the declared type |
| `algo-sort` | all three sorts correct; `[repeat: …]` gone, every index still goes through a temporary and `for` is lost |
| `algo-dp` | all three recurrences correct; the 2D array is now declared `int[][]` with one cast on its initializer |
| `algo-string` | KMP and counting correct; `new StringBuilder` split into `new` + `.<init>` |
| `algo-tree` | recursion correct; every field read goes through a temporary, generics dropped |
| `algo-matrix` | all three correct; `new int[m][n]` becomes `Array.newInstance` and the result is returned as `Object` |

### Composite flows

| fixture | headline |
| --- | --- |
| `complex-indy-lambda` | method references stay as `LambdaMetafactory` `call_site` metadata |
| `complex-indy-concat` | one `a + b + c` becomes two lines of `call_site` metadata plus `invoke<Custom>` |
| `complex-inner` | captures and inner construction correct; generic type arguments lost |
| `complex-exceptions` | correct except the TWR-in-loop handler placement; constructors print as `new X;` + `x.<init>(…)` |
| `complex-flow` | behaviour correct; ternary, enum cases and loop exits are all expanded into temporaries, empty `if`s and gotos |
| `complex-switch-nested` | labelled `break`/`continue` collapse into gotos; every operation carries an `[after_operands_before_effect]` snapshot |
| `complex-recursion` | mutual/accumulator recursion and memoisation correct; temporaries everywhere |
| `complex-map-nested` | `computeIfAbsent` lambdas correct; nested generic arguments lost, `via` and dangling casts appear |
| `complex-annotation` | annotations kept; enum values print the value and the type twice |

### Obfuscated

| fixture | headline |
| --- | --- |
| `obf-flow-r8` | R8-renamed enum switch renders as Kotlin `when` and loses the `default:` order |
| `obf-exceptions-proguard` | nested classes flattened, `@MemberClasses` dropped, one short name reused for overloads |
| `obf-dp-r8` | mapped members retain the source fixture's lowered array and loop forms |

### Obfuscated fixtures

A fixture whose directory holds a `rules.pro` is obfuscated before it is dexed:
R8 by default, ProGuard when the first line says `# obfuscator: proguard`. Both
are local tools only - R8 comes from the Android Studio install, ProGuard jars
live under `.cache/tools/proguard/` (GPLv2, never committed) - while the
generated DEX and `mapping.txt` are ours and do stay in the tree.

For obfuscated fixtures, use `mapping.txt` to resolve original owners, names and
full method prototypes. A short mapped name alone is ambiguous; body resemblance
is not member identity. The mapping supports fixture review, not external
runtime declaration input to Diana.

Kotlin reviews also record language-specific findings: Metadata presentation,
copy$default masks, object singletons, delegated properties, continuation state
and exhaustive type selection.

A fixture may pin its own `javac.args` / `d8.args` (`complex-indy-*` need
`--release 11 -XDstringConcat=indy` and `--no-desugaring --min-api 28` to keep
`invoke-custom` in the DEX); the default stays `--release 8 -g` and `--min-api 24`.

The catalog's round-one headlines are historical observations, not current
acceptance results. Use each fixture's current review for its original members,
reviewed paths, concrete findings and remaining uncertainty. Actual execution
counts describe only the stated checks; they are not quality ratings.
