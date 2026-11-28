## Diana CLI 0.4.5

- DEX recovery preserves the local state at explicit rethrows after cleanup.
  In FairEmail's `monitorAccount()`, early scheduling failures retain the
  previous timestamp, and failures after a refresh retain the fresh value.
- Two new DecBench fixtures exercise this transfer through split catcher
  entries and a wide register move. They also exercise cancellation failures
  that replace a pending exception.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.5/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.5
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.5 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
