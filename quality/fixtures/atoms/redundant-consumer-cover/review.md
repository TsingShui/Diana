# Redundant-consumer and alias review

LLM review compares the authored construction/alias input against ordinary Engine
output on `f859adf` and the redundant-root replacement. Container allocation and
its constructor complete before the alias Move. The same-input initialization
facts establish that the following Container check adds no behavior. A native
return descriptor alone was not a redundancy witness; the prepared input uses
the actual initialized allocation plus its alias.

The baseline emits new Container() and then returns v1 without assigning v1.
Its Move operation at the original alias PC has no source anchor. In barrier,
after still executes between construction and that invalid read. The omitted
check's operand owned the alias evaluation, but the omitted check prints no
evaluation; the alias write therefore disappeared with its carrier. This
contradicts the input's available initialized reference and original Move identity.

The replacement removes established redundant roots before producer coverage.
The alias remains available to its real consumer, with its original value/state
binding and selected result commit. A successful immediate path returns the
allocated object. Allocation failure runs no constructor, while constructor
failure runs no alias or after. Barrier keeps after after the completed
construction; its failure prevents normal return. The same object is returned
after normal after completion. Each original operation retains one anchor.

Six independent allocation/constructor/helper scenarios execute through the
limited selected-plan model and match their declared result and event order.
Restoring the old omitted-consumer rule causes both normal scenarios to read a
MissingValue. This is a final-plan correspondence failure, not an unimplemented
opcode or an input/JVM exception. No contradiction was found on the reviewed
normal or fault paths after the repair. Native helpers, Object's super call and
VM allocation remain modeled assumptions; VM/device execution is unrun.
