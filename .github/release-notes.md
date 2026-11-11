## Diana CLI 0.2.6

This release fixes silent static-value corruption, unblocks platform framework
DEX inputs with hidden-API metadata, and preserves construction sequences that
previously failed semantic recovery.

### Decompilation fixes

- Original encoded static field values are restored, including mixed scalar
  widths, negative zero and original UTF-16 string units.
- Allocations and direct constructor calls with different owners remain explicit
  `allocate-instance` and `init-direct` operations. Their exact original targets
  and surrounding effects are preserved.
- DEX map kind `0xF000` is admitted as opaque platform hidden-API metadata under
  the shared envelope checks.
- Unavailable method bodies contain `// Diana: WARNING:` comments with the
  concrete reason and issue URL, instead of generated exception throws.
  Original input throws remain ordinary code.

### CLI compatibility changes

- `get-class` and `get-method` JSON now use schema 5 with method-local `issues`.
- Recovery failures retain the available publication, including `-o` output,
  write a diagnostic to stderr and return exit code 1 in both presentations.
- Automation consuming these commands needs to handle schema 5 and retained
  partial results on exit code 1.

The changelog also records the retirement of obsolete pipeline specifications
and their active documentation references.

These archives contain the CLI executable and Apache-2.0 license. macOS and Linux
use `.tar.gz`; Windows uses `.zip`. Linux binaries target musl.

### Install

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.2.6/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.2.6
```

Or build the released CLI with Rust 1.93+ and native build tools:

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.2.6 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
Java-like output does not promise compilable Java. The 0.2 series remains
experimental.
