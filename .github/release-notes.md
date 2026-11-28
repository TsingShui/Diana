## Diana CLI 0.4.3

- DEX recovery preserves call order across guarded values, including database
  reads before network requests and layout construction before adapter updates.
- Complex exception handling retains shared cleanup arguments and dispatches
  failures to the original handlers. If only part of a method can be recovered,
  Diana displays available code alongside the recovery diagnostic.
- Java-like conditions use more ordered `&&` and `||` expressions.
- DecBench adds full APK export evaluation and a browser view for reviewing
  per-method differences. FairEmail joins the reproducible public APK corpus.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.3/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.3
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.3 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
