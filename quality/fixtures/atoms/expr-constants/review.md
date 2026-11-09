# Fixture: expr-constants

constants only. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — exact character-use spelling

Compared df10b74 with this rendering change on the same DEX. Only charValue
changes text: its existing range-proved char120 now prints as 'x'. P2–P8
products and every other method's text are unchanged; prior bounded judgments
below are reused. No fixture behavior or Java-output compilation was executed.

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `intValue` | ✔ 123456 |
| `longValue` | ✔ 1234567890123 |
| `floatValue` | ✔ 1.5f |
| `doubleValue` | ✔ 2.25 |
| `charValue` | ~ The same UTF-16 unit120 prints as 'x' from its supplied char view, without a conversion. |
| `booleanValue` | ✔ false |
| `stringValue` | ✔ LABEL |
| `nullValue` | ✔ null |
| `classValue` | ✔ The value agrees; the class literal is assigned to a temporary. |

### Findings

- The int/long/float/double/boolean/string/null constant values are recovered.
- The character 'x' now prints as a character literal from its existing per-use view.
- Class literals are expressed as an intermediate assignment before use. Recorded forms: `ID ID = ID . class`.

### Ranked fixes

Improve direct class-literal presentation while preserving resolution order. Recorded forms: `X.class`.

## Rounds

- Scalar-use disclosure: replaced numeric char120 spelling with the exact 'x' literal; other constant views stay unchanged.
- R1 2026-11-08: The int/long/float/double/boolean/string/null constant values are recovered.
