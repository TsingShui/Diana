## Diana CLI 0.2.5

This release improves cross-reference performance and cleans up the unused
profiling compatibility shim.

### Cross references

- Cross-reference scans apply resolved target-index prefilters before decoding
  annotations, static values, field types, and method return types.
- Field and method annotation association no longer resolves complete member
  declarations when the query only needs the declaring member identity.
- Type queries can answer return-type and field-type relations without
  materializing unrelated declarations.

### Instrumentation

- The opt-in `perf` feature adds fastrace spans and aggregated xref,
  annotation, class-data, batch, resolve, analyze, and publish counters.
- Normal release builds remain instrumentation-free.
- Removed the unused no-op `profile_scope!` compatibility macro and its
  `profiling.rs` module.

These archives contain the CLI executable and Apache-2.0 license. macOS and Linux
use `.tar.gz`; Windows uses `.zip`. Linux binaries target musl.

### Install

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.2.5/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.2.5
```

Or build the released CLI with Rust 1.93+ and native build tools:

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.2.5 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
Java-like output does not promise compilable Java. The 0.2 series remains
experimental.
