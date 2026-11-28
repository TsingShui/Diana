## Diana CLI 0.4.26

- Preserve saved values in interface static initializers, including field reads
  followed by mutation or a typed exception handler.
- Keep interface field writes at their original positions when intervening
  effects can observe publication during initialization.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.26/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.26
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.26 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
