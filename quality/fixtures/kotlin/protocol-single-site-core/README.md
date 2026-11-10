# One protocol matcher for a single-site branching body

Project-authored Apache-2.0 challenge derived from
[protocol-owned-reads](../protocol-owned-reads/README.md) and
[protocol-common-controls](../protocol-common-controls/README.md).
Baseline: `429998c`. One ordinary continuation method has a single state/reentry,
a saved seed, effectful argument preparation and an observed constructor behind
a negative guard. The shared protocol relation must cover this acyclic body as
well as single-site loops; site count does not establish a distinct language.

Original selector: Lfixture/OneSite;,
choose(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;.
Java explains the logical normal run with its established frame supplied;
Smali fixes original state dispatch, seed restoration, marker exit and invalid
state construction. All data facts come from that input.

## Independent scenarios

Prepare records P(seed) and returns seed + 7. Load records L(argument), then
returns the scenario Number, the distinct suspended marker, or a fault. Record
records R(n). ReadBox allocation records A and its constructor stores first then
second after Object super construction. Observe records O(first,second).
Boxing returns the corresponding Integer. Class initialization, frame/super
construction and marker acquisition are assumed successful; native helper models
are not executed runtime implementations. Arithmetic wraps as Java int.

| Path | Required observation |
|---|---|
| choose(3), reply 5 | Integer(8); P(3), L(10), R(5), A, O(5,3). |
| choose(3), reply -2 | Integer(-1); P(3), L(10), R(-2); no ReadBox/Observe. |
| Initial Load fault | P(3), L(10), fault; no Record, construction or return. |
| Initial Load suspension | P(3), L(10), marker return; no shared-tail work yet. |
| Resume state 1, seed 3, reply 5 | Restore seed and check result; R(5), A, O(5,3), Integer(8); no Prepare/Load repeat. |
| Resumed failure | Propagate before Record, construction or Observe. |
| Box allocation fault after reply 5 | P(3), L(10), R(5), failed A; no Observe or return. |
| Invalid state | Original IllegalStateException construction and throw; no logical-body helpers. |

The selected grammar must retain one call/state/resume association, the original
seed field pair, both normal-result/seed deliveries, source ordering and original
identities. It must use ordinary source/ABI scopes without needing a loop or a
new single-site renderer. Coroutine/VM execution is unrun. Expectations precede
candidate inspection.

## Preparation and observations

Prepared with Smali 2.5.2, API 24 and Temurin OpenJDK 21.0.12.1+1:

```sh
mkdir -p .cache/fixtures/protocol-single-site-core
java -cp "$DIANA_SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/protocol-single-site-core/classes.dex quality/fixtures/kotlin/protocol-single-site-core
cp .cache/fixtures/protocol-single-site-core/classes.dex quality/fixtures/kotlin/protocol-single-site-core/classes.dex
```

Inspect the same prepared DEX through dbg_shell with `p9-all` and text_only true.
Both revisions retain three code and four no-code methods, with no construction
or rendering failures. The baseline keeps executable state dispatch; the candidate
publishes one acyclic protocol and readable source/ABI scopes. The new test keeps
one call, the original spill field pair, two normal deliveries, original leaf
ownership and preparation/publication order. The 44-test scoped nextest set passes.
These are product/recovery checks, not executed coroutine/VM acceptance.
Strict all-feature engine/example/relevant-test Clippy and formatting pass.
Historical fixture API adaptation and coroutine/VM execution remain unrun/deferred.
