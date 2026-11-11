# Review: static-field-value-widths

LLM semantic review compares the Java source, prepared DEX and candidate output.
The `b5eb94d` baseline prints descriptor defaults for every nondefault encoded
scalar. Integration assertions check each independent value and the sequence.
They do not execute the generated Java or establish ART behavior.

## Semantics

| Subject | Judgment and reasoning |
|---|---|
| Signed and wide values | Agreement: byte-7, short-300, int-1234567 and long1234605616436508552 remain attached to their original fields. The following END6 field is not shifted by a preceding preparation's byte width. |
| Character and boolean | Agreement: CHAR is UTF-16 unit81 (`Q`), and BOOL is true. Neither is replaced by its default value. |
| Floating-point values | Agreement: FLOAT1.25, DOUBLE-2.5 and NEGATIVE_ZERO-0.0 preserve their values. Negative zero's sign survives instead of becoming positive zero. |
| String units | Agreement: LABEL is `tail`. SURROGATE must remain exactly one D800 UTF-16 unit and print as an escape, never null or a replacement scalar. |
| Zero and initializer | ZERO remains0. The SEQUENCE initializer remains [-7,-300,-1234567,6] in original order; the scalar fix introduces no additional initializer or reordered array write. |

The original static-field-values fixture remains covered alongside this challenge.
Both fixture expectations were authored before inspecting candidate output.
