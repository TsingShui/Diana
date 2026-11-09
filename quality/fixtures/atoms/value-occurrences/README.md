# Value occurrences in reused storage

## Input and independent expectations

These Java and Smali sources are independently authored for Diana. Smali is the
assembly input. Explicit Moves force copy-affinity storage reuse without equating
their original values or states. Expectations were written before inspecting
baseline or candidate output.

Owner: `Lfixture/ValueOccurrences;`.

| Member | Independent behavior and recovery requirement |
|---|---|
| `chain(II)I` | Return wrapping `(a + b) * 3`; identity Moves must not prevent expression composition. Cases (1,2) -> 9, (-4,2) -> -6, (2147483647,1) -> -2147483648. |
| `effects(I)I` | Execute tick(x), then pair(x+7,2), exactly once each. With x=3 return 102 and increment visits once. A composed input must not disappear inside an origin-only Move. |
| `shared(I)I` | Execute tick(x) once, then pair(saved,saved). With x=3 return 110 and increment visits once. Two reads require one materialization, not two ticks. |
| `repeated(I)I` | Execute tick(x), then tick(x+7). With x=3 return 17 and increment visits twice. Calls must not disappear or reverse. |
| `bytes()[B` | Allocate one fresh byte array containing [1,2,3]. A reused representative must not block its exact payload literal. |
| `checked(Ljava/lang/Object;)I` | Execute identity(x), then check String, then length. For "abc" return 3; for null throw NullPointerException after the successful cast; for a non-String object throw ClassCastException before length. |

tick increments visits and returns wrapping x+7. pair returns wrapping a*10+b;
identity returns its argument unchanged. These are actual helper bodies, not
unmodeled native declarations.

## Preparation and evidence

Assemble once with the locally available Smali assembler into
`.cache/fixtures/value-occurrences/classes.dex`, then publish `classes.dex`
beside these sources. Baseline is f83f2fa. Candidate is the uncommitted
`work/value-aware-recovery` worktree based on f83f2fa.

Prepared with Smali 2.5.2, API 28, and Temurin OpenJDK 21.0.12.1 on macOS
arm64. The version records the actual environment; it is not an admission pin.
From the repository root, with SMALI_CLASSPATH pointing at an available assembler:

```sh
mkdir -p .cache/fixtures/value-occurrences
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble -a 28 \
  quality/fixtures/atoms/value-occurrences \
  -o .cache/fixtures/value-occurrences/classes.dex
cp .cache/fixtures/value-occurrences/classes.dex quality/fixtures/atoms/value-occurrences/classes.dex
```

Inspection used the same prepared DEX with f83f2fa and the accompanying candidate
through dbg_shell p9-all. Rust checks used rustc 1.98.0 and cargo-nextest 0.9.149.
The `value_occurrences` integration target runs 11 tests: f83f2fa passes 5 and
fails 6; the candidate passes all 11. These include recovery/origin checks and a
small linear integer executor, not general method behavioral acceptance. See review.md
for the path reasoning and its unexecuted scopes.
