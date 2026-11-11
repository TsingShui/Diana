# Review: direct-initialization-effects

LLM path-by-path review compares the authored Smali, Java helper explanations and
candidate output on the same prepared DEX. The baseline `b5eb94d` replaces build
with a recovery-failure body. The candidate retains the body below. Structural
integration assertions are supporting evidence; these are not executed ART or
decompiled-Java behavioral results.

## Semantics

| Path | Judgment and reasoning |
|---|---|
| Positive argument, no fault | Agreement: allocation precedes event1, Parent/event2, event4 and publication. The returned alias is the published object. Parent stores the input7; Child's value99 store is never introduced. Trace is124. |
| Negative argument, no fault | Agreement: the `i < 0` arm executes event3 between event1 and initialization, yielding trace1324 and stored value-7. The branch does not duplicate allocation or initialization. |
| Fault1 or fault3 | Agreement: the original handler captures the thrown RuntimeException, runs event5 and rethrows that same exception. Constructor, event4 and publication are skipped. Trace is15 or135. |
| Fault2 | Agreement under the helper body: Parent records event2 before throwing and before its value store. The captured exception reaches event5 and rethrow; no successful initialization or publication is invented. Trace is125. |
| Fault4 | Agreement: initialization has completed, then event4 throws before publication. Its handler runs event5 and rethrows. Trace is1245, last remains null. |
| Allocation/linkage Error | Agreement for handler selection: the original typed RuntimeException handler does not catch an Error. Allocation remains inside its original protected sequence and no later operation runs if it fails. Runtime acceptance of the optimized constructor form remains unverified. |
| Failure inside handler | Agreement: event5 is outside the protected body of that catch. An independent helper failure escapes rather than recursively entering the same catch or restoring the primary error. |

The pure reference return is printed inside the try instead of immediately after
its end. It has no cast, call or throwing conversion, so this placement adds no
exceptional transfer or observable work. The original alias copy is eliminated,
but allocation, direct initialization, publication and return all refer to the
same output variable. No second object or constructor is created. This challenge
therefore adds relevant effect, branch, alias and exception interactions to the
parent fixture without weakening its exact-target requirement.
