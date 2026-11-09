# Guarded entry frontiers and optional prefixes

## Independent expectations

Authored Java/Smali, no third-party input. Seed: CaseEntryJoins.route has a valid
single-entry typed guarded core, but an optional nonthrowing goto predecessor
created another body entrance and prevented the producer from delivering that
inner TryPlan. The initial repair retains an established single-entry prefix
checkpoint while growing the same predecessor closure once. It does not change
clauses, priority, outside rethrow delivery, handler ownership or guarded seeds.

The fresh branch subject has TWO guarded read sites. Their base body needs a
common nonthrowing flag prefix to become single-entry; still-earlier goto
prefixes have independent throwing predecessors and must stay outside. This is
more than retaining an unexpanded single-read body. Split is a genuine negative:
its two guarded reads have separate throwing prefixes with only outer cleanup
protection. No single closed prefix may absorb those calls or discard a seed to
pretend that one shared typed handler is privately owned.

Owner: `Lfixture/PrefixFrontiers;`. Subjects `branch(III)I`, `split(II)I`; concrete
mark/read/cleanup helpers are in the DEX. Java explains split with two catches;
Smali retains one original capture/body. Java's implicit constructor is not a
Smali subject. Trace starts at zero; arithmetic wraps as int. Standard exception
construction succeeds. No asynchronous/linkage/allocation fault or concurrent
trace writer is modeled.

branch marks1/2/3 according to lead0/1/other. Its flag0 reads row0, other flags
read row1. NFE/NPE mark20 and select100; ISE/Error bypass that typed handler.
Normal result selection precedes mark40, then cleanup(row2) marks32, then mark50.
Split uses prefix mark60/61 and cleanup(row3) marks33. Prefix calls have only outer
cleanup protection in the DEX; the typed try cannot acquire their exceptions.

| Subject and arguments | Expected exit | Ordered mark events |
|---|---|---|
| branch(0,0,0) | return 1 | 1,10,40,32,50 |
| branch(1,1,0) | return 2 | 2,11,40,32,50 |
| branch(9,0,1) | return 100 | 3,10,20,40,32,50 |
| branch(0,1,2) | return 100 | 1,11,20,40,32,50 |
| branch(0,0,3) | throw original ISE("row") | 1,10,32; no mark40/50 |
| branch(0,0,4) | throw original Error | 1,10,32 |
| branch(0,0,5) | throw normal-cleanup IAE | 1,10,40,32; no mark50 |
| branch(0,0,6) | throw IAE replacing Error | 1,10,32; no suppression/retry |
| split(0,0) | return 1 | 60,10,40,33,50 |
| split(1,0) | return 2 | 61,11,40,33,50 |
| split(0,1) | return 100 | 60,10,20,40,33,50 |
| split(1,2) | return 100 | 61,11,20,40,33,50 |
| split(0,3) | throw original ISE("row") | 60,10,33 |
| split(1,5) | throw normal-cleanup IAE | 61,11,40,33 |
| split(0,6) | throw IAE replacing Error | 60,10,33 |

Prefix selection may omit optional predecessors only at an already recorded
complete frontier. Every guarded seed and all original forward content stay in
the body; no path is clipped or handler duplicated. Unknown/multiple entry or
later ownership/guard/priority refusal remains local explicit code, not a retry
with another prefix, boundary or method analysis. Normal LT is unchanged.

Expectations precede first baseline/candidate inspection. No behavioral execution
result is claimed.

## Preparation

```sh
mkdir -p .cache/fixtures/protected-prefix-frontiers
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/protected-prefix-frontiers/classes.dex \
  quality/fixtures/exceptions/protected-prefix-frontiers
cp .cache/fixtures/protected-prefix-frontiers/classes.dex \
  quality/fixtures/exceptions/protected-prefix-frontiers/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/protected-prefix-frontiers/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and OpenJDK 21.0.12.1. Baseline is the recorded
working tree based on `ebbc003` after case-entry DAG recovery and before entry-
aware protected prefix completion. Both comparisons consume the same prepared
DEX. Branch retains a common flag-prefix entry for both reads and both nested
source owners; split keeps its two-entry explicit recovery graph. See review.md
for the direct reading judgment and unrun behavioral execution.
