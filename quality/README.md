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
compare original input/source behavior and helper bodies with current decompiled
output, path by path. Structural assertions, similarity scores and test results
are supporting evidence, not substitutes for this semantic reading.

Missing effects or case bodies can be semantic
errors; an explicit handler chain or graph can instead be a readability obstacle
that preserves execution facts. The notation alone does not establish a semantic
error. String metrics measure shape, not correctness, and never replace that
judgement or independently executed behavioral checks.

```text
quality/
├── build.sh                     regenerate DEX from Smali or Java/Kotlin sources
├── dump.py                      reference + Diana output side by side, per method
├── score.py                     structure/naming recovery per method, no thresholds
├── lib.py                       method splitting and token normalisation
├── .gitignore                   keeps the committed fixture DEX in the tree
└── fixtures/<theme>/<slug>/     one structure per directory, one theme per parent
    ├── <Slug>.java              readable explanation; compile input without Smali
    ├── <Slug>.smali             optional assembly input; takes precedence over Java
    ├── classes.dex              prepared input, committed for reproducibility
    ├── README.md                reproduction setup and independent scenarios, if applicable
    └── review.md                written quality verdict, if applicable
```

A fixture tests **one structure** (constants, two-register branches, one catch):
that is what makes the Java source a usable expectation and keeps a review short.
The themes are `atoms/`, `branches/`, `loops/`, `exceptions/`, `sync/`,
`classes/`, `generics/`, `algorithms/`, `composite/`, `obfuscated/` and
`kotlin/`; the two catalog tables below use the same grouping, and a slug stays
unique across themes so `--slug` needs no path.

The fixture DEX is committed **on purpose**: inspection and tests then need no
JDK, Android SDK, assembler or network. The generated-output exception covers
`quality/fixtures/**/classes.dex` only.

## Workflow

```sh
quality/build.sh                       # after editing assembly/compile inputs
python3 quality/score.py               # structure/naming recovery per method
python3 quality/dump.py                # writes .cache/review/<slug>.md
python3 quality/dump.py --slug branch-if2
```

For a fixture with Smali, set `SMALI_CLASSPATH` to a locally available assembler
classpath before running `build.sh`. Its Java file is an explanation, not an
alternative compile input. No tool is downloaded implicitly, and a failed
assembly does not publish a replacement DEX.

A fixture written in Kotlin is **never scored**. Its review is a semantic
judgement written by reading the Kotlin reference next to the output: `?.`, `?:`,
`!!`, `when` and safe-call chains have no lexical counterpart in Java-like output,
so any number would measure syntax rather than recovery. `score.py` therefore
refuses Kotlin fixtures outright instead of reporting a misleading value, and no
hand-written Java "expected" file is added as a workaround.

`score.py` measures **how much of the reference structure survived**: it
normalises both bodies (identifiers to `ID`, string/char literals to `STR`/`CHR`,
comments dropped, keywords/operators/numbers kept) and reports the token-stream
similarity, the share of reference identifiers that reappear, and the first
divergence. Because a fixture holds one structure, that number is meaningful, and
it needs no hand-written expectations. It is a measurement, not a verdict: a 100%
match can still be wrong, and an equivalent shape can score low.

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
   **Every written quality review carries a per-method `### Semantics` table** - `✔` equivalent,
   `~` equivalent but different shape, `✘` wrong, `?` not yet judged - next to the
   numeric score. The numbers never replace the reading: `score.py` measures how
   much structure survived, the semantics table says whether it still means the
   same thing. That file already holds the coverage table and the preparation record,
   while reproduction scenarios and test evidence stay in the fixture's README.
4. Improvements are made, `dump.py` is re-run, and the **current verdict is
   rewritten in place** while `## Rounds` gains one line. The file stays bounded:
   it always opens with what is true now, and older rounds remain readable in
   `git log -p`.

