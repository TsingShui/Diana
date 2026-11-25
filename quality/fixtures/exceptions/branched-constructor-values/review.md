# Branched constructor values review

In the Smali both constructor choices for the first object join before the
protected range. The single-argument constructor records and stores 3.
The two-argument constructor adds 3 and 4, then records and stores 7.
A recording failure here escapes without allocating the second object. On
success the first initialized object reaches both the normal use and the
handler. The second constructor records and stores 5 or 11, according to its
independent flag. A matching failure at that stage reaches the handler without
publishing or reading a partially initialized second value.

Diana 0.3.0 retains the two first constructor calls in the respective branches
and expresses the second pair as a conditional inside the consumer arguments.
The rendered expression reads the first object's field before it evaluates
the second constructor. The Smali reads it afterwards. The already initialized
first object is non-null, its field is final, and neither constructor nor helper
publishes that object or writes its field again. Both schedules therefore supply
the same first argument and expose the same calls and exceptions. Successful
routes pass (3,5), (3,11), (7,5) or (7,11) to the consumer.

A matching consumer fault and a matching second-constructor fault each reach
`recover` with the first object's field. The handler runs outside its own
protected range. Its return or failure escapes directly. An ArithmeticException
from the protected second constructor propagates with its original identity.
The optimized release build produces byte-for-byte identical Java to
Diana 0.3.0 for the complete class. Its first join still supplies the same
object to the handler, and the second conditional constructor runs inside
the original protected scope. The paired Java explanation and both generated
classes each passed the nine independent host-Java scenarios using the
fixture's helper implementations. The execution covers injected helper faults.
Allocation failure and Android runtime execution were outside the run.
