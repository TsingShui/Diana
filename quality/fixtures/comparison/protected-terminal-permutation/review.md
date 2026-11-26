# Conditional path review

I compared both Smali bodies and Java explanations with Diana's recovered classes and the ordinary diff output. On key zero the old conditional throws its first argument within [3, 7). The handler at 10 consumes the RuntimeException and returns null. A null argument raises NullPointerException at the same instruction and reaches that handler. The new throw at 9 lies outside the interval. The supplied exception escapes by identity, or a null argument raises an escaping NullPointerException. A nonzero key reaches return-object with the original argument in both versions, including when the argument is null.

The recovered old method encloses the conditional throw in a RuntimeException catch. The recovered new method shows the same conditional with an escaping throw. The shared string load precedes the interval. The Java models assume successful string resolution. The intervening constant and jump block is unreachable in both DEX inputs. No helper contributes further effects.

The prior comparison preserved the normal branch and terminal correspondence but declared the method unchanged. Its endpoint correspondence missed the instruction that crossed the boundary. The corrected report identifies the protected throw at old code unit 6 and the unprotected throw at new code unit 9.

The fixture CLI regression checked those two original sites. Four branch and argument combinations per version were also exercised on OpenJDK 21.0.12.1 against the authored Java and recovered Java separately. Native ART execution and VM resource failures were outside the executed checks.
