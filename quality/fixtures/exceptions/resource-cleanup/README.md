# Resource cleanup ownership

## Input and expectations

These Java/Smali sources are independently authored, with no third-party code.
Original owner: `Lfixture/ResourceCleanup;`. The three subjects are `loop(I)I`,
`nested(I)I` and `sharedHandler(I)I`. Java explains the behavior; Smali fixes the
actual instruction layout. Integer handles remove allocation dependencies while
retaining the cleanup and suppression topology of two resources and a resource
inside a loop.

`open(I)I`, `read(I)I` and `close(I)V` are native declarations in the input.
Each scenario starts with an empty trace and no live handles. Unless a scenario
specifies failure, `open(id)` returns `id`, `read(handle)` returns `10 * handle`,
and `close(handle)` returns normally. Each call records its argument before
returning or throwing. Acquired handles remain valid until their close attempt.
Failures use distinct, non-null throwable identities E, C and D. All other calls
return normally; `Throwable.addSuppressed` records its receiver/argument and
returns normally. No allocation failure or asynchronous exception is assumed.
Diana receives no external declarations or helper implementation.

Notation: `O`, `R`, `C` denote open/read/close; `S(E,C)` denotes
`E.addSuppressed(C)`. The fault column names the call that throws after recording
its argument. The expected exit includes the primary identity and ordered
suppressed identities. These expectations come from the authored control flow,
not the engine's products.

| Subject / arguments | Fault | Expected exit | Ordered calls |
|---|---|---|---|
| `loop(0)` | none | return 0 | none |
| `loop(2)` | none | return 10 | O(0), R(0), C(0), O(1), R(1), C(1) |
| `loop(2)` | O(0) throws E | throw E | O(0) |
| `loop(2)` | R(0) throws E | throw E | O(0), R(0), C(0) |
| `loop(2)` | R(0) throws E; C(0) throws C | throw E, suppressed [C] | O(0), R(0), C(0), S(E,C) |
| `loop(2)` | normal C(0) throws C | throw C | O(0), R(0), C(0) |
| `nested(5)` | none | return 35 | O(1), O(2), R(1), R(2), C(2), C(1) |
| `nested(5)` | O(1) throws E | throw E | O(1) |
| `nested(5)` | O(2) throws E | throw E | O(1), O(2), C(1) |
| `nested(5)` | R(1) throws E | throw E | O(1), O(2), R(1), C(2), C(1) |
| `nested(5)` | R(2) throws E; C(2) throws C; C(1) throws D | throw E, suppressed [C,D] | O(1), O(2), R(1), R(2), C(2), S(E,C), C(1), S(E,D) |
| `nested(5)` | normal C(2) throws C; C(1) throws D | throw C, suppressed [D] | O(1), O(2), R(1), R(2), C(2), C(1), S(C,D) |
| `nested(5)` | normal C(1) throws C | throw C | O(1), O(2), R(1), R(2), C(2), C(1) |
| `sharedHandler(1)` | none | return 0 | O(0), R(0), R(0), C(0) |
| `sharedHandler(1)` | second R(0) throws E; C(0) throws C | throw E, suppressed [C] | O(0), R(0), R(0), C(0), S(E,C) |

## Trigger and guard

`loop` has private cleanup handlers outside its natural backedge core. The loop's
lexical body needs the established try's complete membership before child
selection. The loop predicate and normal follow remain outside that try.

`nested` routes failures of second acquisition, normal second close, suppression
and inner rethrow to the same outer handler. The inner catch must not absorb that
foreign outer exception destination. Normal first close stays outside the outer
try, so its failure must not close the first resource again.

`sharedHandler` receives exceptions from both the loop and a read after it.
Its capture cannot become private loop content. The cleanup is not automatically
finally: a failed body close is suppressed, but a failed normal close propagates.

## Preparation and inspection

```sh
mkdir -p .cache/fixtures/resource-cleanup
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/resource-cleanup/classes.dex \
  quality/fixtures/exceptions/resource-cleanup
cp .cache/fixtures/resource-cleanup/classes.dex \
  quality/fixtures/exceptions/resource-cleanup/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/resource-cleanup/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. These versions record
the actual environment, not admission requirements. The prepared DEX reproduces
the explicit cleanup layout on `8e07305` through the ordinary Engine entry.
`crates/diana-decomp/tests/resource_cleanup.rs` contains four structural checks
through the ordinary Engine entry. Run them with
`cargo nextest run --locked -p diana-decomp --test resource_cleanup`.
On the unmodified `8e07305` planner, the loop-membership and nested-protection
checks fail; the shared-handler guard passes. With the P6 lexical-handler closure,
foreign-handler frontier and guarded-successor boundary changes, the four checks
pass, including propagation-only sites remaining outside protected bodies. The
same prepared DEX is used before and after. These failures reproduce ownership
and shape gaps, not known behavioral mismatches. Behavioral execution against
this scenario table is unimplemented; structural observations are not EVOLUTION
behavioral acceptance or an admitted semantic regression case.
