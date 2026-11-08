# Fixture: complex-indy-lambda

lambdas, captured lambdas and every method-reference kind. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ComplexIndyLambda;`
- Current numbers from `python3 quality/score.py --slug complex-indy-lambda`:
  structure 32%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `field` | ✔ |
| `staticValue` | ✔ |
| `constructorRef` | ✔ method_handle … kind=6; StringBuilder<(String) -> void> 保留 |
| `listFactory` | ~ 未逐行核对 |
| `arrayFactory` | ~ 未逐行核对 |
| `captured` | ✔ 捕获以 invoke<Custom> call_site<…>(this, p0) 传入，lambda 体是 lambda$captured$1 |
| `boundField` | ~ 未逐行核对 |
| `unboundStatic` | ~ 未逐行核对 |
| `stringOps` | ? 未判定 |

### Score

| method | structure |
|---|---|
| `field` | 100% |
| `staticValue` | 100% |
| `constructorRef` | 14% |
| `listFactory` | 14% |
| `arrayFactory` | 12% |
| `captured` | 9% |
| `boundField` | 12% |
| `unboundStatic` | 12% |
| `stringOps` | 15% |

### Findings

- 方法引用/构造器引用全部走 LambdaMetafactory.metafactory 的 call_site 元数据 + invoke<Custom>
- 结构分 9%~15%（field/staticValue 100%），差异全部来自 indy 形态
- 捕获参数出现在 invoke<Custom> 实参里（this, p0），语义可读但不像 Java

### Ranked fixes

把 LambdaMetafactory 的三种情形还原成 X::new / this::m / x -> …；元数据移出正文

## Rounds

- R1 2026-11-08: structure 32%; 方法引用/构造器引用全部走 LambdaMetafactory.metafactory 的 call_site 元数据 + invoke<Custom>
