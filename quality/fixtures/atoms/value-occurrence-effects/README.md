# Value occurrences with captured effects

## Challenge and independent expectations

These Java/Smali sources are independently authored for Diana after the initial
value-use repair. The discovery fixture is
[Value occurrences](../value-occurrences/README.md). Smali is authoritative.
The retained trigger is copy-affinity reuse of several original values in one
storage representative. Added interactions are a shared arithmetic result, three
ordered helper calls, a protected later call observing an earlier saved result,
and two array payloads separated by a publication.

Expectations were written before inspecting this challenge's engine output.
Owner: `Lfixture/ValueOccurrenceEffects;`.

- `combined(II)I`: compute wrapping a=(x+7+y)*3, then pair(a,a), then tick(pair).
  For (3,2), calls are tick(3), pair(36,36), tick(396), result 403, visits 2.
  For (-8,2), calls are tick(-8), pair(3,3), tick(33), result 40, visits 2.
  The shared value must execute its producer once; arithmetic must stay before pair.
  The independently derived overflow case (650752614,0) produces a=1952257863
  and wrapping pair(a,a)=13: calls are tick(650752614), pair(a,a), tick(13),
  then ArithmeticException with visits 2. Input (13,2) instead fails on the first
  tick with visits 1 and no pair. These cases distinguish the two fault positions.
- `protectedValue(I)I`: the first tick is outside the catch. Input 1 returns 15
  after tick(1), tick(8), visits 2. Input 6 calls tick(6), tick(13), catches the
  latter ArithmeticException and returns the earlier saved 13, visits 2.
  Input 13 increments visits once and throws outside the catch, with no second call.
- `twoArrays()[B`: allocate/fill [1,2,3], publish that object with sink, then
  allocate/fill a distinct [4,5,6] and return the second object. observed must
  reference the first object, never the returned one.

tick increments visits before checking x==13, throws ArithmeticException when
equal, otherwise returns wrapping x+7. pair returns wrapping a*10+b. sink writes
the actual reference to observed. All helper bodies are included in Smali.
No device execution or unspecified native helper is assumed.

## Preparation and evidence

Prepare once with the available Smali assembler using API 28 under
`.cache/fixtures/value-occurrence-effects/`, then publish `classes.dex`.
Baseline: f83f2fa. Candidate: uncommitted work/value-aware-recovery on f83f2fa.

Prepared with Smali 2.5.2, API 28, and Temurin OpenJDK 21.0.12.1 on macOS
arm64. The version records the actual environment; it is not an admission pin.
From the repository root, with SMALI_CLASSPATH pointing at an available assembler:

```sh
mkdir -p .cache/fixtures/value-occurrence-effects
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble -a 28 \
  quality/fixtures/atoms/value-occurrence-effects \
  -o .cache/fixtures/value-occurrence-effects/classes.dex
cp .cache/fixtures/value-occurrence-effects/classes.dex quality/fixtures/atoms/value-occurrence-effects/classes.dex
```

Inspection used the same prepared DEX with f83f2fa and the accompanying candidate
through dbg_shell p9-all. Rust checks used rustc 1.98.0 and cargo-nextest 0.9.149.
The `value_occurrences` integration target runs 11 tests: f83f2fa passes 5 and
fails 6; the candidate passes all 11. These include recovery/origin checks and a
small linear integer executor, not general method behavioral acceptance. See review.md
for the path reasoning and its unexecuted scopes.
