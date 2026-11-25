## Diana CLI 0.3.0

Run `diana export app.apk -o project` to export Java-like source by package.
A standalone DEX works with the same command. The destination must be new.

- Each original class definition gets a source file. Conflicting package paths
  use coordinate-based filenames.
- APK export includes a readable manifest when available and copies the
  original resources, assets, native libraries and other non-root-DEX entries.
- Use `--jobs N` to tune the number of simultaneous class analyses.
- Partial recovery keeps the available files and records diagnostics in
  `export-report.jsonl`. Output failures leave the destination unpublished.

Exception recovery shares immutable value origins within each method when
comparing cleanup paths across multiple protected regions. When those paths
pass different arguments to a cleanup call, recovery keeps the separate calls.
A typed handler with partial catch-all protection also retains its original
cleanup boundary, including calls whose exceptions escape the handler.
The optimizer keeps throwing expressions and cleanup calls within the
protected ranges that govern them.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.3.0/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.3.0
```

Or build the released CLI with Rust 1.93+ and native build tools:

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.3.0 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. macOS and Linux
use `.tar.gz`; Windows uses `.zip`. Linux binaries target musl.

After installing or upgrading the CLI, run `diana skill -i` to install the skill.
Java-like output does not promise compilable Java.
