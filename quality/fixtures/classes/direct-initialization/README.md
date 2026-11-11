# Fixture: direct-initialization

This independently authored reduction isolates an allocation whose type differs
from its direct constructor target. It contains no application or corpus data.
The Smali is the input; Java explains the relevant helper behavior and the
representation difference. Java cannot express `direct` and does not replace
the original instruction semantics. Assembler acceptance is not ART verification.

## Subjects and expectations

- `Lfixture/DirectInitialization;->direct(I)Lfixture/Child;` allocates Child,
  directly calls `Parent.<init>(I)V` on that reference, and returns that same
  object. For argument 7, normal completion leaves `Parent.value` equal to 7.
  Calling Child's constructor instead would change it to 99. Allocation failure
  prevents initialization; a constructor failure prevents the return.
- `Lfixture/DirectInitialization;->ordinary(I)Lfixture/Child;` directly calls
  Child's constructor and can retain natural `new Child(value)` syntax. Its
  normal result has value 99.
- `Lfixture/DirectInitialization;->broken()Lfixture/Child;` deliberately omits
  initialization. Recovery must report a method-local failure as Diana warning
  comments, retaining the reason and issue URL without executable placeholders. Whole-class
  retrieval must retain the other methods and exit nonzero; the selected
  `direct` and `ordinary` requests must remain independent of this failure.

Recovery must preserve the exact constructor owner/prototype and the allocation
type. A missing hierarchy relation is not evidence of a substitute constructor.

## Preparation

Assemble the Smali with a locally available assembler:

```sh
mkdir -p .cache/fixtures/direct-initialization
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/direct-initialization/classes.dex \
  quality/fixtures/classes/direct-initialization
cp .cache/fixtures/direct-initialization/classes.dex \
  quality/fixtures/classes/direct-initialization/classes.dex
```

The recorded assembler is Smali 2.5.2 on OpenJDK 21.0.12.1. No exact tool version
is an admission requirement. The committed DEX permits checks without assembly.
The pre-fix revision is `b5eb94d`; the candidate is the recovery-fixes branch.
