# JVM source metadata

This independently authored class carries bounded class and method type
parameters, a parameterized field, an abstract method, and adjacent wide
parameters. The application test compiles it using Java 17 source
and class format with a JDK 17 or newer and `-g`. One variant also enables `-parameters`.

The class signature describes `T extends Number`, the field uses `List<T>`,
and `pick` returns its second parameter with method-local type `U extends T`.
`delta` subtracts the long parameter from the double parameter. The concrete
methods retain their argument names from local lifetimes when the declaration
parameter table is absent. The abstract declaration has its explicit argument
name in the variant with `MethodParameters`.

The test mutates two independent signature constants without changing the
descriptors or executable Code. An invalid class signature leaves the erased
class, field and method declarations available. A method signature that
contradicts its descriptor leaves the original long argument, erased return
type and original return statement. The CLI publishes each affected declaration
location and the same result text in both presentations.

The local regression run used OpenJDK 21.0.12.1.
