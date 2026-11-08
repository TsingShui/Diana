# Fixture: sync-block

synchronized block, including an early return inside it. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug sync-block`:
  structure 42%, naming 83%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `add` | 41% | 75% | `insert: ref=- ours=int ID ; ID ID` |
| `clearIfPositive` | 49% | 100% | `insert: ref=- ours=ID ID ; ID ID` |
| `nested` | 35% | 75% | `insert: ref=- ours=int ID ; ID ID` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `add` | ✔ 早返回前已释放 |
| `clearIfPositive` | ✔ |
| `nested` | ✔ 嵌套锁与释放正确 |

### Findings

- `synchronized` 形状保留，嵌套块也正确嵌套（结构 35~49%）
- 每个块都附 `try { … } catch (all) { release synchronized …; throw; }` 与 `goto L3`
- 语义正确：早返回路径先释放再返回

### Ranked fixes

压缩成 `synchronized (lock) { … }` 单一容器；`release synchronized`/`goto` 移出正文；参数名恢复

## Rounds

- R1 2026-11-08: structure 42%, naming 83%; `synchronized` 形状保留，嵌套块也正确嵌套（结构 35~49%）
