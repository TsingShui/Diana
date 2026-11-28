# Semantic review

The original first use of the allocated class occurs after preparation
and before the argument call. The class initializer records that point.
The 0.4.8 publication moves preparation into the first constructor
argument. Evaluating that recovered class creation initializes the class
before preparation. The independent Java execution exposes the swap.

If preparation throws, the original records that fault at stage 10 and
the allocated class remains cold. The publication initializes the class
before the fault, so a later successful retry sees an already warmed
class. If initialization throws, the original has already run preparation
and catches the original error object at stage 15. The publication skips
preparation in that scenario and reports stage 20.

The argument fault still occurs before constructor invocation and
publication. Its modeled stage 20 and throwable identity agree.
Constructor and publication faults preserve their objects and the
call-prefix behavior apart from the early initialization. The original
reports stage 30 on both, while the publication reports stage 20.
After a failed class initialization, the two sources both observe an
erroneous class on later use, with the same preparation and catch-stage
discrepancies.

The original DEX witness independently verifies the initializer and
constructor body hooks and interprets the retained caller protected
range, operand values and targets. The executed Java reference
exercises real JVM class initialization in each scenario process.
The conclusions cover these modeled hooks and their emitted traces.

The 0.4.9 publication performs preparation first and keeps allocation as
its own statement after stage 15. A preparation failure records stage 10
and a later call still performs first-use initialization. A class
initialization failure occurs after preparation and records stage 15.
The erroneous-class retry runs preparation and reaches allocation again,
where the model observes the VM's subsequent error type at stage 15.

The argument call runs after stage 20 and before stage 30. Its failure
carries the original exception to the handler and prevents initialization
and publication. A successful argument feeds the descriptor `(III)V`
along with the preparation result and stage 30. Only the allocated
receiver can be initialized and published. Constructor and publication
faults carry their own exception objects and the updated stage 30.
A successful repeat allocates a second receiver and uses the already
initialized class. The replay compares operation counts, order, argument
values, and receiver identity against the original DEX witness.

The prior `ArgumentEffects.tree` and `caughtError` controls still use
natural `new` expressions with protected argument calls. Their original
allocation, arguments, and initializer share handler inputs and dispatch.
The protected branch-join regression also exercises separately ordered
throwing expressions and mutually exclusive alternatives with one shared
handler state.
