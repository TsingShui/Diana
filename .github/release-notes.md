## Diana CLI 0.4.24

- Recover counted `for` loops when a guarded or trailing `continue` delivers
  the induction update, keeping exit effects inside their original branches.
- Keep updates in the loop body when a continuation skips them or when
  intervening effects, exception handling, or monitor release prevent moving them.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.24/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.24
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.24 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
