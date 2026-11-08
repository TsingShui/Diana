# Fixture: algo-tree

binary search tree: recursive insert/height/inOrder and iterative contains. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug algo-tree`:
  structure 33%, naming 60%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `insert` | ✔ 递归插入与 `root` 回写正确 |
| `contains` | ✔ 反演但等价（`!=` 后按 `>=` 选边） |
| `height` | ✔ 左右递归取大 + 1 |
| `inOrder` | ✔ 传出的列表按左-根-右填充 |

### Score

| method | structure | naming |
|---|---|---|
| `insert` | 23% | 67% |
| `contains` | 46% | 75% |
| `height` | 22% | 50% |
| `inOrder` | 43% | 50% |

### Findings

- 方法按名字排序输出，重载的 `insert`/`height`/`inOrder` 与入口方法混在一起，需靠签名区分
- `List<Integer> out = new ArrayList<>()` 印成 `ArrayList arrayList = new ArrayList()`，泛型实参丢失
- `Integer.valueOf(p0.key)` 出现在 `add` 前，值正确但多一层
- 结构分 22%~46%，主要来自每个字段读取先存临时（`AlgoTree$Node v2 = p0.left; this.height(v2)`）

### Ranked fixes

字段读取直接内联到调用；保留泛型实参；重载按完整原型排序

## Rounds

- R1 2026-11-08: structure 33%, naming 60%; 方法按名字排序输出，重载的 `insert`/`height`/`inOrder` 与入口方法混在一起，需靠签名区分
