## Diana CLI 0.4.4

- DEX recovery emits one `finally` when multiple protected ranges forward
  to shared cleanup and the normal path runs equivalent cleanup.
- Nested typed catches and values observed by an enclosing catch stay on
  their original paths. The cleanup runs on both normal and exceptional exits,
  and a cleanup failure replaces the pending exception.
- In FairEmail's `monitorAccount()`, the keepalive alarm cancellation is
  now expressed once in a `finally`. Three new DecBench fixtures cover
  the split-entry mechanism and its interaction with enclosing handlers.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.4/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.4
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.4 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
