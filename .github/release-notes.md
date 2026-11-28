## Diana CLI 0.4.9

- DEX recovery preserves each catch's live state when allocation, argument
  evaluation, and initialization throw at different points. Diana publishes
  separate allocation and initialization operations when one `new` would
  change the original order or handler inputs.
- DecBench now checks first-use class initialization, constructor and
  publication faults, retries, and the identity of the allocated object.
  Its source-aligned checks read explicit initializers by original owner
  and descriptor, including array parameters.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.9/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.9
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.9 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
