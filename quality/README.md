# Quality fixtures and review

Diana's directory for **decompilation output quality**: hand-written Java
fixtures, the DEX generated from them, and written reviews of what the CLI
actually produced. It exists to answer one question — *did the readable output
get better* — separately from semantic checks and from the source-only fixture
workflow in `docs/diana/FIXTURES.md`.

No script scores the output. Quality is judged by reading, because the failures
that matter here (a lost `case` body, a handler printed outside its `try`, a
`for` turned into a graph) are semantic, and string metrics were measured to be
both false-positive and false-negative prone.

```text
quality/
├── build.sh                     regenerate every fixture's classes.dex
├── dump.py                      reference + Diana output side by side, per method
├── score.py                     structure/naming recovery per method, no thresholds
├── lib.py                       method splitting and token normalisation
├── .gitignore                   keeps the committed fixture DEX in the tree
└── fixtures/<slug>-<structure>/ one structure per directory
    ├── <Slug>.java              readable reference, compile input, the expectation
    ├── classes.dex              generated input, committed for reproducibility
    └── review.md                current verdict, ranked fixes, round log
```

A fixture tests **one structure** (constants, two-register branches, one catch):
that is what makes the Java source a usable expectation and keeps a review short.
The corpus is grouped by category prefix: `expr-`, `field-`, `invoke-`, `array-`,
`branch-`, then later `loop-`, `exception-`, `sync-`, `class-`, `kotlin-`,
`malformed-`.

The fixture DEX is committed **on purpose**: a review then needs only the Diana
CLI, with no JDK, Android SDK or network. That is a deliberate exception to the
"source recipes only" rule, and it covers `quality/fixtures/**/classes.dex` only.

## Workflow

```sh
quality/build.sh                       # only after editing a fixture's Java
python3 quality/score.py               # structure/naming recovery per method
python3 quality/dump.py                # writes .cache/review/<slug>.md
python3 quality/dump.py --slug branch-if2
```

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
2. A reviewer (LLM or person) judges each pair on four axes — **semantics**
   (same values, effects, call order, exceptions, locks), **facts** (original
   calls, constants, identities), **shape** (reference constructs recovered) and
   **notation** (only-a-decompiler forms in the body).
3. The judgement is written into the fixture itself, `fixtures/<slug>/review.md`.
   **Every fixture carries a per-method `### Semantics` table** - `✔` equivalent,
   `~` equivalent but different shape, `✘` wrong, `?` not yet judged - next to the
   numeric score. The numbers never replace the reading: `score.py` measures how
   much structure survived, the semantics table says whether it still means the
   same thing. That file already holds the coverage table and the preparation record,
   so a fixture is three files and no side directories.
4. Improvements are made, `dump.py` is re-run, and the **current verdict is
   rewritten in place** while `## Rounds` gains one line. The file stays bounded:
   it always opens with what is true now, and older rounds remain readable in
   `git log -p`.

Commands are only re-verified with the ordinary equivalence checks (`get-class`,
`get-method`, `xref`, `outline` byte-identical before/after); the review says
*whether it reads better*, not *whether it is correct*.

## Fixtures

