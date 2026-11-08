# Fixture: loop-for-multi

for with several initialisers/updaters and empty segments. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopForMulti;`
- Current numbers from `python3 quality/score.py --slug loop-for-multi`:
  structure 64%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `twoPointers` | ✔ 双指针与交换正确（left++/right-- 都在循环尾） |
| `endless` | ✔ for(;;) + break 折叠成 while (index < limit) |
| `emptyUpdate` | ✔ for(;c;) 的体与更新都在体内，正确 |
| `emptyCondition` | ✔ for(;;step++) 的体被搬进谓词块，取值仍正确 |

### Score

| method | structure |
|---|---|
| `twoPointers` | 63% |
| `endless` | 60% |
| `emptyUpdate` | 77% |
| `emptyCondition` | 57% |

### Findings

- 多初始化/多更新被拆成独立赋值（v2=(v1+-1); v6=0; v5=0;），for 形态全丢
- for(;;) 与空段 for 都被折叠成 while (cond)；emptyCondition 把循环体搬进了 test { … } 谓词块
- 4 个方法全部 while 化，emptyCondition 结构 57%，是最不像源码的一类

### Ranked fixes

还原 for (init; cond; update)：多初始化用逗号、空段保留空位

## Rounds

- R1 2026-11-08: structure 64%; 多初始化/多更新被拆成独立赋值（v2=(v1+-1); v6=0; v5=0;），for 形态全丢
