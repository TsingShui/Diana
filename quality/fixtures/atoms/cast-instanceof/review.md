# Fixture: cast-instanceof

## Current verdict — terminal guard follow-up

Read the changed terminal alternatives and remaining body. The selected
return/throw keeps its original evaluation, while other copies, calls, accesses
and results remain on the opposite path in their original order. Both-edge guard
orientation preserves the predicate's exact meaning; no data, check or effect is
dropped.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


instanceof / check-cast. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 2, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `isString` | ✔ |
| `castToNumber` | ✔ Equivalent with an additional temporary. |
| `castAfterTest` | ✔ Inverted but equivalent. |
| `arrayCast` | ✔ Equivalent with an additional temporary. |

### Findings

- instanceof and the Number cast agree. Recorded forms: `(Number) v1`.
- Checks appear at declarations: `int[] v1 = (int[]) p0; return v1;` and `Number v1 = (Number) p0;`.
- A direct return of a cast becomes a declaration followed by return. Recorded forms: `return (int[]) value`.

### Ranked fixes

Improve direct cast expression presentation and parameter naming.

## Rounds

- R2 2026-11-08: Casts moved into declarations; arrayCast/castToNumber no longer need Object intermediates, but still split declaration and return.
- R1 2026-11-08: instanceof and the Number cast agree. Recorded forms: `(Number) v1`.
