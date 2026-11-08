# Typed catch priority with a shared cleanup

## Input and expectations

The Java explanation and Smali input are independently authored for this
repository; they contain no third-party code or data. Original owner:
`Lfixture/TypedCatchSharedCleanup;`. The subject is
`Lfixture/TypedCatchSharedCleanup;->classify(Ljava/lang/String;I)I`.

`open(I)I`, `read(I)I`, `close(I)V` and `parse(Ljava/lang/String;)I` are native
helpers. The first `try` has a multi-catch for `NumberFormatException` and
`NullPointerException`, a broader `RuntimeException` catch, and a finally that
closes the first resource. The multi-catch handler acquires a second resource and
uses a nested try/finally to close it. The nested try's close failure is not
suppressed: an explicit finally replaces the pending exception on that path.

Each scenario starts with an empty trace. Unless a scenario states a fault,
`open(0) = 50`, `open(1) = 51`, `read(51) = 11`, `parse("x") = 7`, and every
close returns normally. A scenario may make a selected helper call throw a
distinct, non-null throwable: `NFE`, `NPE`, `ISE` or `AE` are `RuntimeException`
subclasses, `E` is a generic selected helper failure and `C`, `D` are close
failures. The mode argument constructs a throwable before `parse` is reached:
mode 1 constructs `NFE("mode")`, mode 2 constructs `NPE("mode")`, and mode 3
constructs `ISE("mode")`. No asynchronous exception or allocation failure is
assumed.

Notation: `O`, `P`, `R`, `C` denote open, parse, read and close. Catch priority is
first `NumberFormatException`, then `NullPointerException`, then
`RuntimeException`; the final catch-all belongs to the outer finally. On a
multi-catch path, `result = read(51) + 100`; on the broad path, `result = -2`.

| Subject and arguments | Fault | Expected exit | Ordered calls and effects |
|---|---|---|---|
| `classify("x", 0)` | none | return 7 | O(0), P("x"), C(50) |
| `classify("x", 1)` | constructed NFE | return 111 | O(0), O(1), R(51), C(51), C(50) |
| `classify("x", 2)` | constructed NPE | return 111 | O(0), O(1), R(51), C(51), C(50) |
| `classify("x", 3)` | constructed ISE | return -2 | O(0), C(50) |
| `classify("x", 0)` | P throws NFE | return 111 | O(0), P("x"), O(1), R(51), C(51), C(50) |
| `classify("x", 0)` | P throws NPE | return 111 | O(0), P("x"), O(1), R(51), C(51), C(50) |
| `classify("x", 0)` | P throws AE | return -2 | O(0), P("x"), C(50) |
| `classify("x", 0)` | P throws Error | throw Error | O(0), P("x"), C(50) |
| `classify("x", 1)` | O(1) throws E | throw E | O(0), O(1), C(50) |
| `classify("x", 1)` | R(51) throws E; C(51) throws C | throw C | O(0), O(1), R(51), C(51), C(50) |
| `classify("x", 1)` | R(51) throws E | throw E | O(0), O(1), R(51), C(51), C(50) |
| `classify("x", 1)` | C(51) throws C | throw C | O(0), O(1), R(51), C(51), C(50) |
| `classify("x", 0)` | C(50) throws C | throw C | O(0), P("x"), C(50) |

Close arguments use returned handles, not acquisition IDs: open(0) returns 50
and open(1) returns 51. The initial table mixed those two namespaces; the corrected
calls above follow the authored helper model and Java/Smali data flow.

The NFE/NPE scenarios are the catch-priority check: both are subclasses of
`RuntimeException`, so a broad-catch-first selection would produce -2 instead of
111. The `Error` scenario bypasses both typed catches and enters the outer finally.
On the multi-catch path the nested finally close runs before the outer close. A
failing detail close replaces the read failure; a failing outer close then
replaces that result or exception.

## Trigger structure

The protected inner body has one try item with clauses ordered
`NumberFormatException`, `NullPointerException`, `RuntimeException`, then a
catch-all to the outer finally. The multi-catch clause is two typed clauses with
the same target. The handler region is split around the nested detail body: the
detail body has its own catch-all close handler, while the detail rethrow, the
detail close and the broad handler are covered by the outer catch-all. The shared
outer `close(res)` is outside every protected range, so a normal first-close failure
propagates without re-entering the resource handlers.

The prepared DEX is assembled from `TypedCatchSharedCleanup.smali`. The p6
product reports one natural try item for the nested read close plus explicit
protection for the typed body; its chains preserve the four typed routes and the
two catch-all routes.

## Preparation and inspection

```sh
mkdir -p .cache/fixtures/typed-catch-shared-cleanup
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/typed-catch-shared-cleanup/classes.dex \
  quality/fixtures/exceptions/typed-catch-shared-cleanup
cp .cache/fixtures/typed-catch-shared-cleanup/classes.dex \
  quality/fixtures/exceptions/typed-catch-shared-cleanup/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/typed-catch-shared-cleanup/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. The current
confirmation used repository HEAD `6d663d9`; engine sources are unchanged from
`12de370`. `review.md` records the reading judgment.

## Review status

The review is a manual source/output reading judgment. No automated behavioral
executor, Java recompilation or fixture test was run. The native `parse` and
resource helpers are modeled by the scenario table; no helper body in the DEX
decides the expected result.
