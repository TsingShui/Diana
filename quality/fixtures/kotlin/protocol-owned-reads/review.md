# Owned protocol-observer review

LLM review compares the prepared Java/Smali with ordinary Engine output on
`f859adf` and the shared P7 observer/redundant-root replacement. The same DEX
supplies all class, constructor, protocol, storage and member facts.

Loop's initial path initializes sum/index to zero and checksum to bias. Each
iteration prepares its argument and captures sum + checksum before recording
checksum and invoking load. Immediate and resumed completion reach the same
body. The resumed path restores the exact limit/checksum/index/capture fields
and checks failure before consuming the returned value. Successful completion
updates sum, constructs ReadBox(sum, old checksum), observes its two fields,
updates checksum by the old index and records sum before incrementing index.
For limit one/bias two, that is 14; the two-iteration scenario is 28. A load or
allocation fault prevents all later work specified in the scenario table.

Flow preserves record(first) before its negative test. After a nonnegative first
reply, the second result is recorded and ReadBox(first,second) is constructed
and observed before the zero-second early return. The third state publishes
both scalar spills only after that work. State-two resume restores first and
performs construction/observe after second success. State-three resume restores
both values and does not execute construction again. Third success records its
value before the combined return; each fault prevents its later record/return.

The owned construction arguments retain their original initialization use sites
and exact order. Source declarations follow actual selected storage, including
the existing explicit protocol forwarding; raw read origins retain the original
storage. The matched continuation occurrence stays implicit, while ABI entries,
state publication and marker/error paths retain original correspondence.

The baseline lost the origin of a pure alias Move before its redundant frame
check. Removing redundant consumers before coverage preserves that alias and
its origin. The larger acting construction/alias counterexample is reviewed
in atoms/redundant-consumer-cover. No new protocol family or resume association
is inferred by this change.

The two protocol product checks pass and find no missing original operation
anchor. Path review found no contradiction on the declared loop, early-return,
resume, allocation and helper-fault paths. These coroutine paths are reviewed,
not executed by the selected-plan model or a VM. Complete protocol consolidation
and recovery-design acceptance remain unrun.
