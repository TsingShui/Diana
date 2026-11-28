## Diana CLI 0.4.2

The embedded Agent Skill now recommends commands and workflows for APK, DEX,
and JVM analysis. It shows how to reuse original class and method coordinates,
follow callers and callees, inspect archive entries and resources, compare
builds, and recover available sources and diagnostics.

Run `diana skill` to read the guide. For a first-time installation, run
`diana skill -i`. If you already installed the skill, review the printed
guide before replacing your installed copy.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.2/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.2
```

To build the released CLI from source, use Rust 1.100 or newer and native
build tools.

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.2 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. Linux and macOS
archives use `.tar.gz`. Windows archives use `.zip`. Linux binaries target musl.
