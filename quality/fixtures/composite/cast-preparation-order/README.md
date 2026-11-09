# Cast and argument preparation order

Independently authored Java/Smali under the repository Apache-2.0 license.
Original subject: `Lfixture/CarrierPreparation;->ordered(Ljava/lang/Object;I)I`.
The engine consumes the prepared DEX alone. The input extends the
[cast carrier contexts](../cast-carrier-contexts/README.md) with a throwing
preparation before a nonredundant check inside the later short-circuit run.
Baseline: `33b8abd`. Expectations were authored before inspection.

A nonzero flag returns 1 without string resolution, cast or call. On zero flag,
const-string resolves x first, the receiver is checked as int[] second, and
Object.equals receives the checked receiver and that exact resolved string last.
The explicit array-helper model records E(receiver,string), returns false, and
throws NullPointerException for a null receiver. String resolution records S and
may throw a distinct resolution fault R. The int[] cast permits null, preserves
an array's identity, and throws ClassCastException for an ordinary object O.
This is an explicit test/review model, not a VM or external declaration input.

Normal array/zero-flag result is 0 with S,E(A,x). Null/zero-flag throws NPE after
S,E(null,x). O/zero-flag throws CCE after S and before E. A resolution fault throws
R after S before any cast or E, including when the receiver is O. Nonzero flag
makes every fault unreachable and has no effects. Reversing string resolution
and cast changes that competing-fault case. No check may be treated as transparent
merely because it writes no register.

```sh
mkdir -p .cache/fixtures/cast-preparation-order
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/cast-preparation-order/classes.dex quality/fixtures/composite/cast-preparation-order
cp .cache/fixtures/cast-preparation-order/classes.dex quality/fixtures/composite/cast-preparation-order/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/composite/cast-preparation-order/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

The added companion castFirst(Ljava/lang/Object;I)I places the cast before string
resolution. Nonzero flag still skips everything. On zero flag with O, CCE skips
S even when resolution fault R is selected. With A, normal result is 0 after
S,E(A,x); resolution fault R follows S and prevents E. With null, S precedes the
modeled E(null,x) and NPE. This order is eligible for one operand forest.

The added produced()I subject calls produce, checks its reference, then calls
consume with that exact checked value. Produce records P and returns the modeled
reference; consume records C(value) and returns 7. A producer fault FP records P
and skips check/C. An incompatible returned O records P then throws CCE before C.
A/null normal return 7 with P,C(value); a consumer fault FC follows P,C(value).
No helper or VM is actually executed. Expectations precede the added DEX run.

## Observed preparation and checks

Prepared on 2026-11-10 with Smali 2.5.2, API 24 and Temurin OpenJDK
21.0.12.1+1. Native selectors: produce()Ljava/lang/Object; and
consume(Ljava/lang/Object;)I. Baseline 33b8abd places the string after the cast
in ordered; the containing candidate preserves its earlier source position.
CastFirst still has compact recovery; produced has one checked producer forest.
All five original members remain available, with three executable subjects.

The baseline fault-order sensitivity test fails on O because its cast skips the
required S event. Candidate's two new cases execute 29 independent scenarios.
The final scoped run has thirteen passes across the three new test targets;
engine library/example/three-target Clippy with all features and the dev
inspection build pass. The cached baseline uses the same locked dependency
versions. No historical fixture suite, release duplicate, external declarations
or actual helper/VM execution was run.
