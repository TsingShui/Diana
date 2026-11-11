# Fixture: static-field-values

This fixture isolates original static-field preparation values. The class has no
method logic beyond the compiler-provided constructor; its purpose is to verify
that P0 preparation scalars reach the rendered field declarations instead of
falling back to each type's default.

## Provenance and preparation

The Java source was authored for this repository. No third-party code or data is
included. The committed DEX was produced with `javac --release 8 -g` and
`d8 --min-api 24` from that source. The fixture can be regenerated with the
repository `quality/build.sh` toolchain; the committed input is sufficient for
inspection and tests.

Recorded preparation environment: `javac 21.0.12.1` and D8
`9.2.4-dev` (`084a89126ae0596c599143df57a654646af28313`).

Original descriptor: `Lfixture/StaticFieldValues;`.

## Expectations

The original class data stores these static preparations:

| field | value |
|---|---|
| `CLOSE` | `5` |
| `PREPARE_CAPTURE` | `3` |
| `PREPARE_PREVIEW` | `1` |
| `PROCESS_CAPTURE_IMAGE` | `4` |
| `PROCESS_PREVIEW` | `2` |
| `LABEL` | `"fixture"` |

The rendered class must preserve each value. A declaration that uses the
descriptor default (`0` for `int`, `null` for `String`) is a semantic failure,
even if the class remains syntactically readable.
