# Fixture: sync-block

synchronized block, including an early return inside it. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — monitor continuation review

Java and final add(int), clearIfPositive() and nested(int) were reread.
Release no longer prints a redundant ordinary goto before its already supplied
control completion. add still captures the post-write counter while held, releases,
then returns that value. clearIfPositive still writes zero only when positive;
its final unadorned void return is display-omitted. nested still acquires the same
LOCK twice, writes while both obligations are held, releases the inner obligation,
reads counter while the outer remains held, then releases and returns. Exceptional
paths retain their matching inner/outer releases and same pending throwable.
Static initialization and the constructor are unchanged.

This follow-up is bounded direct LLM path reading, not executed behavioral PASS.
Corpus inspection preserves exact P2/P3/P4/P5 and original P6 graph/lock facts,
including synchronized release targets, routing and before/after plans. Unchanged
text reuses prior reviews. See the new
[monitor-exit challenge](../../sync/monitor-iteration-exits/review.md) for scope
and the initial candidate regression; no device run or Java recompilation ran.

## Prior verdict — round 1, 2026-11-08

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
