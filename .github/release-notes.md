## Diana CLI 0.4.0

Run `diana diff old.apk new.apk` to compare methods across two APKs or
standalone DEX files. The comparison includes original method locators,
instruction changes, pairing evidence and unresolved matches.

Bulk export uses less CPU time and memory by reusing control-flow, predicate,
and Java type facts. Run `diana export app.apk -o project --jobs 16` to write
Java-like sources by package along with the APK manifest, resources, assets,
and native libraries. Local recovery issues appear in `export-report.jsonl`.

Guarded tail reconstruction preserves predicate inputs that change between
evaluations. Cleanup recovery keeps the order of calls and the exceptions
that can escape each protected path.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.0/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.0
```

Or build the released CLI with Rust 1.93+ and native build tools:

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.0 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. macOS and Linux
use `.tar.gz`; Windows uses `.zip`. Linux binaries target musl.

After installing or upgrading the CLI, run `diana skill -i` to install the skill.
Java-like output does not promise compilable Java.
