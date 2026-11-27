# JVM multidimensional allocation

`Tensor.partial` allocates two dimensions of a three-dimensional reference
array. The two dimension helpers run left to right and update the order field
to 12. The third dimension stays unallocated.

`Tensor.guarded` adds a third evaluated dimension, an arithmetic trace and a
typed catch. Its positive path returns the outer length and records 123.
Negative lengths, including a negative inner length after a zero outer
length, still evaluate all three expressions. The catch returns -123.
`CheckTensor` encodes these outcomes independently.

```sh
cargo nextest run --locked -p diana --test jvm_arrays_test
```

The test uses OpenJDK's compiler module to prepare the original input.
It recompiles Diana's recovered class and executes the independent scenarios
with the JDK. The JVM specification defines dimensions as an ordered stack
sequence and permits the declared array rank to exceed the allocation count.
[Java Virtual Machine Specification, multianewarray](https://docs.oracle.com/javase/specs/jvms/se25/html/jvms-6.html#jvms-6.5.multianewarray)