| slug | covers |
|---|---|
| `expr-constants` | every constant form: int, long, float, double, char, boolean, String, null, class literal |
| `expr-arith` | add, subtract, multiply, modulo, bits, shifts, negation |
| `expr-compare` | `<`, `>=`, `==`, `!=`, `&&`, `\|\|`, `!` |
| `field-basic` | static/instance reads and writes, compound assignment |
| `invoke-basic` | static, virtual, interface, constructor, `invoke-range` args, generic call |
| `array-basic` | sized and literal arrays, byte/boolean arrays, read, write, `.length`, 2D |
| `branch-if2` | two-register branches with effects on both sides |
| `branch-ifz` | single-register branches, `null` check |
| `cast-instanceof` | `instanceof`, `check-cast`, array cast |
| `exception-one-catch` | one catch that returns, one that rethrows, effect order inside `try` |
| `exception-multi-catch` | one multi-catch clause, one caught rethrow |
| `exception-finally` | `finally` on the normal path and on the throwing path |
| `exception-two-resources` | try-with-resources over two resources, `close` + `addSuppressed` |
| `loop-for` | classic `for` with `break` and `continue` in the body |
| `loop-do-while` | `do-while`, `do-while` with `break`, nested `do-while` |
| `branch-switch-string` | switch over `String`, grouped `case` labels, effects in branches |
| `branch-switch-enum` | switch over an enum with a `default` arm |
| `sync-block` | `synchronized` block, early return inside it, nested blocks |
| `class-inner-member` | member inner class, synthetic accessor to a private field |
| `class-inner-static` | static nested class with its own state |
| `class-anonymous` | anonymous class capturing a local and a field |
| `class-enum-body` | enum with constructor state, method and a constant body |
| `class-interface-default` | interface `default` and `static` methods |
| `class-init-order` | field initializers, instance block, static init, `this(...)` |
| `complex-flow` | enum state machine: switch, loop with break/continue, try/finally, ternary |
| `complex-exceptions` | nested `try`/`finally`, caught rethrow with cause, TWR in a loop, multi-catch |
| `complex-inner` | anonymous class, member inner, static nested, generic bag |
| `algo-sort` | insertion, quick and merge sort |
| `algo-graph` | BFS, DFS and connected components on an adjacency matrix |
| `algo-dp` | edit distance, longest increasing subsequence, knapsack |
| `algo-tree` | binary search tree: recursive insert/height/inOrder, iterative contains |
| `algo-string` | KMP prefix/search, word frequency over `HashMap`, reversal |
| `complex-generics` | generic class, bounded generic method, bridge method, generic call |
| `complex-lambda` | lambdas, capturing lambda, anonymous supplier, method reference, comparator |
| `complex-iteration` | iterator loops, enhanced `for`, `Map.Entry`, `iterator.remove()` |
| `complex-switch-nested` | `switch` inside a loop inside `try`, labelled `break`/`continue` |
| `complex-stream` | stream pipelines, method references, `Optional`, `IntStream` |
| `complex-map-nested` | `Map<String, List<Integer>>`, `computeIfAbsent`, `getOrDefault`, entry loops |
| `complex-recursion` | mutual recursion, accumulator recursion, memoised Fibonacci |
| `algo-matrix` | matrix multiply, transpose, spiral traversal |
| `branch-switch-packed` | packed switch: fallthrough, grouped cases, negative and default arms |
| `branch-switch-sparse` | sparse switch: non-contiguous and large case values |
| `expr-wide` | `long` shifts/arithmetic, `float`/`double`, NaN comparison, narrowing casts |
| `exception-finally-transfer` | `finally` with `return`/`break`/`continue`, try inside catch |
| `class-bridge` | bounded generics, a real bridge method, wildcard signatures |
| `complex-annotation` | runtime annotations with array, enum, class and default values |
| `obf-flow-r8` | R8-obfuscated `complex-flow` (enum switch, loop, try/finally, ternary) |
| `obf-dp-r8` | R8-obfuscated `algo-dp` (2D arrays, nested loops, `Math.min`) |
| `obf-exceptions-proguard` | ProGuard-obfuscated `complex-exceptions` (nested try/finally, TWR in a loop, multi-catch) |
| `kotlin-null-safety` | `?.`, `?:`, `!!`, safe-call chain, `@Metadata` |
| `kotlin-data-class` | data class, `componentN`, `copy`, `copy$default`, `equals`/`hashCode`/`toString` |
| `kotlin-sealed-when` | sealed hierarchy, exhaustive `when`, smart casts |
| `kotlin-object-singleton` | `object` with `INSTANCE`, companion object, synthetic accessors |
| `kotlin-property-delegate` | `by lazy`, custom `ReadWriteProperty`, custom setter, `lateinit` |
| `kotlin-suspend-state` | suspend functions, continuation state machine, suspend inside try/loop |

Kotlin fixtures are **never scored**; their review is the record (see below).

## Reviews

