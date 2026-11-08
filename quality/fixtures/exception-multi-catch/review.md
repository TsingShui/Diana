# Fixture: exception-multi-catch

one try, one multi-catch clause, one caught rethrow. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug exception-multi-catch`:
  structure 43%, naming 80%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `parse` | 43% | 80% | `insert: ref=- ours=int ID ; ID ID` |
| `rethrow` | 42% | 80% | `insert: ref=- ours=int ID ; ID ID` |

### Findings

- multi-catch 被拆成两个 `catch` 加 `goto` 汇合，`|` 形态丢失
- 方法头多出 `int v; Type v;` 前置声明与 `exception_chain c0`
- 语义正确：两个 catch 都置 `state` 后返回/重抛

### Ranked fixes

还原 `catch (A | B e)`；去掉前置声明与 `exception_chain`；参数名恢复

## Rounds

- R1 2026-11-08: structure 43%, naming 80%; multi-catch 被拆成两个 `catch` 加 `goto` 汇合，`|` 形态丢失
