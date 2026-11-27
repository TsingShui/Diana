<p align="center">
  <img src="assets/diana.png" width="176" height="176" alt="Diana — a white, laurel-crowned profile on black">
</p>
<h1 align="center">Diana</h1>
<p align="center">Native APK/DEX decompilation and static code investigation.</p>
<p align="center">
  <a href="#about">About</a> · <a href="#install">Install</a> · <a href="#usage">Usage</a>
</p>

## About

Diana is an in-development Rust decompiler for APK and DEX files.
Diana GUI is in development and aims to support more than DEX.

[Apache-2.0](LICENSE)

## Install

With Cargo (Rust 1.93+ and native build tools; installs the CLI only):

```sh
cargo install --git https://github.com/TsingShui/Diana.git --branch main --locked diana
```

From a release (macOS/Linux):

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/main/install.sh -o install-diana.sh &&
  sh install-diana.sh
```

Or download a binary from the [releases page](https://github.com/TsingShui/Diana/releases).

## Usage

### CLI

```sh
diana --help
Analyze APK archives and standalone DEX files

Usage: diana <COMMAND>

Commands:
  skill           Print the Diana Agent Skill, or install it with -i
  entries         List the entries an archive holds, in central-directory order
  classes         List classes defined in every root DEX
  tree            Show the package hierarchy of defined classes
  fields          List every field declared in every root DEX
  methods         List defined methods, excluding reference-only method_ids entries
  strings         List every string in every root DEX
  resources       List resources.arsc IDs, names, configurations and original values
  get-class       Locate a class in an APK or DEX and decompile it
  get-method      Decompile one method as Java, selected by its methods.at coordinate
  outline-class   Show one class call tree with aggregated imports
  outline-method  Show the call tree below one method
  diff            Compare DEX methods across two versions
  export          Export Java-like sources by package, APK resources and the manifest
  extract         Write one original entry to -o; --raw keeps binary XML
  xref            Find code references in every root DEX
  manifest        Decode the binary AndroidManifest.xml
  help            Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

Examples:
  diana classes app.apk --filter MainActivity
  diana strings app.apk --filter Authorization
  diana get-class app.apk 'e0:l0:c123'
  diana get-method app.apk 'e0:l0:c123:m45'
  diana outline-class app.apk 'e0:l0:c123' --call-depth 2
  diana diff old.apk new.apk --format json -o changes.json
  diana xref app.apk string Authorization
  diana manifest app.apk
Copy classes.at to get-class, methods.at to get-method/outline-method; locators (entry:index) skip the APK-wide scan.
Root DEX only: classes.dex and classes<N>.dex at the archive root, or a bare DEX; DEX in assets/, res/raw/ or nested archives needs extract, then a bare-DEX run.
Lists show 50 rows by default and warn on truncation; --no-limit reaches several MB on a large APK, so prefer --filter or -o.
Outline crosses root DEX; a class outside the input stays [other-dex]. Java may contain explicit stubs.
```

Install the Agent Skill to `~/.agents/skills/diana/SKILL.md`:

```sh
diana skill -i
```

Installation leaves existing files unchanged. Run `diana skill` to print the
content instead.

Type cross-references include code uses, field types, and method return types.
Add `--parameter-types` to include method parameter declarations, and
`--typed-uses` to include type references through called methods and fields:

```sh
diana xref app.apk type Activity --substring
diana xref app.apk type Activity --substring --parameter-types
diana xref app.apk type Activity --substring --typed-uses
```

Code references are grouped by caller and target. Cross-reference results show
up to 50 rows by default; use `--no-limit` to show all matches.

### GUI

Still in development.

### Project Goals

For fun.

### RSS / Speed

Diana aims for low memory use and fast queries on large APKs.
