# Exception producer types at a loop join

This independently authored fixture reduces the unresolved exception value
seen in the FairEmail POP3 method
`Lcom/sun/mail/pop3/Protocol;->retr(ILjava/io/OutputStream;)Z`.
The authored owner is `Lfixture/ExceptionJoinTypes;`, with
`run()V` and `runShared()V`. The loop joins an initial null with the
exception value written at each typed catch ingress. Later code checks for
null and throws the retained value using its original cast and route.

The challenge uses two ordered DEX catch clauses that share one landing
pad. It requires the handler's producer evidence to admit both exception
types. The distinct-handler case records which catch ran. The shared-handler
case calls the common helper once irrespective of the selected catch type.

The native model assumes that `nextStep` returns a finite sequence
terminated by zero. A successful write keeps the prior exception. A typed
write failure updates the saved exception, calls the corresponding observer
and resumes the loop. Observer calls complete normally. Normal termination
returns when there was no failure. On termination after one or more failures,
the method rethrows the last caught object with the same dynamic type.
An exception outside the two declared types escapes directly from the write.

The DEX was assembled into `.cache/fixtures/exception-join-types/classes.dex`
using the Smali builder from [Apktool 2.11.1](https://github.com/iBotPeaches/Apktool/releases/tag/v2.11.1)
on OpenJDK 21.0.12.1, API 24. The Java launcher used
`SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`.

Diana 0.4.2 at `49af4a17` reports unresolved source variable allocation
in both methods through `get-method`. The working-tree candidate restores
both bodies and the original cast and throw routes. The engine test checks
the displayed catches and both terminal throws. The effects and exception
paths were reviewed against the Smali. The fixture does not execute the
rendered methods or the native helpers. [Semantic review](review.md)
