## Diana CLI 0.4.10

- DEX recovery keeps observable element computations before `filled-new-array`.
  For example, FairEmail's `getSortTitle` evaluates `getDisplayName` and reads
  the account name before allocating the array.
- DecBench now exercises array allocation and element producer faults,
  typed catches, and the identity of repeated references in array results.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.10/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.10
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.10 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
