# Working on the Diana application

The root `AGENTS.md` and `docs/diana/CONTRIBUTING.md` apply. Read the owning
implementation and package README before changing application behavior.

- This package owns file/input snapshots, ZIP/APK handling, query and result
  adaptation, CLI routing and atomic output publication. The engine remains the
  independent library in `crates/diana-decomp/`.
- Use `clap` major version 4 with `derive` for argument parsing and `libdeflater`
  major version 1 for whole-entry ZIP DEFLATE. Operator-approved streaming
  prefixes use `flate2` major version 1 with the `zlib-rs` backend. Obtain operator
  approval before introducing another parser or DEFLATE backend. The bundled native C dependency of
  libdeflater is approved; active Rust code forbids unsafe.
- Do not delegate to or depend on rasc, dexdec or rusty-dex. Reuse historical
  input/ZIP ideas only after focused review with compatibility and security evidence.
- Source fixtures and regression inputs follow
  [FIXTURES](../../docs/diana/FIXTURES.md).
- Automated tests follow the root testing policy. Exercise ordinary application
  commands or owning-module interfaces; preserve production input, query,
  extraction and safety checks. Test authorization does not add production
  verification or another engine path.
- Build from the repository root with `cargo build --locked -p diana`.