| fixture | structure / naming (round 1) | headline |
|---|---|---|
| `expr-constants` | 96% / 96% | `char` prints as `120`, class literal becomes a temp assignment |
| `invoke-basic` | 92% / 62% | all four call forms exact; generics and constructor temporaries differ |
| `branch-ifz` | 89% / 70% | branches kept; losses are naming and field qualification only |
| `field-basic` | 88% / 83% | `this.`/`X.` qualification and `+=` expansion |
| `cast-instanceof` | 87% / 77% | correct; casts go through an extra `Object v1 = p0;` |
| `expr-arith` | 86% / 44% | correct; each expression becomes `int v = (…); return v;` |
| `array-basic` | 85% / 60% | correct; array creation/literals go through a temporary |
| `branch-if2` | 71% / 50% | correct but inverted (`<` → `>=`) and split into early returns |
| `expr-compare` | 53% / 38% | predicates expanded to temporaries plus `if` |
| `exception-one-catch` | 48% / 57% | semantics correct; `[exn c0]`, `exception_chain`, `nop` dominate the diff |
| `exception-finally` | 78% / 50% | `finally` becomes a duplicated normal path plus `catch (all)` rethrow |
| `exception-multi-catch` | 43% / 80% | `\|` becomes two catches plus `goto`; declarations hoisted |
| `exception-two-resources` | **7.6%** / 33% | handlers printed outside their protected range; no `try (` form |
| `loop-for` | 62% / 38% | no `for`; `break`/`continue` are labels and gotos |
| `loop-do-while` | 54% / 34% | `do` lost entirely; body wrapped in `while (test { graph<Unstructured> … })` |
| `branch-switch-string` | 18% / 71% | two-level hash/equals switch plus `graph<MultipleEntry>` |
| `branch-switch-enum` | 42% / 25% | `ordinal()` switch correct, temporaries and `default` order differ |
| `sync-block` | 42% / 83% | `synchronized` kept, but every block carries `catch (all)`, `release`, `goto` |
| `class-enum-body` | 90% / 83% | methods correct; enum constants become `static final dex_flags<enum>` plus a static block |
| `class-inner-static` | 89% / 54% | nested type referenced by name; constructor result goes through a temporary |
| `class-inner-member` | 84% / 67% | inner class rendered separately; `this.` qualification added |
| `class-interface-default` | 78% / 89% | `default` keyword lost; `plus()` condition inverted |
| `class-anonymous` | 73% / 67% | capture via `new Outer$1(this, p0)` is right; result goes through a temporary |
| `class-init-order` | 68% / 100% | static block, initializer and instance block order correct; field reads go through temporaries |
| `complex-inner` | 74% / 36% | captures and inner construction correct; generic type arguments lost |
| `algo-graph` | 50% / 45% | BFS/DFS values correct; `Deque`/`ArrayDeque` generics and element casts lose the declared type |
| `algo-sort` | 50% / 49% | all three sorts correct; every index goes through a temporary and `for` is lost |
| `complex-exceptions` | 30% / 63% | correct except the TWR-in-loop handler placement; constructors print as `new X;` + `x.<init>(…)` |
| `algo-dp` | 29% / 37% | all three recurrences correct; 2D arrays go through `Array.newInstance` with uncast reads |
| `complex-flow` | 23% / 70% | behaviour correct; ternary, enum cases and loop exits are all expanded into temporaries, empty `if`s and gotos |
| `complex-generics` | 72% / 58% | `repeat` recovers `<T> List<T>`, `largest` only keeps `@Signature`; annotations enter the body |
| `complex-iteration` | 66% / 59% | enhanced `for` and `Map.Entry` become `Iterator` + `Object` + cast |
| `complex-lambda` | 64% / 57% | captures correct; lambda body name and `$r8$lambda$…` leak into the class body |
| `algo-string` | 38% / 62% | KMP and counting correct; `new StringBuilder` split into `new` + `.<init>` |
| `algo-tree` | 34% / 60% | recursion correct; every field read goes through a temporary, generics dropped |
| `complex-switch-nested` | 17% / 38% | labelled `break`/`continue` collapse into gotos; every operation carries an `[after_operands_before_effect]` snapshot |
| `complex-recursion` | 66% / 55% | mutual/accumulator recursion and memoisation correct; temporaries everywhere |
| `complex-stream` | 62% / 73% | pipelines and method references correct; lambda bodies and `$r8$lambda$…` names enter the class body |
| `complex-map-nested` | 54% / 69% | `computeIfAbsent` lambdas correct; nested generic arguments lost, `via` and dangling casts appear |
| `algo-matrix` | 28% / 29% | all three correct; `new int[m][n]` becomes `Array.newInstance` and the result is returned as `Object` |
| `branch-switch-packed` | 55% / 60% | fallthrough and grouped cases correct; case bodies move into `graph<MultipleEntry>` |
| `branch-switch-sparse` | 55% / 58% | correct; d8 lowers it to a packed table and every hole is listed |
| `expr-wide` | 60% / 40% | all correct; `cmpl/cmpg` print as `compare<CmplDouble>(…)` |
| `exception-finally-transfer` | 49% / 61% | correct, including break/return inside `finally`; no `finally` keyword survives |
| `class-bridge` | 77% / 60% | bridge method rendered and delegating; bounds only survive as `@Signature` fragments |
| `complex-annotation` | 92% / 78% | annotations kept; enum values print the value and the type twice |
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

Corpus totals (round 1): **structure 62%, naming 60%** over 183 methods (the
obfuscated fixtures contribute structure only; Kotlin fixtures are unreviewed by
the score). The
`complex-*` and `algo-*` fixtures are the lowest scorers: real code multiplies the
temporary variables, `while (test { … })` loops and goto-based exits that the
single-structure fixtures isolate. The second
batch (loops, switches, exceptions, synchronization) is exactly where the recovery
is weakest: string switch 18%, two-resource TWR 8%, synchronized 42%.

An external comparison for context: over `dex-decompiler`'s fixture corpus the
same kind of reading gave ASC the best shape recovery, `dex-decompiler` the best
names with several wrong methods, and Diana the fewest wrong methods with the
weakest shapes.
