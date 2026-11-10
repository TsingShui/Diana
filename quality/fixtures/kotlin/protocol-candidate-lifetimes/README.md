# Protocol candidate lifetime and outside observers

Project-authored Apache-2.0 representation challenge derived from
[protocol-owned-reads](../protocol-owned-reads/README.md). Baseline: `f565f88`.
The loop still saves/restores its pre-load checksum and other original state.
The added interaction observes that checksum in a second construction after
loop completion, including zero iterations. It therefore cannot become a private
repeated-body alias whose original store or outside read disappears.

Original selector: Lfixture/IntervalLoop;,
total(IILkotlin/coroutines/Continuation;)Ljava/lang/Object;.
Java explains the normal run with its frame supplied to the logical helper; Smali fixes marker exits, reentry, spills and the
original invalid-state arm. ReadBox's constructor calls Object first, then stores
first and second. No source or helper is an external engine declaration input.

## Independent scenarios

Prepare records P(i) and returns i + 10. Load records L(argument), returning that
integer through Number, a distinct suspended marker, or a fault. Record records
R(n); Observe records O(first,second), and each ReadBox allocation records A.
All arithmetic wraps as Java int. Marker acquisition, class initialization and
super construction are assumed successful. Native helper models are not executed
VM/callee implementations. Reentry consumes the original saved values and the
same reply; a resumed failure propagates before downstream work.

| Scenario | Required result and ordered effects |
|---|---|
| total(0,2) | 2; A, O(0,2), R(0). No prepare/load or body box. |
| total(1,2), immediate or resumed load | 14; P(0), R(2), L(10), A, O(12,2), R(12), A, O(2,2), R(1). |
| total(2,2), either load resumes | 28; the first body trace above, then P(1), R(2), L(11), A, O(25,2), R(25), A, O(14,3), R(2). |
| First load faults | P(0), R(2), L(10), fault; no box/Observe or tail. |
| Second load faults | First body trace, then P(1), R(2), L(11), fault; no second body box or tail. |
| Zero-iteration tail allocation faults | Failed A; no Observe, Record or return. |
| Tail allocation faults after one iteration | First body trace, then failed A; no O(2,2), R(1) or return. |
| Invalid state | Original IllegalStateException construction and throw; no logical-body helpers. |

The common product must retain exact call/state/resume/spill associations and
one static placement for every original leaf. Copy forwarding must account for
actual outside observers. This challenges removal of published per-form plans;
a baseline with correct output is a compatibility guard, not a newly repaired
behavioral defect. Coroutine/VM execution is unrun.

## Preparation

Assemble under `.cache/fixtures/protocol-candidate-lifetimes`, publish the
successful classes.dex beside these sources, then inspect the same DEX on both
revisions using the ordinary dbg_shell `p9-all` command.

Prepared with Smali 2.5.2, API 24 and Temurin OpenJDK 21.0.12.1+1:

```sh
mkdir -p .cache/fixtures/protocol-candidate-lifetimes
java -cp "$DIANA_SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/protocol-candidate-lifetimes/classes.dex quality/fixtures/kotlin/protocol-candidate-lifetimes
cp .cache/fixtures/protocol-candidate-lifetimes/classes.dex quality/fixtures/kotlin/protocol-candidate-lifetimes/classes.dex
```

Both revisions retain three code and four no-code methods with no stops or
rendering issues. Their class texts are identical. The candidate-lifetime test
checks canonical reentry/spill facts, one static leaf placement and the retained
outside read. This supports the representation change without claiming a new
behavioral repair. Coroutine/VM execution is unrun.
The 43-test scoped nextest set and strict scoped Clippy pass. Formatting and
desensitization pass; the full historical suite is not claimed as passing.
