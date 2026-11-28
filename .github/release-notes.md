## Diana CLI 0.4.19

- DEX `fill-array-data` recovery evaluates the destination once and checks
  its full length before writing. Empty payloads still check for null.
- Recovered array source carries the original bits of floating-point NaN
  payloads. Proven fresh, fully populated arrays remain compact initializers.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.19/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.19
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.19 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
