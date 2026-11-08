# Fixture: invoke-basic

static/virtual/interface/constructor calls. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug invoke-basic`:
  structure 92%, naming 62%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `staticCall` | 100% | 75% |
| `virtualCall` | 100% | 50% |
| `interfaceCall` | 100% | 50% |
| `constructorCall` | 82% | 33% |
| `manyArguments` | 88% | 100% |
| `listCall` | 81% | 62% |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `staticCall` | ✔ |
| `virtualCall` | ✔ |
| `interfaceCall` | ✔ |
| `constructorCall` | ✔ 构造正确，经临时返回 |
| `manyArguments` | ✔ |
| `listCall` | ✔ 泛型实参丢失，属形态 |

### Findings

- 四种调用都 100% 结构一致
- 构造器调用多出临时变量（82%）；泛型 `List<Integer>` 的类型实参丢失（81%）

### Ranked fixes

构造器调用直接返回；保留泛型实参

## Rounds

- R1 2026-11-08: structure 92%, naming 62%; 四种调用都 100% 结构一致
