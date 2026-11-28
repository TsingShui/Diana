# Diana documentation

Diana is a native Rust decompiler and static analysis tool for APK, DEX, JAR
and JVM class files. It recovers Java-like source from the analyzed input and
reports available code, original identities and material uncertainty.

The engine in `crates/diana-decomp/` owns DEX reading and source recovery for
DEX and JVM inputs. The CLI owns file and archive access. The package READMEs
describe the current interfaces and behavior.

| Document | Responsibility |
|---|---|
| [Engine README](../../crates/diana-decomp/README.md) | DEX and JVM entry points, input boundaries and fork provenance. |
| [CLI README](../../apps/diana/README.md) | Application commands, selectors and publication behavior. |
| [CLI Skill](../../apps/diana/SKILL.md) | Recommended workflows for common analysis tasks. |
| [GUI DESIGN](../../apps/diana-gui/DESIGN.md) | Host-level resource, identity, navigation, view and result contracts; not an Engine integration status claim. |
| [SOURCE_MAP.md](SOURCE_MAP.md) | Source attribution, reproducibility and evidence boundaries. |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution, Rust and acceptance rules. |
| [FIXTURES.md](FIXTURES.md) | Java explanations, Smali inputs, assembly records and independent expectations. |
| [quality/README](../../quality/README.md) | Output-quality fixtures, their committed DEX and the written reviews. |
| [EVOLUTION.md](EVOLUTION.md) | Fixture-driven validation for substantive recovery changes. |
| [WRITING.md](WRITING.md) | Documentation conventions. |

Engine checks use the crate tests and ordinary CLI requests. Build/lint success,
byte roundtrip and finite corpus checks have different evidence scopes; none alone
proves complete decompilation correctness. Production safety and module-owned
transform checks remain required.

## Regression workflow status

[The release workflow](../../.github/workflows/release.yml) builds and publishes
the CLI on `diana-v*` tag pushes or manual dispatch for an existing tag.
[DecBench](../../quality/decbench/README.md) runs its decompilation evaluation
through a [scheduled or manually dispatched workflow](../../.github/workflows/decbench.yml).
Run the [scoped format, lint and test checks](CONTRIBUTING.md#testing-policy)
locally for ordinary changes.
