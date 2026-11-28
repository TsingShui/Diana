## Diana CLI 0.4.7

- DEX and JVM recovery preserve integral overloads in calls, `new`
  expressions, and constructor delegation. The published arguments keep
  their value and select the original wider formal where needed.
- DecBench now checks crossed numeric overloads and edge values such as
  U+FFFF and negative bytes. A JVM fixture also checks the selected
  constructors, returned values, and effect order.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.7/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.7
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.7 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
