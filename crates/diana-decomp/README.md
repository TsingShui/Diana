# diana-decomp

Diana's DEX recovery backend.

Forked from [asLody/dexdec](https://github.com/asLody/dexdec) at revision
`08ed478781ddddb16381c31b9cf6d8946fdf2919`.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).

The fork keeps the recovery backend and owns the DEX reader, P0 class facts and
P1 decoded methods. The application still owns the files and lends the DEX bytes;
the engine never owns or reloads an input image. The public decompilation entry is
`Engine` in `src/engine.rs`; methods do not pass through a separate
`MethodCode`/`LocalSymbols` conversion.

Class and selected-method analysis return method-local `failures` beside the
rendered source. Consumers can retain available results while distinguishing an
incomplete recovery from a successful request without scanning source text.
Unavailable method bodies print Diana warning comments with the concrete reason
and issue URL, retaining the distinction from original executable statements.

Allocation and direct initialization remain separate when their original owners
differ. The backend prints `allocate-instance` and `init-direct` with the exact
constructor reference; it does not substitute the allocated class's constructor.
These explicit operations keep their original sites, aliases and effects.

DEX map kind `0xF000` (platform hidden-API class-data metadata) is admitted as an
opaque section under the shared map envelope checks. Its hidden-API flags are
not interpreted or used to infer source declarations or values.
