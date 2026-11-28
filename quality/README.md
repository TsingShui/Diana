# Quality fixtures and review

Diana keeps focused fixtures under `quality/fixtures/<theme>/<slug>/`. Each
fixture directory holds its input or preparation recipe, original identities,
and independent expectations. When a fixture includes a prepared `classes.dex`,
use it directly for inspection. Follow the fixture's recipe to regenerate
changed inputs. [Fixture preparation](../docs/diana/FIXTURES.md)

## Workflow

1. Select a fixture and read its source, recipe, and expectations. For a
   substantive change to semantics or recovery, use the discovery case,
   complexity challenge, and affected prior cases.
   [Repair workflow](../docs/diana/EVOLUTION.md#primary-iteration)
2. Run `diana classes` or `diana methods` on the prepared input. Copy the
   relevant `at` coordinate verbatim and quote it when passing it to
   `get-class` or `get-method`. Record both the coordinate and the original
   descriptor or full method prototype. Capture a fresh coordinate from each
   input when comparing different builds.
   [Original identity and selection](../apps/diana/README.md#original-identity-and-selection)
3. Compare the selected result with the original source and instructions. Keep
   the available publication and its diagnostics when retrieval reports
   incomplete recovery.
4. Run the relevant DecBench suites and Nextest tests. Record the revision,
   input, command, exercised scenarios, and actual outcome.

For example, build the CLI and inspect the packed-switch fixture:

```sh
cargo build --locked -p diana
dex=quality/fixtures/branches/branch-switch-packed/classes.dex
target/debug/diana classes "$dex"
target/debug/diana methods "$dex" --class fixture.BranchSwitchPacked
target/debug/diana get-class "$dex" 'e0:l0:c0'
target/debug/diana get-method "$dex" 'e0:l0:c0:m1'
```

The coordinates above refer to that checked-in DEX. For a JVM fixture, build
its authored input using the local recipe and use the same lookup and
retrieval sequence with the JVM coordinates printed for that input.

### Optional review dump

`dump.py` can pair readable Java and Diana output for one simple DEX fixture.
It uses the first source file, pairs method bodies by name, and skips
constructors. Overloads, other classes, and results from a retrieval that
exits nonzero can therefore be absent from the dump. Use original coordinates
for evidence about a specific member.

```sh
python3 quality/dump.py --binary target/debug/diana --slug branch-switch-packed
```

## Reviews

Use the fixture's `review.md` for path-by-path semantic reasoning. Compare
results, ordered effects, exceptions, and relevant helper behavior. Describe
readability and notation findings separately and mark any unresolved paths.
For Kotlin fixtures, compare against the authored Kotlin and check the lowered
behavior. Record executed scenarios and outcomes in the fixture's README.
[Review and acceptance criteria](../docs/diana/EVOLUTION.md#iteration-decision)

## Diana DecBench

DecBench evaluates one built executable against independent fixture
expectations. Use `suites` to locate a manifest and `run --suite` to scope the
evaluation. An authored method suite also accepts `--at` for one member.
Use the same input and oracle for comparison across revisions.
[DecBench commands and evidence](decbench/README.md)

```sh
python3 quality/decbench suites
python3 quality/decbench run --binary target/debug/diana \
  --suite dex-structure-branch-switch-packed --at 'e0:l0:c0:m1' \
  --revision "$(git rev-parse HEAD)" --output .cache/decbench/switch.json
```

## Source-aligned APK reviews

Public application reviews record pinned APK and source provenance, selection
and retrieval coverage, method-level comparisons, and evidence limits. Keep
downloaded inputs and generated output in ignored local storage.
Replay AntennaPod and NewPipe through the public corpus commands.
[Public DecBench corpus](decbench/README.md#replay-a-public-apk-corpus) ·
[AntennaPod 3.12.3 review](reports/antennapod-3.12.3.md)

## Fixtures

| Theme | Examples of what it covers |
| --- | --- |
| [Algorithms](fixtures/algorithms/) | Sorting, graphs, search, dynamic programming, matrices |
| [Atoms](fixtures/atoms/) | Instructions, values, arrays, calls, type views |
| [Branches](fixtures/branches/) | Conditions, switches, fallthrough, joins |
| [Classes](fixtures/classes/) | Declarations, initialization, nested types |
| [Comparison](fixtures/comparison/) | Method comparison across APK and DEX versions |
| [Composite](fixtures/composite/) | Interactions among control flow, calls, effects, and recovery |
| [Exceptions](fixtures/exceptions/) | Handler, cleanup, rethrow, and resource behavior |
| [Export](fixtures/export/) | Multidex, archive resources, partial output |
| [Generics](fixtures/generics/) | Inference, nested calls, and signatures |
| [JVM](fixtures/jvm/) | Class and JAR inputs, bytecode, metadata, archive projects |
| [Kotlin](fixtures/kotlin/) | Null handling, generated members, properties, and suspend functions |
| [Loops](fixtures/loops/) | Iteration, backedges, and nested exits |
| [Obfuscated](fixtures/obfuscated/) | R8 and ProGuard inputs with mappings |
| [Synchronization](fixtures/sync/) | Monitor regions and cleanup |

### Obfuscated fixtures

Use a fixture's `mapping.txt` to match authored owners, names, and full
prototypes to the definitions in its DEX. Select the exact DEX definition
with the coordinate printed by Diana. The mapping is review evidence.
