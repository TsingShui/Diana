# Review: direct-initialization

Compared the same prepared DEX on pre-fix `b5eb94d` and the recovery-fixes
candidate. This is LLM semantic reading of the source, helper bodies and output;
the integration assertions support it. No ART execution or Java compilation of
the decompiled result was performed.

## Semantics

| Subject | Judgment and reasoning |
|---|---|
| `direct(I)Lfixture/Child;` | Agreement on retained operations. The output allocates one Child, invokes exactly `Lfixture/Parent;-><init>(I)V` on that reference, then returns it. Parent stores the argument. Child's separate constructor would store 99; it is not invoked by the output. Allocation failure skips initialization, and initialization failure skips the return. |
| `ordinary(I)Lfixture/Child;` | Agreement. Matching allocation and constructor owners retain `new Child(argument)`. Its Parent call and subsequent value99 store belong to Child's original constructor. This method is byte-for-byte unchanged from the baseline output. |
| `broken()Lfixture/Child;` | Correct local failure for this negative input: the original reference is returned without any constructor. The candidate reports the missing constructor in Diana warning comments with the issue URL. It introduces no executable throw or return. The CLI tests verify nonzero exit, stderr, structured method identity, text/JSON parity and retained sibling output. |

The baseline suppressed `direct` along with the intentional `broken` failure.
The candidate recovers `direct` and retains only the independent negative case.
The explicit notation makes no claim that ART accepts every optimized DEX form.
