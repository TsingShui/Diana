## Diana CLI 0.4.22

- DEX and JVM recovery retain noncanonical NaN payload bits in returns,
  branch selections, field stores, and other executable expressions.
- Encoded field and annotation values show NaN bit literals. JVM
  ConstantValue fields retain their early initialization provenance.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.22/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.22
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.22 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