A fixture review records the LLM's semantic judgment and its evidence separately
from readability. It is written quality evidence, not an automated behavioral
execution result or regression acceptance. An LLM judgment MUST NOT override a
failed deterministic check; repair acceptance still follows
[EVOLUTION](../docs/diana/EVOLUTION.md#acceptance-and-regression).

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
| `kotlin-range-loops` | `until`, `downTo`, `step`, `repeat`, `forEach`, labelled `break@` |

Kotlin fixtures are **never scored**; their review is the record (see below).

## Reviews

### Atoms

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `expr-constants` | 96% / 96% | `char` prints as `120`, class literal becomes a temp assignment |
| `invoke-basic` | 92% / 62% | all four call forms exact; generics and constructor temporaries differ |
| `field-basic` | 88% / 83% | `this.`/`X.` qualification and `+=` expansion |
| `cast-instanceof` | 87% / 77% | correct; casts go through an extra `Object v1 = p0;` |
| `expr-arith` | 86% / 44% | correct; each expression becomes `int v = (…); return v;` |
| `array-basic` | 85% / 60% | correct; array creation/literals go through a temporary |
| `expr-compare` | 53% / 38% | predicates expanded to temporaries plus `if` |
| `expr-wide` | 60% / 40% | all correct; `cmpl/cmpg` print as `compare<CmplDouble>(…)` |

### Branches

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `branch-ifz` | 89% / 70% | branches kept; losses are naming and field qualification only |
| `branch-if2` | 71% / 50% | correct but inverted (`<` → `>=`) and split into early returns |
| `branch-if-inverted` | 77% / 75% | inverted empty-arm controls keep origin anchors; float and exception notation remains explicit |
| `branch-switch-string` | 18% / 71% | two-level hash/equals switch plus `graph<MultipleEntry>` |
| `branch-switch-enum` | 42% / 25% | `ordinal()` switch correct, temporaries and `default` order differ |
| `branch-switch-packed` | 55% / 60% | fallthrough and grouped cases correct; case bodies move into `graph<MultipleEntry>` |
| `branch-switch-sparse` | 55% / 58% | correct; d8 lowers it to a packed table and every hole is listed |

### Loops

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `loop-for` | 62% / 38% | no `for`; `break`/`continue` are labels and gotos |
| `loop-for-multi` | 64% / 26% | multi-init/update and empty segments all become independent assignments |
| `loop-map-views` | 64% / 69% | map views and custom `Iterable` become `Iterator` + `Object` + cast |
| `loop-do-while-sideeffect` | 61% / 51% | side-effecting conditions print in the loop body with an inverted `break`; the checked value now prints as a cast on its use, so the whole class is plain Java |
| `loop-shared-exit` | 49% / 31% | private exit work stays in the branch; nested and labelled breaks replace the gotos |
| `loop-do-while` | 54% / 34% | `do-while` recovered; `v1 = (v1 + 1)` against `index++` dominates the diff |
| `loop-tail-shapes` | 82% / 58% | tail tests correct both ways; remaining `nop` and goto-based exits |

### Loop semantics evidence

Executed differential check, 2026-11-08: original fixture methods against Diana's rendered methods over generated inputs (equal return values, exception classes, mutated arrays, matrices, deque contents and static state). 3054 comparisons across `loop-for`, `loop-for-multi`, `loop-do-while`, `loop-do-while-sideeffect`, `loop-shared-exit`, `algo-sort`, `algo-matrix` and `algo-dp`, 0 mismatches; `knapsack` contributes 1568, `quick` 896, `multiply` 64 and `editDistance` 64. All eight classes render as plain Java. The checker is sensitive: a changed loop bound in `LoopDoWhile.countUp` produced 5 mismatches, a broken swap in `quick` produced array mismatches, an inverted null test in `consume` produced mismatches, one changed store in `editDistance` produced 33, and a re-inverted `drain` predicate produced 7. Not covered: loops that render with `goto`. This is a bounded behavioral comparison, not a proof for all inputs.

### Exceptions

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `exception-one-catch` | 48% / 57% | semantics correct; `[exn c0]`, `exception_chain`, `nop` dominate the diff |
| `exception-finally` | 78% / 50% | `finally` becomes a duplicated normal path plus `catch (all)` rethrow |
| `exception-multi-catch` | 43% / 80% | `\|` becomes two catches plus `goto`; declarations hoisted |
| `exception-two-resources` | **7.6%** / 33% | handlers printed outside their protected range; no `try (` form |
| `exception-finally-transfer` | 49% / 61% | correct, including break/return inside `finally`; no `finally` keyword survives |

### Synchronization

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `sync-block` | 42% / 83% | `synchronized` kept, but every block carries `catch (all)`, `release`, `goto` |

### Classes and inner types

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `class-enum-body` | 90% / 83% | methods correct; enum constants become `static final dex_flags<enum>` plus a static block |
| `class-inner-static` | 89% / 54% | nested type referenced by name; constructor result goes through a temporary |
| `class-inner-member` | 84% / 67% | inner class rendered separately; `this.` qualification added |
| `class-interface-default` | 78% / 89% | `default` keyword lost; `plus()` condition inverted |
| `class-anonymous` | 73% / 67% | capture via `new Outer$1(this, p0)` is right; result goes through a temporary |
| `class-init-order` | 68% / 100% | static block, initializer and instance block order correct; field reads go through temporaries |
| `class-bridge` | 77% / 60% | bridge method rendered and delegating; bounds only survive as `@Signature` fragments |

### Generic, lambda, stream and iteration

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `complex-generics` | 72% / 58% | `repeat` recovers `<T> List<T>`, `largest` only keeps `@Signature`; annotations enter the body |
| `complex-iteration` | 66% / 59% | enhanced `for` and `Map.Entry` become `Iterator` + `Object` + cast |
| `complex-lambda` | 64% / 57% | captures correct; lambda body name and `$r8$lambda$…` leak into the class body |
| `complex-stream` | 62% / 73% | pipelines and method references correct; lambda bodies and `$r8$lambda$…` names enter the class body |

### Algorithms

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `algo-graph` | 50% / 45% | BFS/DFS values correct; `Deque`/`ArrayDeque` generics and element casts lose the declared type |
| `algo-sort` | 29% / 31% | all three sorts correct; `[repeat: …]` gone, every index still goes through a temporary and `for` is lost |
| `algo-dp` | 14% / 37% | all three recurrences correct; the 2D array is now declared `int[][]` with one cast on its initializer |
| `algo-string` | 38% / 62% | KMP and counting correct; `new StringBuilder` split into `new` + `.<init>` |
| `algo-tree` | 34% / 60% | recursion correct; every field read goes through a temporary, generics dropped |
| `algo-matrix` | 28% / 29% | all three correct; `new int[m][n]` becomes `Array.newInstance` and the result is returned as `Object` |

### Composite flows

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `complex-indy-lambda` | 32% / 68% | method references stay as `LambdaMetafactory` `call_site` metadata |
| `complex-indy-concat` | 28% / 60% | one `a + b + c` becomes two lines of `call_site` metadata plus `invoke<Custom>` |
| `complex-inner` | 74% / 36% | captures and inner construction correct; generic type arguments lost |
| `complex-exceptions` | 30% / 63% | correct except the TWR-in-loop handler placement; constructors print as `new X;` + `x.<init>(…)` |
| `complex-flow` | 23% / 70% | behaviour correct; ternary, enum cases and loop exits are all expanded into temporaries, empty `if`s and gotos |
| `complex-switch-nested` | 17% / 38% | labelled `break`/`continue` collapse into gotos; every operation carries an `[after_operands_before_effect]` snapshot |
| `complex-recursion` | 66% / 55% | mutual/accumulator recursion and memoisation correct; temporaries everywhere |
| `complex-map-nested` | 54% / 69% | `computeIfAbsent` lambdas correct; nested generic arguments lost, `via` and dangling casts appear |
| `complex-annotation` | 92% / 78% | annotations kept; enum values print the value and the type twice |

### Obfuscated

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `obf-flow-r8` | 43% / obf | R8-renamed enum switch renders as Kotlin `when` and loses the `default:` order |
| `obf-exceptions-proguard` | 31% / obf | nested classes flattened, `@MemberClasses` dropped, one short name reused for overloads |
| `obf-dp-r8` | 30% / obf | scores match the unobfuscated source fixture: renaming alone costs no structure |

### Obfuscated fixtures

A fixture whose directory holds a `rules.pro` is obfuscated before it is dexed:
R8 by default, ProGuard when the first line says `# obfuscator: proguard`. Both
are local tools only - R8 comes from the Android Studio install, ProGuard jars
live under `.cache/tools/proguard/` (GPLv2, never committed) - while the
generated DEX and `mapping.txt` are ours and do stay in the tree.

`score.py` reads `mapping.txt` and pairs each reference method with the produced
body through the mapped name **and parameter types**, because an obfuscator may
give one short name to several overloads. Structure is comparable with the
unobfuscated source fixture; naming is reported as `obf` because the obfuscator
destroyed it.

Kotlin fixtures (6) are reviewed by reading only: `score.py` refuses them and the
review carries a per-method semantics verdict plus the Kotlin-specific findings
(`@Metadata` into the body, `copy$default` masks, `INSTANCE` singletons,
`$delegate` fields, continuation state machines, `when` lowered to `instanceof`).

A fixture may pin its own `javac.args` / `d8.args` (`complex-indy-*` need
`--release 11 -XDstringConcat=indy` and `--no-desugaring --min-api 28` to keep
`invoke-custom` in the DEX); the default stays `--release 8 -g` and `--min-api 24`.

Corpus totals: **structure 63%, naming 60%** over 241 methods (the
obfuscated fixtures contribute structure only; Kotlin fixtures are unreviewed by
the score). The
`complex-*` and `algo-*` fixtures are the lowest scorers: real code multiplies the
temporary variables, `while (true)` loops with an inverted exit and goto-based exits that the
single-structure fixtures isolate. The second
batch (loops, switches, exceptions, synchronization) is exactly where the recovery
is weakest: string switch 18%, two-resource TWR 8%, synchronized 42%.

An external comparison for context: over `dex-decompiler`'s fixture corpus the
same kind of reading gave ASC the best shape recovery, `dex-decompiler` the best
names with several wrong methods, and Diana the fewest wrong methods with the
weakest shapes.
