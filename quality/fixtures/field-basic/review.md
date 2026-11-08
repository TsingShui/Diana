# Fixture: field-basic

static/instance field access. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug field-basic`:
  structure 88%, naming 83%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `readStatic` | 92% | 100% |
| `writeStatic` | 93% | 67% |
| `readLabel` | 92% | 100% |
| `readInstance` | 91% | 100% |
| `writeInstance` | 93% | 67% |
| `addToInstance` | 67% | 67% |

### Findings

- 读写都正确，差异是插入 `this .` / `X . ` 限定与 `int ID = this.ID` 临时
- `value += delta` 展开为读、加、写三步（66.7%）

### Ranked fixes

复合赋值保留 `+=`；去掉不必要的 `this.`

## Rounds

- R1 2026-11-08: structure 88%, naming 83%; 读写都正确，差异是插入 `this .` / `X . ` 限定与 `int ID = this.ID` 临时
