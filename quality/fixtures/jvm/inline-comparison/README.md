# JVM inline comparison

`before/Expansion.java` contains two private static helpers that disappear in
`after/Expansion.java`. Their callers keep independent calls before and after
both expansions. `run()` takes no helper arguments. `forwarded(int, long)`
passes its own parameters, which occupy different local slots because the
caller has a receiver and the helper does not.

The diff reports the original helper at byte 0 and its expansion at byte 3 of
the new caller in both cases. Each expansion contains seven operations. The
old and new instructions use JVM byte offsets in both text and JSON.

`changedArguments()` swaps two values of the same type. `protectedCall()`
retains an exception handler and exercises its normal and exceptional paths.
`unrelated()` copies a helper's operation sequence into a method that never
called that helper. These three changes do not establish inline fragments.
`Verify.java` specifies expected return values and effect traces independently
from the diff.

Prepare and execute the fixture from the repository root with OpenJDK 21.0.12.1
and Python 3. The recipe compiles Java 17 class files and writes its products
under `.cache`.

```sh
python3 quality/fixtures/jvm/inline-comparison/prepare.py .cache/fixtures/inline-comparison
java -cp .cache/fixtures/inline-comparison/before Verify before
java -cp .cache/fixtures/inline-comparison/after Verify after
cargo nextest run --locked -p diana --test jvm_inline_diff_test
```
