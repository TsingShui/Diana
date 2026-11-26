# R8 inlines a helper into a caller with a recycled name

This compiler-produced challenge extends the [assembled inline case](../handler-inline-challenge/) with obfuscation, register changes and a collision between an old helper's full signature and the new caller's signature. Both original Java inputs and ProGuard-format R8 rules were authored for this comparison under Apache-2.0. Java and the R8 configurations are the assembly inputs for this fixture.

## Independent expectations

| Role | Before | After |
| --- | --- | --- |
| Entry | `La/a;->c(II)I` (original run) | `La/a;->a(II)I` (original dispatch) |
| Helper | `La/a;->a(II)I` (original calc) | step inlined into dispatch |
| Marker | `La/a;->b()Ljava/lang/String;` | `La/a;->b()Ljava/lang/String;` |

The old entry rotates its first parameter left by three, calls calc at code-unit offset 5 with the rotation and divisor, and rotates the returned integer right by seven. The new entry evaluates the same calculation in place. At (0, 1) both entries return 0xc6000000. At (0x10000000, -1) the left rotation produces 0x80000000, Integer.reverse gives 1, the quotient is -1, Math.abs gives 1, and both entries rotate 100 to 0xc8000000. A zero divisor throws ArithmeticException in both versions before Math.abs, addition or the final rotation. The marker returns the literal `same authored class` in both.

The old helper's six instructions before its return have widths of 3, 1, 1, 3, 1 and 2 code units. Their sequence is Integer.reverse, move-result, integer division, Math.abs, move-result and addition of 99. The block occupies [0,11) in the old helper and [5,16) in the updated entry. The helper uses v0 and v1, the inline body uses v1 and v2. The left rotation at offset 1 in both builds and the right rotation at old offset 10 and new offset 17 delimit the corresponding entry. A same-signature whole-method pair would misattribute both rotations to the old helper.

## Compiler and reproduction

Compilation used the OpenJDK 21.0.12.1 compiler module with Java 8 source and target plus line and source information. Both inputs used R8 8.6.2-dev from Android SDK Build Tools 35.0.0 in release mode, minimum API 24, with the API 35 android.jar as R8's build library. Shrinking, optimization and obfuscation were enabled in both builds. The old rules separately retain the public calc method; the new rules allow the package-private step method to be inlined. The original compiler mapping for the new input identifies step inside dispatch. That observation supplied an independent compiler witness; Diana receives only the two prepared DEX files.

Set R8_JAR and ANDROID_JAR to the installed R8 JAR and the build library. From the repository root, build the inputs with the available compiler and keep scratch outputs under the fixture cache. The recorded offsets refer to the checked-in DEX inputs.

```sh
set -e
slug=optimized-single-inline
for side in before after; do
  src="quality/fixtures/comparison/$slug/$side"
  cache=".cache/fixtures/$slug/$side"
  mkdir -p "$cache"
  run=$(mktemp -d "$cache/run.XXXXXX")
  mkdir "$run/classes" "$run/dex"
  java -m jdk.compiler/com.sun.tools.javac.Main -source 8 -target 8 \
    -Xlint:-options -g:lines,source -d "$run/classes" "$src"/*.java
  (cd "$run/classes" && python3 -m zipfile -c ../input.jar bench)
  java -cp "$R8_JAR" com.android.tools.r8.R8 --release --min-api 24 \
    --thread-count 2 --lib "$ANDROID_JAR" --pg-conf "$src/rules.pro" \
    --pg-map-output "$run/mapping.txt" --output "$run/dex" "$run/input.jar"
  cp "$run/dex/classes.dex" "$src/classes.dex"
done
diana diff quality/fixtures/comparison/optimized-single-inline/before/classes.dex \
  quality/fixtures/comparison/optimized-single-inline/after/classes.dex
cargo nextest run --locked -p diana --test diff_test -E 'test(compiler_inline)'
```

The comparison candidate on base 7be3f1e initially paired old a with new a, left old c unmatched, and omitted the inline fragment. Its report placed both rotations among the changes attributed to the helper. The [semantic review](review.md) describes the ordinary get-class output and the original operations.

The revised comparison on base 7be3f1e passed the focused regression on the same compiled inputs. It matched old c to new a and reported six instructions from old a at offset 0 inside new a at offset 5. The marker stayed unchanged. A complete run of `cargo nextest run --locked -p diana --test diff_test` passed all eleven tests, including the hand-assembled inline case and the independent R8 correspondence case.
