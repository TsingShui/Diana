## Diana CLI 0.4.14

- DEX and JVM recovery preserve runtime static final field writes and the
  initialization triggered by cross-class field reads.
- Static initializer recovery keeps writes on the original declaring class,
  even when the current class and the target share a simple name.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.14/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.14
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.14 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
