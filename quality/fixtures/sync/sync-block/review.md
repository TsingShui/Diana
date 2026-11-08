# Fixture: sync-block

synchronized block, including an early return inside it. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `add` | ✔ The monitor is released before early return. |
| `clearIfPositive` | ✔ |
| `nested` | ✔ Nested acquisition and release order agree. |

### Findings

- Synchronized scopes and their nesting remain visible. Recorded forms: `synchronized`.
- Each scope includes explicit catch-all release/rethrow handling and a goto L3. Recorded forms: `try { … } catch (all) { release synchronized …; throw; }`.
- Early-return paths release before returning.

### Ranked fixes

Improve synchronized and explicit-release/transfer presentation and parameter names. Recorded forms: `synchronized (lock) { … }`, `release synchronized`, `goto`.

## Rounds

- R1 2026-11-08: Synchronized scopes and their nesting remain visible. Recorded forms: `synchronized`.
