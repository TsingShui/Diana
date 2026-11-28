## Diana CLI 0.4.20

- Recovered DEX and JVM conditions preserve unordered floating-point paths
  when a guard branch is inverted or combined with a short-circuit test.
- Floating-point self-comparisons retain their NaN behavior. Integral
  comparisons still use compact inverse relational operators when proven.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.20/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.20
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.20 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
