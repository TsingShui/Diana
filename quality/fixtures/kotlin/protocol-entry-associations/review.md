# Entry-association path review

LLM review of the authored Smali/Java and ordinary Engine output at baseline
`a5e8fa6` and after the shared-entry replacement. The helper models and scenario
expectations were recorded before candidate inspection. This is a path judgment,
not executed helper/coroutine/VM equivalence.

## Captured body

The original initial failure check precedes the seed field read and Prepare in
both views. Prepare receives that original seed; its value supplies the first
Load key. The savedSeed write and state 5 publication remain original ordered
ABI operations before Load. The continuation occurrence belongs to `this` and
the original method reference, rather than an inferred helper implementation.

Immediate first completion copies the call result and seed into the same
existing merge storage used by the resume ingress. State 5 restores savedSeed
and checks the original parameter before those resume assignments. The candidate
retains both captures in its correspondence and their explicit copies; it does
not move the seed read after suspension or use the prepared key as the seed.
With seed 3 and first reply 5, the logical path therefore preserves P(3), L(10),
R(5,3). A negative reply preserves that Record and returns boxed -1 before any
second spill, state write, Load or Observe.

On the nonnegative path, savedSeed, savedReply and state 2 remain published in
their original order. The second Load receives the first reply. State 2 restores
both fields in the original order and runs its failure check before its merge
copies. The final Number read supplies Observe together with the preserved seed
and first reply; the returned object is the second reply, not a newly boxed
value. Replies 5/9 preserve O(9,3,5) and the second Integer(9) identity.

The two marker edges retain one original ABI return. Either marker prevents
that invocation's remaining logical tail. A resumed check fault prevents the
corresponding Record/Observe. First/second cast and null/intValue faults remain
at their original result reads and prevent later effects. The default still
constructs and throws the original IllegalStateException and message without
running modeled helpers. No contradiction was found on these modeled paths;
actual runtime linkage and arbitrary native helper behavior remain unrun.

## Publication and dispatch variants

The captureless members retain Load(5), the first Number/intValue read, Load of
that value, and the second reply return. Their comparison and sparse-switch
entries both associate state 5 with the first resume and state 2 with the second.
Resume input remains parameter zero in both encodings. The switch is not treated
as evidence for a nonexistent result field or continuation allocation.

Each call's single original result copy has an operation-owned successful
publication. It no longer also executes on the selected edge/body. The shared
marker return stays in one ABI owner, including when both calls own publications.
The product checks observe every original leaf once and each publication's
original transfer/copy identity. L(5), L(7) and the second Integer(9) remain the
modeled normal result for first/second replies 7/9. Suspension and first-result
faults stop their applicable tails; no additional helper is introduced.

## Resume-only effect guard

AssociatedBadResume adds R(seed,seed) only after the first resumed failure check.
That effect has no immediate-path counterpart. The candidate refuses the source
body and preserves the entire original member with both Record operations,
two loads, captures, marker and default/error paths. Its full text matches the
baseline. Thus R(3,3) precedes R(5,3) only on that resume path, and a failed
resume check prevents both. The positive path is not used to conceal this local
correspondence mismatch.

## Earlier effects and evidence boundaries

Seventeen existing Kotlin groups retain full text/issues and reuse their existing
reviews on that basis. The original protocol-common-controls member changes ABI
comments and gains the original merge target label. Its executable operations
remain unchanged: entry check, Prepare, state publication, Load, result read,
Record, negative guard and observed construction keep their original order.
The ABI comparison chain's failed edges fall through to the next original test
or invalid-state construction; selected edges retain their explicit destinations.
The resume assignment explicitly reaches the delivered merge label. These
changes retain routing and source correspondence without another execution path.

The final scoped nextest set observes source ownership and protocol associations
and separately executes the scalar/construction/lock models it supports. It
does not execute these coroutine scenarios. No numeric recovery score, whole
method verification badge or VM-equivalence claim is inferred from these checks.
