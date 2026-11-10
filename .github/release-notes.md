## Diana CLI 0.2.4

This release improves cross-reference queries, class lookup, and multi-DEX call
navigation.

### Cross references

- Code references are grouped by calling method and matched target rather than
  emitting a row for each instruction. The reported use is the first matching
  use in that method.
- Type queries include direct code references, field types, and method return
  types. Use `--parameter-types` to include method parameter declarations and
  `--typed-uses` to include types referenced through member signatures.
- Type queries no longer include superclass, interface, or annotation type
  declarations. `impl` and `extend` remain available for hierarchy queries.
- Reference scanning avoids unnecessary declaration and metadata decoding,
  stores hits compactly, and encodes selected results in parallel.

### Navigation and lookup

- Outline follows calls across DEX images in the same input archive.
- Method and field listings present the locator and member descriptor in
  separate columns.
- Name-based class and method lookup can use DEX prefixes without inflating
  every complete DEX image.

These archives contain the CLI executable and Apache-2.0 license. macOS and Linux
use `.tar.gz`; Windows uses `.zip`. Linux binaries target musl.

### Install

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.2.4/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.2.4
```

Or build the released CLI with Rust 1.93+ and native build tools:

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.2.4 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
Java-like output does not promise compilable Java. The 0.2 series remains
experimental.
