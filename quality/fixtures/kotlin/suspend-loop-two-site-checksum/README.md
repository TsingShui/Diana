# Two-site loop with checksum and effects

This project-authored complexity challenge follows
[suspend-loop-two-sites](../suspend-loop-two-sites/README.md) and retains both
interior resume entries and the between-site early exit. It adds a mutable
checksum saved across both sites, a pre-second-load record and a post-update
record. Sources use the repository license. Java/Kotlin explain the logical
body; Smali fixes original state, boxing and marker-return behavior.

Subject: `Lfixture/SuspendChecksum;->total(IILkotlin/coroutines/Continuation;)Ljava/lang/Object;`.
I$0/I$1/I$2 are limit/sum/index; I$3 is checksum; I$4 preserves first at site two.
The callback passes zero initial limit/bias, while both resumed paths restore the
actual values. This input was authored after the two-site loop repair and before
candidate output inspection.

## Independent model and scenarios

load(a) records its argument and yields boxed 2*a immediately or after suspension;
record(v) records its value. Selected calls can throw, or the third load can be
modeled to return a negative value. Valid frames and successful field/type
resolution are assumptions. Helpers/scheduler are not executed.

| Scenario | Independent result and effect order |
|---|---|
| limit 0 or -2, bias 7 | Return 7; no helper work. |
| limit 1, bias 5 | load(0), record(5), load(1), record(2); return 7. |
| limit 2, bias 5, any immediate/resumed combination | load(0), record(5), load(1), record(2), load(1), record(5), load(3), record(10); return 16. |
| limit 3, bias 5, alternating sites suspend | Previous sequence, then load(2), record(6), load(5), record(24); return 32. |
| Third load (argument 1) yields -3, limit 3, bias 5 | First iteration completes with sum 2/checksum 5. Then load(1), record(5) exits; return 7. No second call or checksum/index update for that iteration. |
| First-site failure | Preserve earlier completed iteration effects; skip that iteration's record and later work. |
| Second-site failure | Pre-second-load record remains; sum/checksum/post-record/index do not run. |
| Post-update record fails | Sum and checksum arithmetic precede failure; index and next first load are skipped. |
| Physical marker return | Defer the corresponding tail and its effects until matching callback completion. |
| Invalid adjusted label | Original invalid-state error remains before source work. |

Head-false transfer copies occur only on the original predicate exit; the early
negative route skips them and reaches its own saved sum/checksum. The while must
preserve each source update and both resume-copy interfaces without repeating
or fabricating captured values. A written review and structural checks are
supporting evidence, not a final-plan behavior executor.

```sh
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/kotlin/suspend-loop-two-site-checksum \
  -o .cache/fixtures/suspend-loop-two-site-checksum/classes.dex
cp .cache/fixtures/suspend-loop-two-site-checksum/classes.dex \
  quality/fixtures/kotlin/suspend-loop-two-site-checksum/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/kotlin/suspend-loop-two-site-checksum/classes.dex \
  --exec '{"cmd":"classes"}' --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared on 2026-11-09 with the locally available Smali 2.5.2 assembler,
`--api 24` (DEX 037), on Temurin OpenJDK 21.0.12.1+1. Reassembly from the
committed Smali reproduces the prepared DEX byte for byte. The baseline run used
`b14cd40`; the candidate is the coroutine-cycle body change.

The baseline renders `public static Object total(...)`, passes the frame to
`load(index, frame)`, and keeps the label `switch` plus its default/case arms in
a `graph<Irreducible>`. Its loop head prints as
`L41: if (v35 >= v39) { ... goto L72; }` with an explicit `goto L41` latch.
The candidate renders `public static suspend Object total(...)`, omits the
inferred continuation at both calls, and prints one
`while (v35 < v39 [exit: ...])` whose body ends with the original latch
fallthrough. The negative route keeps an explicit `goto L72` to the shared
tail, which still returns `sum + checksum`.

Across the 108 prepared fixtures, only `suspend-loop-two-sites` and this input
change; the other text/JSON products are identical after removing timing fields,
with no lost members, stopped methods or render issues. The written review and
the fixture test are supporting evidence, not a final-plan behavior executor.
