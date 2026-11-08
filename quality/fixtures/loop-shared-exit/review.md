# Fixture: loop-shared-exit

nested loops leaving through a flag or a labelled break. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopSharedExit;`
- Current numbers from `python3 quality/score.py --slug loop-shared-exit`:
  structure 47%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `flagExit` | ✔ !found 变成循环体开头的 if (v12 != 0) break;，等价 |
| `labelledExit` | ~ 未逐行核对 break outer 的落点 |
| `sharedTail` | ~ 未逐行核对两个循环的 continue |

### Score

| method | structure |
|---|---|
| `flagExit` | 48% |
| `labelledExit` | 53% |
| `sharedTail` | 40% |

### Findings

- for (… && !found; …) 的布尔条件被拆到循环体里（if (v12 != 0) break;），found 标志物化成 int
- 内层命中后 goto L9 跳出内层再设标志，结构分 49%
- break outer 需要看 goto 落点才能确认

### Ranked fixes

共享出口用 break 标志或标签重新结构化；布尔标志恢复 boolean

## Rounds

- R1 2026-11-08: structure 47%; for (… && !found; …) 的布尔条件被拆到循环体里（if (v12 != 0) break;），found 标志物化成 int
