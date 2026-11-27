<p align="center">
  <img src="assets/diana.png" width="176" height="176" alt="Diana — a white, laurel-crowned profile on black">
</p>
<h1 align="center">Diana</h1>
<p align="center">Native Android and JVM decompilation and static analysis.</p>
<p align="center">
  <a href="#about">About</a> · <a href="#install">Install</a> · <a href="#usage">Usage</a>
</p>

## About

Diana is an in-development Rust decompiler for APKs, standalone DEX files,
JARs and standalone JVM class files. The CLI recovers Java-like source,
finds cross-references, compares versions and exports sources and resources.

[Apache-2.0](LICENSE)

## Install

With Cargo (Rust 1.100+ and native build tools; installs the CLI only):

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

Run `diana --help` for the command list or `diana <command> --help` for options.

#### Android APK and DEX

```sh
diana classes app.apk --filter MainActivity
diana get-class app.apk 'e0:l0:c123'
diana export app.apk -o apk-project
diana diff old.apk new.apk
```

Use the `at` value returned by `classes` to select an original class.
APK analysis covers root DEX files. The same commands also accept standalone
DEX files.

#### JAR and JVM class files

```sh
diana classes app.jar --filter Main
diana get-class app.jar com.example.Main -o Main.java
diana export app.jar -o jar-project
diana diff old.jar new.jar
```

Use `bytecode` to inspect the original instructions of a JVM method.
JVM analysis also accepts standalone `.class` files and `.diana-jvm.json`
project descriptors for multiple archives or a Java release view. See the
[CLI guide](apps/diana/README.md#jvm-archives-and-class-files) for input
selection, method locators and more examples.

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
