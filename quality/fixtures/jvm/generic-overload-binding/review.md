# Generic invocation overload review

`GenericOperand.<clinit>` first calls the generic producer with descriptor
`()Ljava/lang/Object;` and stores the result in an `Object` local. The
consumer invokes `select(Object)` as encoded. That method verifies the
argument identity, adds trace digit 2, and returns `object`. Diana 0.4.27
puts the generic call directly in the argument. Javac recompiles both
publication forms to `select(String)`, returning `string/13` where the
original returned `object/12`.

`GenericArrayOperand.object` evaluates the leading argument before the
nested argument and generic producer. The original consumer accepts
`Object` and checks the identity of the null, array, or string value.
The 0.4.27 class and selected-method publications preserve the producer
order while discarding the original argument type. Both recompile with
an array cast and the `String[]` overload. Null and the array select the
wrong overload, and the string throws before any overload runs.

The control `GenericArrayOperand.array` has an original array cast.
It selects the array overload for null and array values and throws at
the cast for the string value. It keeps the original consumer descriptor
on both endpoints. Leading, nested, and capture faults retain their
identity and trace in both challenge methods. The competing-fault
scenario still stops at the leading producer.

Diana 0.4.28 gives the nested producer an explicit `Object` type in
both publications of the initializer and the object challenge.
Recompilation selects both original `Object` overloads. The challenge
now delivers the scalar to the object consumer and records trace 1234.
It still evaluates the leading and nested arguments before that consumer.
The array control retains its original cast and overload, and all fault
paths preserve the original prefix of effects and exception identity.
