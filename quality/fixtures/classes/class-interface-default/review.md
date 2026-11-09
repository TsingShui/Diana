# Fixture: class-interface-default

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


interface default and static methods. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `doubled` | ✔ The body agrees, but the default keyword is absent. |
| `plus` | ✔ Inverted but equivalent. |
| `constant` | ✔ |

### Findings

- The interface method body is printed as public int doubled() without its source default modifier.
- constant() agrees; plus() uses an inverted condition with equivalent branch selection. Recorded forms: `if (p0 >= 0)`.
- The method extractor previously missed default methods; its correction expanded the recorded method count from 86 to 88.

### Ranked fixes

Improve default-method declarations, condition presentation and parameter names.

## Rounds

- R1 2026-11-08: The interface method body is printed as public int doubled() without its source default modifier.
