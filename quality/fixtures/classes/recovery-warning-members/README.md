# Fixture: recovery-warning-members

This presentation challenge extends the deliberately uninitialized `broken`
method in [direct-initialization](../direct-initialization/README.md). It retains
the same missing-constructor trigger and adds constructor/class-initializer
declarations, independently readable sibling methods, and an original throw
whose message resembles Diana's old diagnostic. Smali is authoritative; Java
explains the representable helpers and the malformed bodies. No ART acceptance
or executable behavior of the malformed methods is claimed.

## Independent expectations

| Original member | Required publication |
|---|---|
| `Lfixture/RecoveryWarningMembers;->broken()Ljava/lang/Object;` | A Diana warning comment with the issue URL and missing-constructor reason. No invented throw, return, constructor invocation or executable replacement body. |
| `Lfixture/BrokenConstructor;-><init>()V` | The constructor declaration remains visible with the same warning comments. Its issue retains the original `<init>` identity. |
| `Lfixture/BrokenInitializer;-><clinit>()V` | The static initializer remains visible with the same warning comments. Its issue retains the original `<clinit>` identity. |
| `Lfixture/RecoveryWarningMembers;->originalThrow()V` | One original UnsupportedOperationException construction with the authored message, followed by its throw. Its diagnostic-like string does not turn the original throw into a Diana warning. |
| Each `sibling()I` | The original return value remains 7, 9 or 11, respectively. These selected-method requests succeed independently of their failed sibling. |

Every failed class/method request publishes its available text and returns exit
code 1 with a structured issue and stderr diagnostic. Text and JSON contain
identical source text. The warning comments express unavailability, not an empty
implementation or recovered execution semantics.

The baseline is the recovery-fixes release executable before the warning-comment
change, based on `b5eb94d` with the preceding recovery fixes. The candidate is the
same branch with this presentation change. Both inspect the same prepared DEX.

## Preparation

```sh
mkdir -p .cache/fixtures/recovery-warning-members
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/recovery-warning-members/classes.dex \
  quality/fixtures/classes/recovery-warning-members
cp .cache/fixtures/recovery-warning-members/classes.dex \
  quality/fixtures/classes/recovery-warning-members/classes.dex
```

The prepared DEX is published beside the sources. Assembly used Smali 2.5.2 on
OpenJDK 21.0.12.1; exact tool versions are not admission requirements.

## Observed inspection

The same prepared DEX was inspected with `get-class` and selected `get-method`
requests on the baseline and candidate. The baseline emitted executable
UnsupportedOperationException placeholders for the three failed member kinds.
The candidate emitted two Diana comment lines for each failure and retained
the exact structured issues, exit code 1 and stderr diagnostics. Text and JSON
source presentations were byte-identical. Each selected sibling request
succeeded with its independently specified return value. The selected original
throw request succeeded and retained the input's exception and message.

These are CLI publication and structural observations, supported by the
path-by-path review. The malformed methods were not executed on ART.

Regression checks passed: 526 nextest cases, workspace Clippy and the release
workspace build. A release-to-release comparison covered all 138 prepared DEXes
and 229 classes. Only this fixture's three classes and the original
direct-initialization class changed, each replacing a generated failure throw
with warning comments. Structured issues, exit codes and stderr were unchanged.
