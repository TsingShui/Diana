# Fixture: exception-one-catch

one try, one catch. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug exception-one-catch`:
  structure 48%, naming 57%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `catchReturns` | 55% | 50% |
| `catchRethrows` | 56% | 60% |
| `sideEffectsInTry` | 34% | 60% |

### Findings

- 语义全部正确（catch 返回、catch 内 rethrow、try 内效果顺序都保住）
- 每个操作挂 `[exn c0]`，方法头多一行 `exception_chain`，try 前多 `nop;`

### Ranked fixes

`[exn cN]`/`exception_chain`/`nop` 移出正文；`catch` 形如 `catch (T e)`

## Rounds

- R1 2026-11-08: structure 48%, naming 57%; 语义全部正确（catch 返回、catch 内 rethrow、try 内效果顺序都保住）
