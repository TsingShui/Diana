# Fixture: expr-constants

constants only. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `intValue` | ✔ 123456 |
| `longValue` | ✔ 1234567890123 |
| `floatValue` | ✔ 1.5f |
| `doubleValue` | ✔ 2.25 |
| `charValue` | ✔ The value agrees; the character is printed numerically as 120. |
| `booleanValue` | ✔ false |
| `stringValue` | ✔ LABEL |
| `nullValue` | ✔ null |
| `classValue` | ✔ The value agrees; the class literal is assigned to a temporary. |

### Findings

- The int/long/float/double/boolean/string/null constant values are recovered.
- The character 'x' is printed as 120 rather than a character literal.
- Class literals are expressed as an intermediate assignment before use. Recorded forms: `ID ID = ID . class`.

### Ranked fixes

Improve character literals such as 'x' and direct class-literal presentation. Recorded forms: `X.class`.

## Rounds

- R1 2026-11-08: The int/long/float/double/boolean/string/null constant values are recovered.
