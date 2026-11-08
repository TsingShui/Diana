# Java explanations and Smali fixtures

A fixture is a focused, independently authored input for discovery, reproduction,
regression or a complexity challenge. Java explains the intended behavior; Smali
fixes the DEX instruction structure. A reduction isolates an observed cause; a
challenge retains it while adding interacting mechanisms. The primary
[evolution workflow](EVOLUTION.md#primary-iteration) requires new complexity
challenges after repair and owns iteration and behavioral-regression decisions. Fixture preparation follows the root
[permission](../../AGENTS.md) and [testing policy](CONTRIBUTING.md#testing-policy).

## Files and ownership

Store every fixture in `quality/fixtures/<theme>/<slug>/`, including quality,
reproduction and regression inputs. Reuse the corresponding theme or create it
when absent. Use a lowercase, hyphenated slug for the mechanism rather than the
originating application. Do not create a top-level or package-local fixture tree.

```text
quality/fixtures/<theme>/<slug>/
├── Case.java
├── Case.smali
├── classes.dex
└── README.md
```

| File | Responsibility |
|---|---|
| `Case.java` | Readable explanation of the relevant behavior and effects. |
| `Case.smali` | Actual assembly input, including branch targets and effect order; when present, it takes precedence over compiling the Java explanation. |
| `classes.dex` | Prepared input kept with its sources so inspection and tests need no assembler. |
| `README.md` | Provenance/license, Java–Smali correspondence, assembly commands, original selectors, expectations and observed reproduction status. |

Authors MAY add supporting source types when required by the mechanism. Authors
MUST keep APKs, generated class files, downloaded tools, scratch DEX and inspection
logs under ignored `.cache/` storage. Assemble fixture DEX under
`.cache/fixtures/<slug>/`, then copy the successful result to the fixture's
`classes.dex`. Only `quality/fixtures/**/classes.dex` is exempt from the
generated-output ban. A failed assembly MUST NOT publish or test a stale result.

[quality/README](../../quality/README.md) describes the shared fixture directory
and written-review workflow. Sharing a location does not turn DEX inputs or
written reviews into executed behavioral results. Reviews support the primary
iteration decision under EVOLUTION; behavioral-regression acceptance requires
its separately executed checks.

There is no mandatory `expected.json` or per-fixture `build.sh`. Authors MUST write
independent behavioral expectations in the README. An authorized regression case
encodes those same expectations in `crates/diana-decomp/tests/<case>.rs` and links
back to the fixture; it does not introduce a competing source of expected behavior.
Engine fixture tests MUST use the ordinary Engine entry and original selectors.
Shared helpers belong in `tests/support/` when actual consumers require them;
production stage modules MUST NOT hold fixture test implementations.

## Complexity challenge records

Follow [EVOLUTION](EVOLUTION.md#complexity-challenge) for the primary repair loop.
Keep each new challenge beside its own Java/Smali, DEX, expectations and review;
link the original discovery fixture rather than replacing or expanding it in place.
Its README MUST identify the repaired mechanism/invariant, the added interaction,
the parent fixture, required scenarios and pre-fix/candidate revisions. Its review
MUST explain both generalization and preservation of earlier effects. Keep
compatibility guards distinct from cases demonstrating a new repair benefit.

Authors MUST derive helper arguments and fault reachability from the input:
an acquisition ID is not necessarily its returned handle, and a selected fault
cannot occur if an earlier branch skips its call. Native declarations require an
explicit helper model; models are not claims of an executed implementation.

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
3. Copy the successfully prepared DEX to `quality/fixtures/<theme>/<slug>/classes.dex`.
4. Record the actual assembler version, invocation and relevant format/API options.
5. Resolve original class descriptors and full method prototypes in that generated DEX.
6. Confirm that the required instruction structure survived assembly.

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
[EVOLUTION's iteration decision](EVOLUTION.md#iteration-decision) and its separate
[behavioral-regression criteria](EVOLUTION.md#acceptance-and-regression).

## Fixture review

Authors MUST use LLM semantic-equivalence review as the primary method for written
fixture reviews. Compare the authored input/source behavior and helper bodies
with current decompiled output, path by path. Record the original member,
reviewed scenarios, reasoning and unresolved paths in `review.md`.

Structural checks, similarity scores and test results supplement this review;
they do not replace semantic reading. A written judgment is not a claim that
behavioral execution passed and does not override a failing test. The detailed
[quality review workflow](../../quality/README.md#workflow) keeps semantic
agreement, concrete mismatches, uncertainty and readability findings distinct.
