# Fixture: shapes

One class, one method per decompilation shape that Diana currently loses. The Java
source is the readable reference; `classes.dex` is generated from it and committed so a
review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with: `javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`
  output for the versions in use when the DEX was regenerated).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- The file holds the current verdict and a one-line log per round; older rounds live in git.

## Coverage

| method | shape under test |
|---|---|
| `paramNaming` | parameter and local naming, classic `for`, `+=` |
| `loopFor` | classic `for` |
| `enhancedFor` | enhanced `for`, `+=` |
| `doWhile` | `do/while` |
| `ternaryMax` | ternary |
| `shortCircuitOr` | `\|\|` |
| `packedSwitch` | packed switch with four cases |
| `stringSwitch` | string switch (hash + equals) |
| `tryFinally` | try/finally, `+=` |
| `tryWithResources` | try-with-resources |
| `syncBlock` | `synchronized` block, `+=` |
| `arraySum2d` | nested enhanced `for`, `+=` |
| `stringConcat` | string building |

Each method is expected to recover its shape **and** keep the Java identifiers visible
in the readable output. Nested helper types (`Resource`) are separate classes and are not
part of the reviewed surface: the engine renders one original class per request.

## Current verdict — round 1, 2026-11-08

### Per method

| method | semantics | shape |
|---|---|---|
| `paramNaming` | ✔ | ✘ no `for`, no `+=`, `p0/v1` instead of `count/label/index/total` |
| `loopFor` | ✔ | ✘ `while` + `test { yield }` block instead of `for` |
| `enhancedFor` | ✔ | ✘ indexed `while`, no enhanced `for`, no `+=` |
| `doWhile` | ✔ | ✘ `while (test { graph<Unstructured> … })` — worst case in the corpus |
| `ternaryMax` | ✔ | ✘ `if/else` into a temporary instead of `? :` |
| `shortCircuitOr` | ✔ | ✘ two negated `if`s instead of `\|\|` |
| `packedSwitch` | ✔ | ~ all cases correct, `default:` printed first |
| `stringSwitch` | ✔ | ✘ two-level `switch` + hash/equals + `graph<MultipleEntry>` |
| `tryFinally` | ✔ | ~ `catch (all)` rethrow instead of `finally` |
| `tryWithResources` | ✔ | ✘ desugared close/addSuppressed, no `try (` |
| `syncBlock` | ✔ | ✔ `synchronized (LOCK)` kept |
| `arraySum2d` | ✔ | ✘ nested indexed `while`, no enhanced `for` |
| `stringConcat` | ✔ | ~ `StringBuilder` chain instead of `+` (defensible) |

### Ranked fixes

1. `doWhile` and any loop whose predicate carries statements: remove the
   `graph<Unstructured>` form. This is the largest readability loss in the corpus.
2. Enhanced `for` over arrays (`enhancedFor`, `arraySum2d`) and classic `for`.
3. `+=` / `++` in the readable output.
4. Condition shape: do not invert, and merge adjacent `if`s into `&&`/`||`.
5. Ternary when both branches are single expressions.
6. `try (` for a single resource whose `close` is the only cleanup.
7. `default:` last inside `switch`.

## Rounds

- R1 2026-11-08: semantics 13/13; `do-while`/`for`/enhanced `for`/ternary/`||`/`+=`/`++`/`try (` not recovered; `synchronized` is
