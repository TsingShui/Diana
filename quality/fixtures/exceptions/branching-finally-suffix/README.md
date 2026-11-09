# Branching finally over a protected loop and typed catches

## Independent input and expectations

Authored Java/Smali; no third-party input. Discovery:
`../typed-catch-shared-cleanup/`. The candidate first recovered its outer
catch-all suffix as finally, retaining incoming body edge copies and comparing
producer-proved value-preserving reads rather than declaration-name identities.
This challenge adds a protected loop, catch rejoin and a two-arm cleanup graph
with a shared end. It also rejects superficially equal calls with unequal arguments.

Subjects: `Lfixture/BranchFinally;->run(III)I`,
`Lfixture/BranchFinally;->different(I)I` and
`Lfixture/BranchFinally;->guardedRethrow(II)I`. Helper read/mark/finish bodies are in
this same DEX. trace starts at zero and mark uses wrapping int arithmetic.
Standard constructors succeed; no linkage, asynchronous, allocation or concurrent
trace-writer faults are modeled. Java's implicit constructor is not a Smali subject.

run reads row+1 unless its mode throws: 1=NFE, 2=NPE, 3=ISE("row"),
4/6=AssertionError("row"). NFE/NPE mark 20 and select 100, then the loop adds and
increments. flag=0 cleanup marks 40; any other flag marks 41 then calls
finish(rows,mode). finish marks 30+rows, then modes 5/6 throw IAE("close").
It runs after the last protected action, never per iteration.

| Subject | Arguments | Expected exit | Ordered mark events |
|---|---|---|---|
| run | (3,0,0) | return 6 | 10,11,12,40 |
| run | (3,0,1) | return 6 | 10,11,12,41,33 |
| run | (2,1,1) | return 200 | 10,20,11,20,41,32 |
| run | (2,2,0) | return 200 | 10,20,11,20,40 |
| run | (3,3,0) | throw original ISE | 10,40 |
| run | (3,4,1) | throw original Error | 10,41,33 |
| run | (3,5,1) | throw IAE, replacing normal completion | 10,11,12,41,33 |
| run | (3,6,1) | throw IAE, replacing Error | 10,41,33 |
| run | (3,6,0) | throw original Error; no finish | 10,40 |
| run | (0,6,0) | return 0; no body error | 40 |
| run | (0,6,1) | throw IAE with no prior Error | 41,30 |
| different | (0) | return 1 | 10,31 |
| different | (1) | throw original NFE | 10,32 |
| different | (5) | throw IAE after successful read | 10,31 |
| different | (6) | throw IAE, replacing Error | 10,32 |
| guardedRethrow | (0,0) | return 1 | 109,10,0 |
| guardedRethrow | (1,0) or (6,0) | return 300 after the pending rethrow is caught | 109,10,0 |
| guardedRethrow | (0,3) | return 300 from the prefix catch, without cleanup | 109 |

different passes code=1 on its normal path and code=2 on its exceptional path.
The two finish calls must not become a single unconditional finally call.
Its literal constants precede the body; matching must compare argument values,
not merely operation kinds or a value-preserving view category.

guardedRethrow is a later safety challenge. Its prefix read(99,prefixMode) and
its inner handler's rethrow share one outside capture returning 300. The two
mark(0) cleanup calls are identical and unguarded, but a pending inner exception
must still take the original outside route. An implicit unhandled rethrow would
change (1,0) from return 300 to a thrown NFE. The Java pair expands that shared
outside body into two catches. These additional expectations were recorded
before inspecting either revision on this method.

Expectations are recorded before candidate inspection. There is no scenario
executor or Java-recompilation acceptance claim.

## Preparation

```sh
mkdir -p .cache/fixtures/branching-finally-suffix
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/branching-finally-suffix/classes.dex \
  quality/fixtures/exceptions/branching-finally-suffix
cp .cache/fixtures/branching-finally-suffix/classes.dex \
  quality/fixtures/exceptions/branching-finally-suffix/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/branching-finally-suffix/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Assembled with Smali 2.5.2, API 24 and OpenJDK 21.0.12.1. Baseline: working tree
on `ebbc003` after catch/loop repairs but before this finally repair, using the
same prepared DEX. The pending-rethrow defect is also present on that baseline;
the final candidate preserves its outside route. See review.md for the direct
semantic comparison and unrun behavioral-execution limitation.
