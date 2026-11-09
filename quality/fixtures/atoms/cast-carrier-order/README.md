# Cast carrier operand order

Independently authored Java and Smali under the repository Apache-2.0 license.
Original owner: `Lfixture/CarrierOrder;`. Smali is the assembly input; the engine
receives only its DEX bytes, without external declarations.

## Independent expectations

`right(Ljava/lang/Object;Ljava/lang/Object;)I` checks the right operand as int[]
before comparing reference identity. The left operand is an ordinary parameter
read. Relocating the check into the right comparison operand is safe because that
preceding read neither acts nor throws. `left` checks the first operand instead.
`guarded(Ljava/lang/Object;Ljava/lang/Object;I)I` checks right before testing flag;
a nonzero flag returns 99, but must still throw if right has an incompatible type.
No carrier on the later comparison can replace that unconditional check.

`afterEffect(Ljava/lang/Object;)Ljava/lang/Object;` checks its input, calls make,
then returns the checked reference. `arrayLength(Ljava/lang/Object;)I` checks
then reads length. `argument(Ljava/lang/Object;)V` checks then calls sink with
integer zero followed by the exact checked reference.

References are null, an int[] A, a distinct int[] B, or an incompatible ordinary
object O. Cast preserves identity, permits null, and throws ClassCastException
for O. Array length on null throws NullPointerException after the successful
null cast; A has length 3 and B length 5. Reference comparisons never call equals.
Native make records M, returns a modeled reference, and may throw E. Native
sink(n,r) records S(n,r), returns void, and may throw E. Class initialization is
assumed established; no helper or VM is actually executed in the written review.

| Subject/scenario | Expected result or error | Ordered helper effects |
|---|---|---|
| right(A,A), right(null,null) | 1 | empty |
| right(A,B), right(A,null) | 0 | empty |
| right(A,O) | ClassCastException before comparison | empty |
| left(A,A), left(null,null) | 1 | empty |
| left(O,A) | ClassCastException | empty |
| guarded(A,A,0) | 1 | empty |
| guarded(A,B,0) | 0 | empty |
| guarded(A,A,1) | 99 | empty |
| guarded(A,O,1) | ClassCastException, not 99 | empty |
| afterEffect(A), make returns B | exact A | M |
| afterEffect(null) | null | M |
| afterEffect(O) | ClassCastException before make | empty |
| afterEffect(A), make faults | exact E | M |
| arrayLength(A), arrayLength(B) | 3, 5 | empty |
| arrayLength(null) | NullPointerException | empty |
| arrayLength(O) | ClassCastException | empty |
| argument(A), argument(null) | void | S(0,A), S(0,null) |
| argument(O) | ClassCastException before sink | empty |
| argument(A), sink faults | exact E | S(0,A) |

At authorship, preparation and inspection were not run. Baseline: `dd3a114`.
The right-operand case is the discovery; the contexts fixture is its complexity
challenge. The shared traversal MUST preserve original check identity, site and
operand state, retain casts before selections/effects, and permit a safe later
operand without a special comparison template. Historical fixtures remain
deferred at the operator's request.

## Preparation

```sh
mkdir -p .cache/fixtures/cast-carrier-order
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/cast-carrier-order/classes.dex quality/fixtures/atoms/cast-carrier-order
cp .cache/fixtures/cast-carrier-order/classes.dex quality/fixtures/atoms/cast-carrier-order/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/atoms/cast-carrier-order/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

## Observed checks

Prepared on 2026-11-10 with Smali 2.5.2, API 24 and Temurin OpenJDK
21.0.12.1+1. Native selectors are make()Ljava/lang/Object; and
sink(ILjava/lang/Object;)V; left has the same full prototype as right.
The same DEX is inspected on dd3a114 and the containing refactor. Baseline
right and argument retain standalone checks; candidate places each at its safe
later operand. Left, guarded, afterEffect and arrayLength retain their text.
Both deliveries contain eight methods, six executable results and no member or
render failure. Three recovery checks reject the baseline's absent inline
carriers; that intentionally failing sensitivity run is not a passing gate.
