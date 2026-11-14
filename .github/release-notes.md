## Diana CLI 0.2.7

This release adds a minimal Agent Skill for Android reverse engineering and a
command to install it in the user-level Agent Skills directory.

### Agent Skill

```sh
diana skill       # Print SKILL.md to stdout
diana skill -i    # Install the skill (--install also works)
```

The skill directs agents to `diana --help` and `diana <command> --help`, keeping
command documentation in the CLI rather than duplicating it in the skill.

Installation creates `~/.agents/skills/diana/SKILL.md` and any missing parent
directories, then reports the installed path on stderr. It publishes the complete
file atomically and refuses to overwrite any existing file, symlink or directory.
To replace an older skill, review and remove the existing file before installing
again, or use the stdout output to update it manually.

The home directory comes from `HOME` on Unix and `USERPROFILE` on Windows and
must be an absolute path. Skill discovery depends on the agent's support for
`~/.agents/skills`; installing the file does not configure individual agents.

### Install or upgrade Diana

For macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/TsingShui/Diana/diana-v0.2.7/install.sh -o install-diana.sh
sh install-diana.sh diana-v0.2.7
```

Or build the released CLI with Rust 1.93+ and native build tools:

```sh
cargo install --git https://github.com/TsingShui/Diana.git --tag diana-v0.2.7 --locked diana
```

Windows users can extract `diana.exe` from the matching ZIP and add it to PATH.
The archives contain the CLI executable and Apache-2.0 license. macOS and Linux
use `.tar.gz`; Windows uses `.zip`. Linux binaries target musl.

After installing or upgrading the CLI, run `diana skill -i` to install the skill.
Java-like output does not promise compilable Java. The 0.2 series remains
experimental.
