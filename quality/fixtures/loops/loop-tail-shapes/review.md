# Fixture: loop-tail-shapes

Every loop shape whose test can sit after the body, next to the head-test shapes
that must stay `while`. A tail test compiles to one block whose branch returns to
its own start; the empty-body variants are the ones that most easily collapse into
each other, so they are the regression for the P6 tail-test classification.

The Java source is the reference and the expectation; the DEX is generated from it
with `javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and
committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug loop-tail-shapes`:
  structure 81.3%, naming 57.8%.

## Current verdict — round 2, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `next` | 100.0% | 100% | - |
| `first` | 100.0% | 100% | - |
| `second` | 100.0% | 100% | - |
| `emptyWhileSideEffect` | 95.7% | 100% | insert: ref=- ours=ID . |
| `emptyDoSideEffect` | 77.2% | 100% | insert: ref=- ours=ID ID ; |
| `emptyWhile` | 80.9% | 33% | delete: ref=int ID = 0 ; ours=- |
| `plainWhile` | 87.9% | 33% | insert: ref=- ours=ID ; |
| `plainDo` | 88.6% | 33% | insert: ref=- ours=; ID |
| `emptyForBody` | 79.4% | 33% | delete: ref=for ( ours=- |
| `initBeforeDo` | 82.1% | 25% | insert: ref=- ours=; int ID ; ID |
| `compoundDo` | 61.3% | 100% | delete: ref=do { } ours=- |
| `breakInsideDo` | 74.5% | 33% | insert: ref=- ours=; ID |
| `breakInsideWhile` | 79.1% | 33% | insert: ref=- ours=ID ; |
| `continueInsideDo` | 76.2% | 25% | insert: ref=- ours=; int ID ; ID |
| `infiniteWithBreak` | 72.7% | 33% | insert: ref=- ours=; ID |
| `scanLength` | 71.1% | 50% | insert: ref=- ours=; int ID ; ID |
| `scanDo` | 54.7% | 50% | insert: ref=- ours=; ID |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `emptyWhileSideEffect` | ✔ `while (next() != null) { }`，每次进入都先测 |
| `emptyDoSideEffect` | ✔ `do { v0 = next(); } while (v0 != null)`，每轮一次调用 |
| `emptyWhile` | ✔ 条件折叠成 `while (0 < p0) { }`，与原无限循环等价 |
| `plainWhile` | ✔ `while (v1 < p0) { v1++; }` |
| `plainDo` | ✔ `do { v1++; } while (v1 < p0);` |
| `emptyForBody` | ✔ `while (v1 < p0) { v1++; }`，`for` 未恢复但取值一致 |
| `initBeforeDo` | ✔ 初始化留在循环外，`do-while` 正确 |
| `compoundDo` | ✔ `while (first()) { if (second()) {} else break; }` 与 `first() && second()` 短路一致 |
| `breakInsideDo` | ✔ `while (true) { v1++; if (v1 > 10) break; … }`，取值一致 |
| `breakInsideWhile` | ✔ `if (v1 != 3) v1++; else break;` |
| `continueInsideDo` | ✔ `continue` 落到条件，等价于 else 分支 |
| `infiniteWithBreak` | ✔ `while (v1 < p0) { v1++; }`，多一条 `nop;` |
| `scanLength` | ✔ `while (true) { v1 = p0.length; if (v2 >= v1) break; v2++; }`，语义正确 |
| `scanDo` | ✔ 复合条件拆成 `if (读数组) {} else break;`，短路顺序保持 |

### Findings

- 尾测试分类规则在两个方向都成立：`plainDo`/`initBeforeDo`/`continueInsideDo` 恢复 `do-while`，
  空体 `emptyWhile`/`emptyWhileSideEffect` 和 `emptyForBody` 保持 `while`
- `emptyDoSideEffect` 与 `compoundDo` 丢掉空体 `do { }`：条件求值被移进循环体，语义等价但 token 差
- 谓词带前导语句的循环现在打印成 `while (true) { …; if (exit) break; }`：Java 合法，但 `while (cond)` 的形状没回来
- `infiniteWithBreak` 多一条 `nop;`，`breakInsideDo` 多一个 `L5:` 标签

### Ranked fixes

谓词前导语句内联成赋值表达式（`while ((v1 = p0.length) > v2)`），把 `while (true)` + `break`
换回 `while (cond)`；随后是空体 `do { }` 的保留与 `nop;` 清理；参数名最后。

## Rounds
- R3 2026-11-08: 空分支清理；`else { }` 与空 `if` 不再打印，空真分支反演成单臂 if（谓词求值位置不变）

- R1 2026-11-08: structure 82%, naming 58%%; 尾测试双向都正确，剩余 `test { }` 与 `nop;`
- R2 2026-11-08: structure 81.3%, naming 57.8%; 谓词伪语法 `while (test { … yield … })` 改为 `while (true) { …; if (exit) break; }`
