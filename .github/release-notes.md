## Diana CLI 0.2.3

A structural release: outline now answers call trees, locators are shorter, and
listing commands stop flooding the terminal.

### Outline

- outline-class and outline-method publish a call tree built from P0
  declarations and P1 invoke sites only; they no longer run the decompiler.
- The class node starts with an imports child: a package tree whose leaves
  aggregate every external type with the called methods and their counts.
- Calls into the same DEX become tree nodes carrying their locator and expand to
  --call-depth N or --expand-all. Repeated and cyclic methods become
  "-> see #N" links; classes in another DEX of the same APK stay [other-dex]
  leaves.
- Nodes render descriptors as simple type names, for example
  MainActivity.access$000(AmeSSActivity, Configuration) #11 [e1332:l0:c7448:m59042].

### Locators

- get-class, get-method and both outline commands accept index-only locators:
  e<entry>:l<logical>:c<class> and ...:c<class>:m<method>. Follow-up commands
  skip the APK-wide class scan.
- classes publishes two columns, at and descriptor, in text and JSON.

### Listing limits

- classes, fields, methods and strings show at most 50 rows by default, matching
  xref. --limit N changes the cap and --no-limit shows every match.
- Text output ends with "//Diana Warn: Too much results, only show top N ...";
  JSON carries "truncated": true.

### Performance

- The embedded platform ABI database decodes on demand from a class-granular
  archive instead of decompressing and cloning the whole database per process.
  On a 199 MB multi-DEX APK, get-class for a 155-method class dropped from
  0.20-0.25 s / 433-442 MB to 0.07 s / 61 MB, with byte-identical output.

### Tooling

- The root help now describes what each command does and ends with examples plus
  the locator hand-off between commands.
- The CI clippy gates pass on the pinned 1.93 toolchain and on stable.

These archives contain the CLI executable and Apache-2.0 license. macOS and Linux
use .tar.gz; Windows uses .zip. Linux binaries target musl.

### Install

Use the shell installer on macOS/Linux:

    curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.2.3/install.sh -o install-diana.sh
    sh install-diana.sh diana-v0.2.3

Or build the released CLI with Rust 1.93+ and native build tools:

    cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.2.3 --locked diana

Windows users can extract diana.exe from the matching ZIP and add its directory
to PATH.

Outline expansion follows same-DEX calls only; it is not a runtime call graph.
Java-like output does not promise compilable Java. The 0.2 series remains
experimental.
