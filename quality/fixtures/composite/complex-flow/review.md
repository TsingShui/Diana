# Fixture: complex-flow

enum state machine with switch, loop, try/finally and ternary. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug complex-flow`:
  structure 23%, naming 69%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `advance` | ✔ 三元与嵌套 if 反演但等价；`case DONE/FAILED` 落 default 正确 |
| `run` | ✔ `continue`/`break` 用 goto 表达；finally 在正常与异常两条路径都写回 steps |
| `summarize` | ✔ `state == DONE ? … : name() + steps` 展开为临时 + if，等价 |

### Score

| method | structure | naming |
|---|---|---|
| `advance` | 18% | 73% |
| `run` | 20% | 50% |
| `summarize` | 31% | 86% |

### Findings

- `switch` 的 `default: break;` 被提到最前，`case 0/1` 顺序保留
- `state = input > 0 ? RUNNING : FAILED` 展开成 if/else 赋值给临时，三元丢失
- 循环里出现空 `if (state != DONE) { }` 与 `goto L6/L8/L9/L10/L12`
- `history[steps] = input` 把 `this.steps` 读了三次（当时无副作用，等价但脆弱）

### Ranked fixes

枚举/三元/空 if 与 goto 清理；`this.steps` 只读一次

## Rounds

- R1 2026-11-08: structure 23%, naming 69%; `switch` 的 `default: break;` 被提到最前，`case 0/1` 顺序保留
