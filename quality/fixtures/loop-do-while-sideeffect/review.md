# Fixture: loop-do-while-sideeffect

loop conditions that call something on every test. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopDoWhileSideEffect;`
- Current numbers from `python3 quality/score.py --slug loop-do-while-sideeffect`:
  structure 61.0%, naming 54.4%.

## Current verdict — round 2, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `consume` | ✔ `while (true) { v1 = p0.poll(); if (v1 == null) break; … }`，调用次数与判空都对 |
| `drain` | ✔ 空的 do 体消失，条件后半段仍以体内 break 表达，等价 |
| `untilZero` | ~ 未逐行核对 values[index++] 的自增次数 |
| `retry` | ✔ 短路语义正确（attempts[index++] 只在第一个条件成立时求值） |

### Score

| method | structure |
|---|---|
| `consume` | 64% |
| `drain` | 70% |
| `untilZero` | 58% |
| `retry` | 52% |

### Findings

- 带副作用的循环条件打印在循环体开头 + 倒置条件 `break`：`while (true) { v1 = poll(); if (v1 == null) break; … }`
- do-while 依旧没有 do 形态；drain 把条件后半段变成体内 if … else break
- retry 里出现 int v4 = (v2 + -1); p0[v4] 这种把 index-1 绕一圈的读取

### Ranked fixes

循环条件里的副作用按原位置打印（while ((v = poll()) != null)）；do-while 恢复

## Rounds

- R1 2026-11-08: structure 63%; 带副作用的循环条件被搬进 test { … } 块：while (test { v1 = p0.poll(); yield v1 != null; })
- R2 2026-11-08: structure 61.0%, naming 54.4%; 谓词伪语法 `test { … yield … }` 改为循环体内倒置条件 + `break`
