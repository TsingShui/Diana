# Three resources with a late acquisition failure

## Input and expectations

The Java explanation and Smali input are independently authored for this
repository; they contain no third-party code or data. Original owner:
`Lfixture/ThreeResourceLateFailure;`. The subject is
`Lfixture/ThreeResourceLateFailure;->openThree(I)I`.

`open(I)I`, `read(I)I` and `close(I)V` are native declarations in the input. The
integer handles replace a resource wrapper that would delegate `close()` to the
same call; Java explains the nested try-with-resources behavior while Smali fixes
the actual instruction layout. Diana receives no declarations or helper bodies.

Each scenario starts with an empty trace and no live handles. Every helper call
records its argument before it returns or throws. Unless a scenario states a
fault, `open(id)` returns `id`, `read(handle)` returns `10 * handle`, and
`close(handle)` returns normally for every attempt. A scenario may make any
selected `open`, `read` or `close` call throw. Fault throwables are distinct and
non-null: `E` for the selected primary call, and `C0`, `C1`, `C2` for failures of
`close(0)`, `close(1)`, `close(2)`. `Throwable.addSuppressed` records its
receiver and argument and returns normally. No asynchronous exception,
allocation failure or concurrent helper call is assumed.

Notation: `O`, `R`, `C` denote open, read and close; `S(E,C)` denotes
`E.addSuppressed(C)`. The fault column names the call that throws after recording
its argument. Suppressed identities are listed in invocation order.

| Subject and arguments | Fault | Expected exit | Ordered calls and effects |
|---|---|---|---|
| `openThree(0)` | none | return 30 | O(0), O(1), O(2), R(0), R(1), R(2), C(2), C(1), C(0) |
| `openThree(0)` | O(0) throws E | throw E | O(0) |
| `openThree(0)` | O(1) throws E | throw E | O(0), O(1), C(0) |
| `openThree(0)` | O(2) throws E | throw E | O(0), O(1), O(2), C(1), C(0) |
| `openThree(0)` | O(2) throws E; C(1) throws C1; C(0) throws C0 | throw E, suppressed [C1, C0] | O(0), O(1), O(2), C(1), S(E,C1), C(0), S(E,C0) |
| `openThree(0)` | R(2) throws E; C(2) throws C2; C(1) throws C1; C(0) throws C0 | throw E, suppressed [C2, C1, C0] | O(0), O(1), O(2), R(0), R(1), R(2), C(2), S(E,C2), C(1), S(E,C1), C(0), S(E,C0) |
| `openThree(0)` | C(2) throws C2; C(1) throws C1; C(0) throws C0 | throw C2, suppressed [C1, C0] | O(0), O(1), O(2), R(0), R(1), R(2), C(2), C(1), S(C2,C1), C(0), S(C2,C0) |
| `openThree(0)` | C(0) throws C0 | throw C0 | O(0), O(1), O(2), R(0), R(1), R(2), C(2), C(1), C(0) |

The first scenario's result is `0 + 10 + 20`. A body failure skips the remaining
reads and the normal close, then closes all three resources from the inside out.
A failure during the third acquisition closes only the second and first
resources. A normal close failure of a later resource is handled by the enclosing
resource and suppresses the following close failures on that same exception.

## Trigger structure

`openThree` has three nested try-with-resources protocols. The Smali labels and
ranges establish the following structure:

- The inner protected body `{:body .. :body_end}` routes to `:inner_handler`,
  which closes handle 2 and carries its own suppression handler.
- The middle protection covers the third acquisition, the body, the normal close
  of handle 2, and the inner handler's suppression and rethrow. It closes handle 1.
- The outer protection covers the second acquisition, the middle rethrow, the
  normal close of handle 1, and the middle suppression handler. It closes handle 0.
- The normal close of handle 0 is outside every handler.

The nesting makes the third acquisition fail before handle 2 exists and makes a
normal close failure of handle 2 enter the middle handler. The prepared DEX is
assembled from `ThreeResourceLateFailure.smali`; the p6 product reports six try
items and six handler chains besides the uncaught chain.

## Preparation and inspection

```sh
mkdir -p .cache/fixtures/three-resources-late-failure
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/three-resources-late-failure/classes.dex \
  quality/fixtures/exceptions/three-resources-late-failure
cp .cache/fixtures/three-resources-late-failure/classes.dex \
  quality/fixtures/exceptions/three-resources-late-failure/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/three-resources-late-failure/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. These versions record
the actual preparation environment, not admission requirements. The current
confirmation used repository HEAD `6d663d9`; the engine sources are unchanged from
`12de370`. The inspection and reading judgment are recorded in `review.md`.

## Review status

The review is a manual source/output reading judgment. No automated behavioral
executor, Java recompilation or fixture test was run. The native helper calls are
modeled by the scenario table above; helper throwables are selectable scenarios,
not faults hard-coded in the input.
