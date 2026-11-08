## Diana CLI 0.1.0

The first native Diana CLI binary release, built from the independent Rust
APK/DEX decompiler engine.

- Browse packages, classes, methods, fields and strings across DEX images.
- Search with ranked fuzzy, substring, regex and exact matching.
- Inspect references, recursive hierarchy, implementations and overrides.
- Read Java-like class/method results and structured outlines with call links.
- Decode manifests, extract entries and query resource values and ID candidates.
- Retain original identities, source spans, local diagnostics and incomplete coverage.
- Stream text and JSON through the shared buffered, atomic output publisher.

These archives contain the CLI executable and Apache-2.0 license. macOS and Linux
use `.tar.gz`; Windows uses `.zip`. Linux binaries target musl.

### Install

Use the shell installer on macOS/Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.1.0/install.sh -o install-diana.sh &&
  sh install-diana.sh diana-v0.1.0
```

Or build the released CLI with Rust 1.93+ and native build tools:

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.1.0 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add its directory
to `PATH`.

Static references and call expansion are not a proven runtime call graph. Resource
ID matches remain candidates, and Java-like output does not promise compilable
Java. The 0.1 series is experimental.
