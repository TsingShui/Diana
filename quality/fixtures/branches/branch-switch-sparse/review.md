# Fixture: branch-switch-sparse

sparse switch with non-contiguous and large case values. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/BranchSwitchSparse;`
- Current numbers from `python3 quality/score.py --slug branch-switch-sparse`:
  structure 55%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `map` | ✔ 非连续值与 default 正确 |
| `table` | ✔ d8 把它降级成 packed 表（含空洞），空洞落 default 正确 |

### Score

| method | structure |
|---|---|
| `map` | 88% |
| `table` | 23% |

### Findings

- `table` 的 7 个非连续 case 被 d8 降成 packed 表，我们把 13 个档位（含 6 个空洞）全部列出并指向 default，语义对但很啰嗦
- `map` 结构 87%：`default: return 0;` 被提前到 switch 之前
- 两个方法都用了 `graph<MultipleEntry>` + `goto Lx`

### Ranked fixes

default 归位；空洞档位不必逐条列出

## Rounds

- R1 2026-11-08: structure 55%; `table` 的 7 个非连续 case 被 d8 降成 packed 表，我们把 13 个档位（含 6 个空洞）全部列出并指向 default，语义对但很啰嗦
