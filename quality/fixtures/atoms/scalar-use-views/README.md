# Range-proved scalar-use views

## Provenance and trigger

Independently authored from the general character-use disclosure problem C1 in
[the source-aligned APK review](../../../reports/antennapod-3.12.3.md).
No upstream application source or DEX is copied. The repository license applies.
`Case.smali` is the input; Java explains the same selected calls and arithmetic.

Original owner: `Lfixture/ScalarViews;`.
Subjects: `carrier(Ljava/lang/String;)I`, `array([C)Ljava/lang/String;`,
`literal(Ljava/lang/String;)Ljava/lang/String;`,
`escaped(Ljava/lang/String;)Ljava/lang/String;`, `boundaries()I` and `parameter(C)I`.

The trigger is a range-proved P4/P7 IntegralView at a use, while its runtime local
retains word storage. The same value reaches char and int overloads. Constant
character, byte and short uses exercise the literal path too. A correct binding
retained only in JSON does not resolve the ambiguous plain-text carrier spelling.
Original numeric storage and argument order remain authoritative.

## Independent expectations

Recorded before the first candidate inspection. These are source/Smali models,
not observed behavior executions. Java integer operations wrap as DEX operations.
The five `probe` helpers have actual bodies: char adds 1000, int adds 2000,
byte adds 3000, short adds 4000; canonical boolean selects 5001 or 5000.

| Subject/input | Result and ordered operations |
|---|---|
| `carrier("A")` | `3130`: charAt(0) returns65; `probe(C,65)` returns1065, then `probe(I,65)` returns2065, then add. Treating both calls as int would instead give4130. |
| `carrier("")` / null | charAt fails before either probe; no invented default unit or probe call. |
| `array({'A'})` | `"A65"` under the ordinary StringBuilder model: constructor, char-array load, append(C,65), append(I,65), toString. |
| `array({})` / null | Constructor precedes the failing array read; both appends and toString are skipped. |
| `literal("a/b/")` | `"a-b-"`, one original replace(CC) with char47 then char45. |
| `escaped("'x")` | UTF-16 units `[0xd800, 0x0078]`; single quote and isolated surrogate stay exact, escaped character literals. |
| `boundaries()` | `50640`: probe(C,65535)=66535, probe(B,-128)=2872, probe(S,-32768)=-28768, probe(Z,true)=5001, probe(Z,false)=5000; add in that order. |
| `parameter('A')` | `1065`; the declared char parameter already states the view, without inventing a conversion or changing its binding. |

Required recovery: expose selected narrow scalar interpretations at their actual
uses, including character literals, while retaining word storage and int uses.
There is no new runtime cast/check/truncation operation. A value-preserving view
spelling consumes existing P4 range evidence, never a desired parameter type
alone. Copy/snapshot contracts and original expression/operation anchors remain.
Unknown library runtime behavior is outside the model.

## Preparation and observation

Assemble once, then use the same bytes on baseline and candidate:

```sh
mkdir -p .cache/fixtures/scalar-use-views
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/atoms/scalar-use-views/Case.smali \
  -o .cache/fixtures/scalar-use-views/classes.dex
cp .cache/fixtures/scalar-use-views/classes.dex \
  quality/fixtures/atoms/scalar-use-views/classes.dex
```

Baseline: `df10b744985e3407f902442b8252fab100700aec`.
Observed preparation used Smali 2.5.2, OpenJDK 21.0.12.1 and API 24; these
identify the run, not an assembler-version admission pin. Class0 subjects are
array/method0, boundaries/1, carrier/2, escaped/3, literal/4 and parameter/5.

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/atoms/scalar-use-views/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo nextest run --locked -p diana-decomp --test scalar_use_views --no-fail-fast
```

The original six checks compiled and ran before production edits: one pass,
five failures. All six pass on the candidate. With the fresh challenge and
retry/resource guards, the final same-source/same-input test run is six
passes/seven failures on the baseline versus13/13 on the candidate. These
results establish product/text sensitivity, not executed scenario behavior.
See [review.md](review.md) and the linked challenge for the bounded iteration
judgment. Native/library execution and Java-output compilation were not run.
