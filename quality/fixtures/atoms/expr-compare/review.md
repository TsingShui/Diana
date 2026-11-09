# Fixture: expr-compare

comparisons and boolean composition. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — selected-route recovery, 2026-11-09

### Semantics

The original LLM reading compared the prepared input, Java explanation and output
on committed baseline `29f1b82` and the selected-route candidate.
Integration rechecks the same paths with dev builds against main `7354242`;
the recovered disjunction and original edge anchors remain, with unchanged
upstream products and identical CLI text/JSON presentations.
Boolean arguments have the Java boolean values false/true (DEX 0/1). No
behavioral executor, device run or Java recompilation ran.

| method | semantics |
|---|---|
| `lessThan` | ~ The inverted `>=` arm produces false; the other produces true. |
| `atLeast` | ~ The original long comparison produces false for `<` and true otherwise; no overflowing subtraction replaces it. |
| `same` | ~ Reference inequality produces false and identity produces true. |
| `different` | ~ Equality produces false and inequality produces true. |
| `both` | ~ Either zero input selects false; both nonzero inputs select true. |
| `either` | ~ The recovered ordered disjunction selects true on the first true input, then on the second true input; both false select false. |
| `negated` | ~ XOR with 1 exchanges the two boolean values. |

`either` was reread after recovery. The first true input skips the second test;
when the first is false the second test selects the same true result through its
original empty goto. The condition now states `p0 != 0 || p1 != 0`, followed by
the original 1/0 result materialization and boolean return. There are no effects
or exceptions on that routing block. The original route transfer and goto
terminal retain one zero-width origin each; no value, edge or copy is fabricated.
Every other method's text is unchanged from the baseline, supporting reuse of
its prior path judgment with the explicit reasoning above.

### Findings

- Comparisons agree, but direct predicate returns become a local plus If. Recorded forms: `return a < b`, `int v; if (a < b) …`.
- Logical negation and conjunction also expand into locals and If. Recorded forms: `!value`, `a && b`.
- `either` now has one readable compound condition without goto or label text.
  Direct boolean-return presentation remains a separate readability limitation.

### Ranked fixes

Improve direct predicate-return presentation.

## Rounds

- R1 2026-11-08: Comparisons agree, but direct predicate returns become a local plus If. Recorded forms: `return a < b`, `int v; if (a < b) …`.
- R2 2026-11-09: P6 owns the selected empty-goto route; P7/P9 recover the disjunction and retain the original routing origins. The new challenge is [short-circuit-shared-routes](../../branches/short-circuit-shared-routes/review.md).
