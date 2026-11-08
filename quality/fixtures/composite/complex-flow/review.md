# Fixture: complex-flow

enum state machine with switch, loop, try/finally and ternary. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `advance` | ✔ Inverted ternary/nested If selections agree; DONE/FAILED correctly take default. |
| `run` | ✔ Continue/break use gotos; finally writes steps on normal and exceptional paths. |
| `summarize` | ✔ The conditional result expands into locals and If with equivalent selection. |

### Findings

- default: break is printed first; case 0/1 order remains. Recorded forms: `switch`, `default: break;`.
- The RUNNING/FAILED ternary expands into If/else assignments to a local. Recorded forms: `state = input > 0 ? RUNNING : FAILED`.
- The loop contains an empty If and goto L6/L8/L9/L10/L12. Recorded forms: `if (state != DONE) { }`.
- history[steps] reads this.steps three times; the recorded path had no intervening side effect, but the presentation is fragile. Recorded forms: `history[steps] = input`.

### Ranked fixes

Improve enum/ternary presentation, empty branches and gotos; avoid redundant field-read presentation. Recorded forms: `this.steps`.

## Rounds

- R1 2026-11-08: default: break is printed first; case 0/1 order remains. Recorded forms: `switch`, `default: break;`.
