## Diana CLI 0.4.27

- Preserve evaluation order and exception timing when recovering field
  initializers with calls, conditional expressions, or short-circuit tests.
- Keep saved values at their original evaluation points when a field
  initializer cannot consume them as its first mandatory evaluation.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.27/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.27
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.27 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
