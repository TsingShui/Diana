## Diana CLI 0.4.17

- DEX and JVM recovery preserve active reads of static final fields with
  implicit default values, including the effects and failures of class
  initialization. Original constant values remain distinct.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.17/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.17
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.17 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
