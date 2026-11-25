# Export review

On baseline `eac4bb4`, the CLI rejected `export` with exit 2. The Diana 0.3.0
candidate delivered package files on the complete, duplicate, nested and partial
variants. I compared the files with the original Java and Smali and read the
local recovery reports. The judgments below concern the inspected definitions.
The APK was not loaded into ART.

| Definition or boundary | Path-by-path review |
|---|---|
| ExprArith | Addition, subtract-one, remainder by seven and negation retain signed int arithmetic. The multiplication retains the long operand and factor 17. Both shifts have the original direction and count. The bitwise expression applies AND and XOR before OR. Division retains its divisor, including the zero-divisor exception. |
| BranchIfz | A nonzero input to `bothPaths` stores 4 then returns 0. Zero stores 3 then returns 1. The other three methods store 1 on zero, 2 on nonzero or 5 on null, respectively. Their other paths perform no write. |
| ClassInnerStatic and Holder | The factory constructs one Holder with the argument. The outer call obtains that object before invoking its doubled method. A failed construction prevents the invocation. Holder's own file first assigns the constructor argument. Its method reads the field twice, stores the sum, reads the updated field and returns it. The outer file contains the call site and the member file contains the body. |
| DirectInitialization | The original direct call on an allocated Child names Parent and passes the argument. Its return uses the original reference. The ordinary constructor has its own Child call and overwrites the inherited field with 99 after Parent returns. The broken method keeps its allocation-without-constructor diagnostic. Recovery retains both other methods and the classes in the other roots. |
| Duplicate Parent and Child definitions | The first Parent stores the parameter. The second invokes event 2 before its store, so a failing event prevents that store. Both Child files invoke their respective Parent before assigning 99. Their identical bodies are separate definitions. The alternate-file report identifies the descriptor, physical root, logical image and class definition. |
| DirectInitializationEffects | The visible event helper appends the argument to trace and throws on equality with fault. For nonnegative input the order is allocation, event 1, Parent/event 2, event 4, publication, return. Negative input inserts event 3 after 1. A RuntimeException from an executed event in the protected range transfers to event 5 before rethrow. Failure in event 5 escapes. Errors outside the handler type do not take that edge. The duplicate archive models each physical definition separately. Runtime class-loader selection among duplicate types is unspecified. |
| Resources and input failures | The complete archive delivers both classes and five resource files plus the readable empty manifest element. The partial archive still delivers five classes and five resources. Its five local issues distinguish the damaged stored asset, unsupported compression, truncated DEX, DEX CRC mismatch and method failure. A pre-CRC candidate had accepted the otherwise intact DEX as a sixth class. The checked reader preserves the intact roots while reporting the damaged root. |

The exact-output comparisons exercise export and ordinary retrieval on the
same prepared definitions. They establish delivery parity. The semantic judgments
above come from reading the operations and branches.
