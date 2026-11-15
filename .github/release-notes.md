## Diana CLI 0.2.8

This release fixes misleading command help. The analyzed image set and the size
of an unfiltered listing are now stated where the commands are described. No
analysis behavior changed.

### Help and documentation fixes

- `classes`, `fields`, `methods`, `strings` and `xref` state that they read root
  DEX images only: `classes.dex` and `classes<N>.dex` at the archive root, or the
  whole input for a bare DEX. DEX stored in `assets/`, `res/raw/` or a nested
  archive is not scanned, and the help names `entries` and `extract` as the way to
  analyze it.
- The inaccurate "every DEX in an APK" scope claim is gone from `classes`, `xref`
  and the top-level notes.
- `strings` documents one row per original `string_ids` entry per root, so equal
  values are not merged, and that text output escapes control characters.
- List commands document the 50-row default with its truncation notice, the
  several-MB size of `--no-limit` output on a large APK, and `--filter` or `-o`
  as the intended way to read a large result.
- The top-level help gains a `strings` example.

The changelog records the same corrections.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.2.8/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.2.8
```

Or build the released CLI with Rust 1.93+ and native build tools:

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.2.8 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. macOS and Linux
use `.tar.gz`; Windows uses `.zip`. Linux binaries target musl.

After installing or upgrading the CLI, run `diana skill -i` to install the skill.
Java-like output does not promise compilable Java. The 0.2 series remains
experimental.
