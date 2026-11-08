# Fixture: class-inner-member

member inner class with a synthetic accessor to a private field. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug class-inner-member`:
  structure 84%, naming 67%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `inner` | 77% | 33% | `replace: ref=return ours=ID ID =` |
| `currentTotal` | 91% | 100% | `insert: ref=- ours=this .` |

### Findings

- 外部类两个方法语义正确；`inner()` 走临时变量再返回，`currentTotal()` 加 `this.` 限定
- 内部类单独渲染为 `ClassInnerMember$Inner`，外部输出只出现 `new Inner(...)`，synthetic accessor 不可见
- 结构 77%（方法体本身很短，扣分主要来自临时变量）

### Ranked fixes

返回值不再中转临时；内部类与其 synthetic accessor 的调用点关系需单独评审

## Rounds

- R1 2026-11-08: structure 84%, naming 67%; 外部类两个方法语义正确；`inner()` 走临时变量再返回，`currentTotal()` 加 `this.` 限定
