## Diana CLI 0.4.6

- DEX recovery preserves the values an enclosing catch sees when normal and
  exceptional exits share one `finally`. The catch sees each value committed
  before the fault, including when cleanup replaces a pending exception.
- Three new DecBench fixtures check a wide timestamp, two values committed
  at different points, and faults on successive loop iterations.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.6/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.6
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.6 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
