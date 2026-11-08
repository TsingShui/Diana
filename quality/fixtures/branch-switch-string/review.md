# Fixture: branch-switch-string

switch over String, with effects in two branches. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug branch-switch-string`:
  structure 18%, naming 71%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `dispatch` | 16% | 75% | `insert: ref=- ours=ID ID ; int ID` |
| `describe` | 21% | 67% | `insert: ref=- ours=int ID ; int ID` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `dispatch` | ✔ hash/equals 映射与分支体正确 |
| `describe` | ✔ 语义正确 |

### Findings

- 两段 switch：先 `hashCode()`，再 `equals()` 比对，中间夹 `graph<MultipleEntry>` 与 goto
- `describe` 里 `case "b": case "c":` 的合并分支丢结构（结构 21%）
- hash 常量（97/98/99）与 `equals` 调用都在，语义正确

### Ranked fixes

把 hash+equals 折回单一 `switch (s) { case "a": … }`，合并分支保留；去掉 `graph<MultipleEntry>`

## Rounds

- R1 2026-11-08: structure 18%, naming 71%; 两段 switch：先 `hashCode()`，再 `equals()` 比对，中间夹 `graph<MultipleEntry>` 与 goto
