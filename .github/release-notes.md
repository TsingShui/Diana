## Diana CLI 0.4.1

Diana now analyzes JAR archives and standalone JVM class files. Use
`diana get-class client.jar com.example.Main` to recover Java-like source or
`diana export client.jar -o project` to export sources by package and retain
non-class resources. JVM inputs also support bytecode, cross-reference and
outline queries.

Run `diana diff old.apk new.apk` to compare root DEX methods across APKs or
standalone DEX files. Use `diana diff old.jar new.jar` for JVM archives. The
comparison includes original method locators, instruction changes, pairing
evidence and unresolved matches.

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
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.4.1/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.4.1
```

Or build the released CLI with Rust 1.93+ and native build tools:

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.4.1 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. macOS and Linux
use `.tar.gz`; Windows uses `.zip`. Linux binaries target musl.

After installing or upgrading the CLI, run `diana skill -i` to install the skill.
Java-like output does not promise compilable Java.
