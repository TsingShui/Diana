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
  entries         List the entries an archive holds, in central-directory order
  classes         List classes defined across every DEX in an APK
  tree            Show the package hierarchy of defined classes
  fields          List every field the archive DEX files declare
  methods         List defined methods, excluding reference-only method_ids entries
  strings         List every string in every root DEX of an archive
  resources       List resources.arsc IDs, names, configurations and original values
  get-class       Locate a class in an APK and decompile it
  get-method      Decompile one method as Java, selected by its methods.at coordinate
  outline-class   Show one class call tree with aggregated imports
  outline-method  Show the call tree below one method
  extract         Write one original entry to -o; --raw keeps binary XML
  xref            Find code references across every DEX in an APK
  manifest        Decode the binary AndroidManifest.xml
  help            Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

Examples:
  diana classes app.apk --filter MainActivity
  diana get-class app.apk 'e0:l0:c123'
  diana get-method app.apk 'e0:l0:c123:m45'
  diana outline-class app.apk 'e0:l0:c123' --call-depth 2
  diana xref app.apk string Authorization
  diana manifest app.apk
Copy classes.at into get-class; copy methods.at into get-method and outline-method.
Locators name the entry and index, so follow-up commands skip the APK-wide scan.
Outline follows calls across every DEX; a class outside the input stays [other-dex].
Java may contain explicit stubs for constructs the recovery cannot express.
```

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
