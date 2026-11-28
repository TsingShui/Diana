# Working on the Diana application

The root `AGENTS.md` and `docs/diana/CONTRIBUTING.md` apply. Read the owning
implementation and package README before changing application behavior.

- This package owns file/input snapshots, ZIP/APK handling, query and result
  adaptation, CLI routing and atomic output publication. The engine remains the
  independent library in `crates/diana-decomp/`.
- Use `clap` major version 4 with `derive` for argument parsing and `libdeflater`
  major version 1 for whole-entry ZIP DEFLATE. Operator-approved streaming
  prefixes use `flate2` major version 1 with the `zlib-rs` backend. Obtain operator
  approval before introducing another parser or DEFLATE backend. The bundled
  native C dependency of libdeflater is approved.
- Do not delegate to or depend on rasc, dexdec or rusty-dex. Reuse historical
  input/ZIP ideas only after focused review with compatibility and security evidence.
- Exercise ordinary CLI commands or owning-module interfaces in regression tests.
- Build from the repository root with `cargo build --locked -p diana`.
