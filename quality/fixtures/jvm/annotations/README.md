# JVM declaration annotations

`Annotated.java` exercises runtime-visible and runtime-invisible declaration
annotations, nested annotations, primitive arrays, a class literal, an enum
member and scalar values. The method has a wide parameter before an ordinary
integer parameter. A separate enum constructor has two compiler-synthetic slots
before its annotated source parameter. The fixture also includes an
annotation default, a return type-use annotation and a string ConstantValue.

The preparation script compiles with OpenJDK and emits Java 17 class files.
It preserves the order of six class records in the three archive variants.

- `annotations.jar` retains the original output of the compiler.
- `damaged.jar` replaces the final byte of one modified UTF-8 annotation
  string with an invalid zero byte. Its byte extent, all later annotation
  values and the rest of the class remain available.
- `unaligned.jar` changes the enum's `MethodParameters` attribute name to
  an unrecognized name of the same length. This preserves the physical
  descriptor and parameter annotation table while withholding the evidence
  needed to map the passed table index to the original parameter slot.

The local verification used OpenJDK 21.0.12.1 and Python 3.13.

```sh
python3 quality/fixtures/jvm/annotations/prepare.py .cache/fixtures/jvm-annotations
cargo nextest run -p diana --test jvm_annotation_test --locked
```
