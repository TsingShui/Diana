# diana-decomp

Diana's DEX recovery backend.

Forked from [asLody/dexdec](https://github.com/asLody/dexdec) at revision
`08ed478781ddddb16381c31b9cf6d8946fdf2919`.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).

The fork keeps the recovery backend and consumes caller-supplied class,
method, instruction, try-range and local-symbol facts. It does not parse or
own a DEX image. The public decompilation entry is `Engine` in `src/engine.rs`.
