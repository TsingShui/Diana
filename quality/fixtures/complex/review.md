# Fixture: complex

The shapes that only show up in real code: enum switch, string switch with side
effects, nested `try` with `finally`, multi-catch, a two-resource try-with-resources, a
synchronized method, `break` and `continue` in one loop, a builder with a conditional
skip, varargs, and a `try` that lives inside a loop.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `javac --release 8 -g` + `d8 --min-api 24`; see the `quality/build.sh`
  output for the versions in use when the DEX was regenerated.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- The file holds the current verdict and a one-line log per round; older rounds live in git.

## Coverage

| method | shape under test |
|---|---|
| `dispatch` | enum switch (d8 lowers it through a `$SwitchMap` holder class) |
| `route` | string switch with a field effect in two branches |
| `nestedTry` | `try` inside `try`, `finally` with a field effect |
| `multiCatch` | multi-catch of two unrelated exception types |
| `twoResources` | try-with-resources over two resources, `close` + `addSuppressed` |
| `guarded` | `synchronized` method plus a field write and an enum `ordinal()` |
| `loopWithBreakContinue` | enhanced `for` with both `continue` and `break` |
| `build` | builder chain, `char` literal argument, conditional `continue` |
| `varargs` | `int...` parameter consumed by an enhanced `for` |
| `tryInsideLoop` | `try`/`catch` inside a counting `for`, accumulating a field-free sum |

Nested helper types (`Res`, `Mode`) are separate classes and are not part of the
reviewed surface: the engine renders one original class per request.

## Current verdict — round 1, 2026-11-08

### Per method

| method | semantics | facts | shape | notation |
|---|---|---|---|---|
| `dispatch` (enum switch) | ✔ | ✔ ordinal switch inlined, `$SwitchMap` resolved away | ~ cases correct, `default:` printed first | ✔ |
| `route` (string switch) | ✔ | ✔ hash/equals kept | ✘ two-level `switch` + `graph<MultipleEntry>` + 4 gotos | ✘ `planned continuation`, labels |
| `nestedTry` | ✔ finally duplicated on both paths | ✔ `log.add("done")` on both | ~ `catch (all)` rethrow instead of `finally` | ✘ `exception_chain`, `[exn c1]`, `nop;` |
| `multiCatch` | ✔ | ✔ | ~ two catches instead of `\|` | ✘ chain, `goto L5`, dead `return v3;` |
| `twoResources` (TWR) | **✘** handlers printed outside any `try`; `H6` reads `v8` before it is assigned in printed order; the outer `try { H10: … }` appears after `throw v8` | ✔ close/addSuppressed kept | ✘ no `try (` | ✘ three `exception_chain`s |
| `guarded` | ✔ | ✔ | ~ `synchronized (this)` added inside an already `synchronized` method (redundant, not wrong) | ✘ `release synchronized`, `goto L3` |
| `loopWithBreakContinue` | ✔ | ✔ | ✘ `continue`/`break` became labels + `goto` (L5/L8/L9) | ✘ 4 gotos |
| `build` | ✔ | **✘** `v5.append<(char)>(59);` — the char literal `';'` is printed as an int with a generic argument | ✘ indexed `while`, no enhanced `for` | ✘ `<(char)>` |
| `varargs` | ✔ | ✔ | ✘ indexed `while`, no enhanced `for` | ✔ |
| `tryInsideLoop` | ✔ | ✔ | ✘✘ `graph<Irreducible>` for a `for` loop containing `try` | ✘ labels, `planned continuation` |

Semantics looked correct in 9/10; one method's printed structure is not
navigable (`twoResources`), and one fact is presented wrongly (`build`).

### Ranked fixes

1. **`twoResources`: handler placement.** Handlers for the outer resource are
   emitted after `return`, at method level, and one of them is wrapped by a `try`
   that itself starts after a `throw`. A reader cannot follow which protected range
   owns `H6`/`H8`/`H10`. The intent (close, then `addSuppressed`) is present.
2. **`build`: char literal.** `append(';')` printed as `append<(char)>(59)`. The
   value is right, the presentation is wrong: a `char` constant should stay a char
   literal, and the generic-argument form should not appear for a non-generic
   method.
3. **`tryInsideLoop`: `graph<Irreducible>`.** A `for` whose body contains `try` is
   a reducible loop; emitting an irreducible graph costs every reader a manual
   reconstruction. Same for `route` (`graph<MultipleEntry>`) and `doWhile`
   (`graph<Unstructured>` in the `shapes` fixture).
4. **`continue`/`break`.** `loopWithBreakContinue` is correct but uses labels and
   gotos where `continue;`/`break;` would print directly.
5. **String and enum switches.** Correct, but two-level + `graph<MultipleEntry>`;
   the reference is one `switch`. Enum is already inlined (`ordinal()`), which is
   good.
6. **`finally` and multi-catch.** Correct desugaring, but `finally` and `|` are the
   reference shapes.
7. **Naming.** `p0`/`v1` everywhere; the reference names (`requested`, `value`,
   `first`, `second`, `part`, `total`) all exist in `debug_info` and are unused.
8. **Notation.** `[exn cN]`, `exception_chain`, `catch (all)`, `release
   synchronized`, `nop;`, `planned continuation`, `<(char)>` in the body.

### Notes

- `dispatch` shows the switch-map class being resolved away, which is better than
  the reference's indirection; keep that behaviour while making `default:` print
  last.
- `guarded` proves the synchronized scope is understood; the extra `synchronized
  (this)` inside a `synchronized` method is the only redundancy.

## Rounds

- R1 2026-11-08: semantics 9/10; `twoResources` handlers outside their `try`; `append<(char)>(59)`; `graph<Irreducible>`/`graph<MultipleEntry>` fallbacks
