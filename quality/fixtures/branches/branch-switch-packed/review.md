# Fixture: branch-switch-packed

packed switch with fallthrough, grouped cases, negative and default arms. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/BranchSwitchPacked;`
- Current numbers from `python3 quality/score.py --slug branch-switch-packed`:
  structure 54%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `classify` | ✔ fallthrough（case 2 落入 case 3）与分组 case（0/1）都正确 |
| `clustered` | ✔ 累积型 fallthrough（case 10 落入 11）正确 |

### Score

| method | structure |
|---|---|
| `classify` | 53% |
| `clustered` | 56% |

### Findings

- `switch` 的每个 case 都被改成 `goto Lx`，case 体集中到 `graph<MultipleEntry>` 里，`default:` 排在最前
- fallthrough 语义保住了（`L3: state=2;` 直接落到 `L4:`），但读起来要跟标签
- 结构 53%~56%，扣分全部来自 goto/graph 而不是取值

### Ranked fixes

把 case 体留在 switch 内、`default` 放最后；避免 `graph<MultipleEntry>`

## Rounds

- R1 2026-11-08: structure 54%; `switch` 的每个 case 都被改成 `goto Lx`，case 体集中到 `graph<MultipleEntry>` 里，`default:` 排在最前
