## Diana CLI 0.4.15

- DEX recovery keeps the monitor instructions of declared-synchronized
  bytecode methods. The DEX access flag remains available as input metadata.
- JVM recovery preserves explicit locks inside implicitly synchronized
  methods. DEX and JVM recovery both retain nested acquisitions of the same
  monitor and their exceptional release paths.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.15/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.15
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.15 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
