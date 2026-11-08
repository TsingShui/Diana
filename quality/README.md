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
├── .gitignore                   keeps the committed fixture DEX in the tree
└── fixtures/<slug>/             one self-contained fixture per directory
    ├── <Slug>.java              readable reference and compile input
    ├── classes.dex              generated input, committed for reproducibility
    └── review.md                coverage, preparation, current verdict, round log
```

The fixture DEX is committed **on purpose**: a review then needs only the Diana
CLI, with no JDK, Android SDK or network. That is a deliberate exception to the
"source recipes only" rule, and it covers `quality/fixtures/**/classes.dex` only.

## Workflow

```sh
quality/build.sh                       # only after editing a fixture's Java
python3 quality/dump.py                # writes .cache/review/<slug>.md
python3 quality/dump.py --slug complex # one fixture, to stdout
```

1. `dump.py` prints every fixture method twice: the Java reference and Diana's
   output for the same method, plus the checks to apply.
2. A reviewer (LLM or person) judges each pair on four axes — **semantics**
   (same values, effects, call order, exceptions, locks), **facts** (original
   calls, constants, identities), **shape** (reference constructs recovered) and
   **notation** (only-a-decompiler forms in the body).
3. The judgement is written into the fixture itself, `fixtures/<slug>/review.md`:
   a per-method verdict, a ranked list of what to fix, and a one-line entry per
   round. That file already holds the coverage table and the preparation record,
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
| `shapes` | one method per basic shape: for / enhanced for / do-while / ternary / `\|\|` / packed switch / string switch / try-finally / try-with-resources / synchronized / nested loops / string building |
| `complex` | the shapes real code has: enum switch, string switch with side effects, nested try + finally, multi-catch, two-resource TWR, synchronized method, break+continue in one loop, builder with a conditional skip, varargs, try inside a loop |

## Reviews

| fixture | round 1 headline |
|---|---|
| [fixtures/complex](fixtures/complex/review.md) | semantics correct in 9/10; `twoResources` handlers are printed outside their `try`; `build` prints `append<(char)>(59)`; `tryInsideLoop` and string switch fall back to `graph<…>` |
| [fixtures/shapes](fixtures/shapes/review.md) | all semantics correct; `do-while`, `for`, enhanced `for`, ternary, `\|\|`, `+=`/`++` and `try (` are not recovered; `synchronized` is |

An external comparison for context: over `dex-decompiler`'s fixture corpus the
same kind of reading gave ASC the best shape recovery, `dex-decompiler` the best
names with several wrong methods, and Diana the fewest wrong methods with the
weakest shapes.
