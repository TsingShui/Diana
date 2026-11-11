# Fixture: static-field-value-widths

This independently authored challenge extends
[static-field-values](../static-field-values/README.md). It combines static
preparations of different encoded widths and kinds, signed values, a string,
negative floating-point zero, an isolated UTF-16 surrogate, an implicit zero and
a separate class initializer.
Original descriptor: `Lfixture/StaticFieldValueWidths;`. No third-party data is
included.

## Independent expectations

The field declarations retain the values in the Java source. In particular,
LONG is 1234605616436508552, FLOAT is 1.25, DOUBLE is -2.5, CHAR is UTF-16 unit81,
BYTE is -7, SHORT is -300, INT is -1234567 and BOOL is true. NEGATIVE_ZERO keeps
its sign bit; ZERO remains zero. LABEL is `tail`, and SURROGATE retains the single
UTF-16 unit D800 rather than becoming null or a replacement character. The class initializer creates
and publishes SEQUENCE in the order [-7, -300, -1234567, 6]. Reading one static
ordinal must not shift values onto another field or replace encoded values with
descriptor defaults.

Pre-fix revision: `b5eb94d`. Candidate: recovery-fixes branch. Prepare the Java
source with `javac --release 8 -g`, then `d8 --min-api 24`, using
`.cache/fixtures/static-field-value-widths/` for generated class files and DEX.
Copy the prepared DEX beside the source. The actual environment is javac
21.0.12.1 and D8 9.2.4-dev; those versions are recorded, not required.
