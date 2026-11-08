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
  structure 76%, naming 58%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `next` | 100% | 100% | - |
| `first` | 100% | 100% | - |
| `second` | 100% | 100% | - |
| `emptyWhileSideEffect` | 96% | 100% | `insert: ref=- ours=ID .` |
| `emptyDoSideEffect` | 60% | 100% | `delete: ref=do { } ours=-` |
| `emptyWhile` | 81% | 33% | `delete: ref=int ID = 0 ; ours=-` |
| `plainWhile` | 88% | 33% | `insert: ref=- ours=ID ;` |
| `plainDo` | 55% | 33% | `insert: ref=- ours=; ID` |
| `emptyForBody` | 79% | 33% | `delete: ref=for ( ours=-` |
| `initBeforeDo` | 53% | 25% | `delete: ref== 0 ours=-` |
| `compoundDo` | 61% | 100% | `delete: ref=do { } ours=-` |
| `breakInsideDo` | 62% | 33% | `insert: ref=- ours=; ID` |
| `breakInsideWhile` | 79% | 33% | `insert: ref=- ours=ID ;` |
| `continueInsideDo` | 74% | 25% | `insert: ref=- ours=; int ID ; ID` |
| `infiniteWithBreak` | 73% | 33% | `insert: ref=- ours=; ID` |
| `scanLength` | 74% | 50% | `insert: ref=- ours=; int ID ; ID` |
| `scanDo` | 53% | 50% | `insert: ref=- ours=; ID` |

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
| `breakInsideDo` | ✔ `while (test { v1++; yield v1 <= 10 }) { … }` 取值一致，形状差 |
| `breakInsideWhile` | ✔ `if (v1 != 3) v1++; else break;` |
| `continueInsideDo` | ✔ `continue` 落到条件，等价于 else 分支 |
| `infiniteWithBreak` | ✔ `while (v1 < p0) { v1++; }`，多一条 `nop;` |
| `scanLength` | ✔ `while (test { v1 = p0.length; yield v2 < v1 })`，语义正确 |
| `scanDo` | ✔ 复合条件拆成体尾 break，短路顺序保持 |

### Findings

- 尾测试分类规则在两个方向都成立：`plainDo`/`initBeforeDo`/`continueInsideDo` 恢复 `do-while`，
  空体 `emptyWhile`/`emptyWhileSideEffect` 和 `emptyForBody` 保持 `while`
- `emptyDoSideEffect` 与 `compoundDo` 丢掉空体 `do { }`：条件求值被移进循环体，语义等价但 token 差
- `breakInsideDo`/`scanDo`/`scanLength` 仍是 `while (test { … })`：体跨块或条件需要前导语句
- `infiniteWithBreak` 多一条 `nop;`，`breakInsideDo` 多一个 `L5:` 标签

### Ranked fixes

谓词前导语句内联（把 `test { v1 = p0.length; yield … }` 合成 `while (v2 < p0.length)`），
随后是空体 `do { }` 的保留与 `nop;` 清理；参数名最后。

## Rounds

- R1 2026-11-08: structure 76%, naming 58%; 尾测试双向都正确，剩余 `test { }` 与 `nop;`
