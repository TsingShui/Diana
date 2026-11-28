# JVM static initializer evaluation order

These independently authored classes and interfaces use ordinary Javac on
JVM 17. Each case observes a local that has already captured an effectful
value when the original initializer uses it in a static field write. The
verification runs each initialized class in a fresh JVM.

| Original method in the prepared JAR | Post-capture use |
| --- | --- |
| `LEarlierOperand;-><clinit>()V` (`j0:m1`) | Second argument after an effectful first argument |
| `LConditionalOperand;-><clinit>()V` (`j1:m0`) | One arm of an interface initializer conditional |
| `LShortOperand;-><clinit>()V` (`j2:m1`) | Right operand of a Boolean short circuit |
| `LProtectedOperand;-><clinit>()V` (`j3:m1`) | Second argument before publication and a later typed catch |
| `LFirstOperand;-><clinit>()V` (`j4:m1`) | First argument, before the later producer |

`ScheduleEffects` records the capture as digit 1. Evaluating the other
argument or predicate appends 2. The argument-consuming call or false-arm
fallback appends 3. The attempt to observe the published reference appends 4.
The typed catch appends 5 only when it receives the exact configured
post-publication exception. The two earlier producers can throw different
exception instances of that same type. The observer inspects the original
cause wrapped by JVM class initialization.

| Scenario | Original result and trace |
| --- | --- |
| Earlier operand normal | captured / 123 |
| Earlier operand with competing faults | capture fault / 1 |
| Conditional false and true | other / 123 and captured / 12 |
| Capture fault before conditional false | capture fault / 1 |
| Boolean false and true | false / 12 and true / 12 |
| Capture fault before Boolean false | capture fault / 1 |
| Protected normal and typed catch | captured / 1234 and captured / 12345 |
| Earlier fault escapes or both pre-try producers can fail | earlier fault / 12 and capture fault / 1 |
| First operand normal and competing faults | captured / 123 and capture fault / 1 |

The verifier checks original invocation order with `javap`. In the
protected initializer the result write at byte 11 precedes the exception
range at bytes 14 through 19. The handler records the exact exception
from the later operation. The verifier checks that the safe first-argument
case stays a compact field initializer with both calls in their original
order. It also accepts explicit `Object` casts that retain the original
argument types when the external declarations are unavailable.
Each class publication is recompiled separately and run with the unchanged
observer and support class.

```sh
fixture=quality/fixtures/jvm/static-initializer-schedule
work=.cache/fixtures/static-initializer-schedule
python3 -B "$fixture/verify.py" --binary target/debug/diana --output "$work/proof.json"
python3 -B quality/decbench prepare --prepared "$work/prepared"
python3 -B quality/decbench run --prepared "$work/prepared" \
  --binary target/debug/diana --suite jvm-static-initializer-schedule \
  --output "$work/decbench.json"
```

The original passes all 14 runtime scenarios. Frozen Diana 0.4.26
fails 12 of them. Its first-operand control passes both runtime scenarios
and keeps the field initializer compact. The default DecBench suite gives
18 passing checks and 12 failing checks.

The 0.4.27 candidate passes all 15 standalone checks and all 30 DecBench
checks on the same prepared JAR. The comparison gate records 12 resolved
failures without losing coverage.
