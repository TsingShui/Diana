# Java explanations and Smali fixtures

A fixture is a small, independently authored reduction of one observed problem.
Java explains the intended behavior; Smali fixes the DEX instruction structure
used for reproduction. The [evolution workflow](EVOLUTION.md) owns repair and
regression acceptance. Fixture preparation follows the root
[permission](../../AGENTS.md) and [testing policy](CONTRIBUTING.md#testing-policy).

## Files and ownership

Store each fixture in `fixtures/<slug>/`, using a lowercase, hyphenated name for
the mechanism rather than the originating application.

```text
fixtures/<slug>/
├── Case.java
├── Case.smali
└── README.md
```

| File | Responsibility |
|---|---|
| `Case.java` | Readable explanation of the relevant behavior and effects. |
| `Case.smali` | Actual assembly input, including branch targets and effect order. |
| `README.md` | Provenance/license, Java–Smali correspondence, assembly commands, original selectors, expectations and observed reproduction status. |

Authors MAY add supporting source types when required by the mechanism. Authors
MUST keep APKs, generated DEX/class files, downloaded tools and inspection logs
under ignored `.cache/` storage. Fixture outputs belong under
`.cache/fixtures/<slug>/`; repository assets remain source-only.

`quality/` is the single exception: its measurement fixtures commit the generated
`classes.dex` next to the Java reference so a review needs no toolchain. Those DEX
files are review inputs, not fixture expectations, regression cases or acceptance
evidence; [quality/README](../../quality/README.md) owns their workflow.

There is no mandatory `expected.json` or per-fixture `build.sh`. Authors MUST write
independent behavioral expectations in the README. An authorized regression case
encodes those same expectations in executable checks and links back to the
fixture; it does not introduce a competing source of expected behavior.

## Java and Smali correspondence

Authors MUST preserve the same relevant behavior in both sources: branch choices,
values and required call, argument, effect, exception or lock order. Java is not
compiled as part of the required DEX preparation. Authors MUST NOT require it to
compile into identical Smali or bind fixture admission to a Java/D8/R8 version.

When a DEX behavior or malformed-input condition cannot be expressed in Java,
authors MUST explain that difference in the README and identify what the Java
example illustrates. Smali and DEX semantics determine the actual input; the
Java explanation MUST NOT silently replace them as the basis for expectations.

Reduce dependencies and unrelated application behavior, not the triggering
structure. The README MUST describe that structure explicitly, such as a branch
that skips an effectful call before reaching a shared result block. A similar
source expression or the mere presence of a constant does not establish it.

Authors MUST record provenance and review the license of copied material. Prefer
independently authored reductions of a general mechanism. Follow the
[corpus rules](CONTRIBUTING.md#corpus-data-and-sensitive-inputs); private application
identities, data and mappings remain outside the repository.

## DEX preparation

1. Use a locally available Smali assembler supporting the fixture's syntax and DEX format.
2. Assemble the authored Smali into `.cache/fixtures/<slug>/classes.dex`.
3. Record the actual assembler version, invocation and relevant format/API options.
4. Resolve original class descriptors and full method prototypes in that generated DEX.
5. Confirm that the required instruction structure survived assembly.

Fixture authors MUST NOT impose an exact assembler version as an admission gate.
A missing tool, unsupported syntax, assembly failure or absent trigger structure
is a preparation failure, not a Diana pass. Multiple DEX inputs MUST have explicit
roles. Authors MUST NOT depend on table indexes, file offsets or register choices
from another build when selecting the subject by identity.

Assembly commands MUST use portable source paths, leave other fixtures' output
intact, and keep tool acquisition explicit. Authors MUST NOT install tools
implicitly or add checksum gates; the existing
[file-verification policy](CONTRIBUTING.md#file-verification) applies. A shared
preparation helper, when authorized and implemented, owns common invocation logic
rather than duplicating toolchains across fixtures.

For a before/after comparison, assemble once and feed the same generated DEX to
both Diana revisions. Each run records its environment; cross-machine DEX byte
identity is not an acceptance requirement. Fixture sources and preparation
libraries MUST NOT become external declaration inputs to Diana.

## Expectations and reproduction evidence

The README MUST identify:

- The original class descriptor and full method name/prototype.
- The triggering DEX structure and the independent evidence establishing it.
- Each scenario's inputs, initial state and relevant environment assumptions.
- Its expected result and ordered observable effects, including call arguments
  or exceptions when material; counts alone do not establish order.
- Commands and Diana revisions for the observed reproduction, or an explicit
  unrun/non-reproducing status.

Authors MUST derive expectations from the authored input, original instruction
semantics and applicable product contracts, not from Diana's current output.
Changing an expectation requires independent evidence and review; refreshing a
snapshot to absorb a regression is not such evidence. Third-party decompiler
output and similarity scores are investigation aids, not correctness gates.

Use the existing [`dbg_shell`](../../crates/diana-decomp/README.md) for engine
inspection. Its products locate facts and failures; they do not independently
prove those facts. Use direct byte evidence or another independent source when
adjudication requires it. Local paths describe generated artifacts, not portable
evidence substitutes for source, commands and observations.

Acceptance for a prepared reproduction: the assembled input has the recorded
trigger structure and reproduces the concrete symptom on the identified Diana
revision. Assembly success, exit zero, legal Java or an empty diagnostic list
alone does not meet this criterion. Repair acceptance additionally follows
[EVOLUTION](EVOLUTION.md#acceptance-and-regression).
