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
Diana-Gui will support more than Dex,but still in devlopment.

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

The [installer](install.sh) detects x86_64/aarch64 and installs into
`~/.local/bin`. Use `DIANA_INSTALL_DIR` for another directory, or pass a release
tag: `sh install-diana.sh diana-v0.1.0`. It accepts `diana-<target>.tar.gz`,
`diana-<version>-<target>.tar.gz` or `diana-v<version>-<target>.tar.gz` containing
one `diana` executable. Windows binaries are available as ZIP archives on the
[releases page](https://github.com/TsingShui/Diana/releases).

From a local checkout:

```sh
git clone --branch main https://github.com/TsingShui/Diana.git diana
cd diana
cargo install --path apps/diana --locked
```

## Usage

### CLI

Commands accept APK/ZIP or bare DEX input. Use `--format json` for machine-readable
results and `-o <file>` for atomic file output.

```sh
# Explore the input and find methods with ranked fuzzy search.
diana tree app.apk --classes
diana methods app.apk --filter oncrt

# Read a class or one exact method.
diana get-class app.apk 'com.example.Worker'
diana get-method app.apk 'Lcom/example/Worker;->run()V' --format json

# Navigate structure and expand referenced method bodies.
diana outline-class app.apk 'com.example.Worker'
diana outline-method app.apk 'Lcom/example/Worker;->run()V' --call-depth 2

# Find callers, indirect implementations and method overrides.
diana xref app.apk method 'Lcom/example/Worker;->run()V' --exact --transitive 2
diana xref app.apk impl 'Ljava/lang/Runnable;' --exact --recursive
diana xref app.apk method 'Lcom/example/Base;->run()V' --exact --relation overrides

# Inspect manifests, resource values and resource-ID candidates.
diana manifest app.apk
diana resources app.apk --filter api_host --substring
diana xref app.apk resource 0x7f120123
```

Search results carry canonical locators that select the original object without
name ambiguity. Filtered searches default to relevance-ranked fuzzy matching;
`--substring`, `--regex` and `--exact` select other modes.

`xref` displays up to 50 rows by default and warns when limited; `--no-limit`
displays all recovered matches. Incomplete scans retain available results and
report coverage/issues separately from display truncation. Resource-ID hits are
explicit candidates, not a resource-access or data-flow proof.

See the [CLI guide](apps/diana/README.md) for all commands, selectors, schemas and
failure behavior.

### GUI

Still in devlopment.

### Project Goals

#### Principles and intent

Diana is an experimental project. For DEX decompilation it borrows the approach of
[Kuna](https://github.com/Noelo-Lab/kuna) and tries to let an LLM improve the
decompiler on its own.

Traditional DEX decompilers chase Java source form. Analysis does not need that.
Diana stays Java-primary, borrows other language features where they say exactly
what happened, and prints low-level source rather than losing semantics. What it
prints is a readable result, not a claim that the output compiles.

#### Goals

1. **Autonomous refinement** — as in Kuna, a dataset should let an LLM keep
   improving the decompiler while we sleep. Diana has no dataset, metric or prompt
   set for that yet.
2. **Agent-first** — latency, accuracy, peak memory and token cost decide design
   tradeoffs, because they bound large-scale reverse-engineering work. Compact JSON
   and the `outline` presentations exist for this reason; see
   [RSS / Speed](#rss--speed).

#### Design principles

- **Phase based** — every stage owns one narrow contract (P0 → method reader → P1 …
  P9), so a change has an obvious home and a failure stays local. Map:
  [phases.md](docs/diana/phases.md).
- **Natural-language spec** — every impactful capability and algorithm is written
  down, at least in part, in [spec/](docs/diana/spec/) next to the stage that owns
  it. A human can audit the design without reading code, and a change lands against
  a written contract instead of a guess.

#### Where things are

| Document                                            | Contents                                                           |
| --------------------------------------------------- | ------------------------------------------------------------------ |
| [phases.md](docs/diana/phases.md)                   | Stage map and current method scheduling.                           |
| [PIPELINE_DESIGN.md](docs/diana/PIPELINE_DESIGN.md) | Approved P0–P9 architecture and stage contracts.                   |
| [spec/](docs/diana/spec/)                           | Product, algorithm and failure contracts per stage.                |
| Engine stage products                               | Inspect the delivered P0–P9 products with the `dbg_shell` example. |

### RSS / Speed

Release CLI with shared output buffering (working tree based on `74edfc4`),
macOS ARM64, Apple M3 Pro, 36 GiB RAM.
Each command starts with the complete APK as input. Query workers: eight where
supported; class-method workers: one. JSON file output is included, except for
raw DEX extraction.

| APK size  | DEX files | Uncompressed DEX size |
| --------- | --------: | --------------------: |
| 198.6 MiB |        41 |             331.4 MiB |
| 233.5 MiB |        14 |             132.8 MiB |

After one warmup per command/input, **cells show median wall seconds / maximum
peak RSS in MiB** across three fresh processes with warm filesystem caches.

| Command          | Scope                            | 198.6 MiB APK | 233.5 MiB APK |
| ---------------- | -------------------------------- | ------------: | ------------: |
| `entries`        | All archive entries              |  <0.01 / 10.4 |   <0.01 / 4.8 |
| `classes`        | All definitions                  |   0.03 / 73.4 |   0.01 / 59.8 |
| `tree`           | All packages and class leaves    |  0.25 / 241.9 |  0.10 / 109.7 |
| `fields`         | All definitions                  |  0.10 / 228.2 |  0.04 / 173.8 |
| `methods`        | All definitions                  |  0.16 / 267.5 |  0.07 / 202.8 |
| `strings`        | All pool values                  |  0.06 / 106.1 |  0.04 / 103.1 |
| `resources`      | Configurations (first APK partial) |   0.07 / 55.6 |   0.01 / 12.9 |
| `get-class`      | One class (19 / 8 methods)       |   0.02 / 30.1 |   0.02 / 24.2 |
| `get-method`     | One method                       |   0.02 / 27.7 |   0.01 / 23.0 |
| `outline-class`  | Same class, no callee expansion  |   0.02 / 30.6 |   0.02 / 24.5 |
| `outline-method` | Same method, no callee expansion |   0.02 / 28.0 |   0.01 / 23.3 |
| `extract`        | One DEX (8.9 / 10.5 MiB)         |   0.01 / 28.1 |   0.02 / 28.7 |
| `xref`           | Regex `^onCreate$`, unlimited     |  1.51 / 521.8 |  0.64 / 232.4 |
| `manifest`       | Full manifest                    |   0.01 / 10.1 |   <0.01 / 4.7 |

Lists are unfiltered. The `xref` row uses `method '^onCreate$' --regex --no-limit`:
case-insensitive regex matching, not fuzzy matching or relevance ranking.
Class/method commands use canonical locators for one nontrivial class/method per
APK, not whole-APK decompilation.
