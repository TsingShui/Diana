#!/usr/bin/env python3
"""Review dump: Java reference next to Diana's output, one method at a time.

This produces the artifact an LLM (or a person) reads when judging decompilation
quality. It deliberately does not score anything: the judgement is about whether
the decompiled method means the same thing, keeps the original facts, and reads
like the reference - which no string metric decides.

    python3 quality/dump.py                 # every fixture -> .cache/review/<slug>.md
    python3 quality/dump.py --slug complex  # one fixture, printed to stdout
    python3 quality/dump.py --slug complex -o review.md
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
QUALITY = ROOT / "quality"
KEYWORDS = set(
    """abstract assert boolean break byte case catch char class const continue
    default do double else enum extends final finally float for goto if implements
    import instanceof int interface long native new package private protected public
    return short static strictfp super switch synchronized this throw throws
    transient try void volatile while true false var record yield""".split()
)
METHOD = re.compile(
    r"(?m)^[ \t]*(?:public|private|protected|static|final|abstract|synchronized|native)[ \t]+"
    r"[^;{}()\n=]*?(\w+)\s*\("
)


def methods(source: str, stem: str = "") -> dict[str, str]:
    """Top-level members of the fixture class: name -> declaration + body."""
    found: dict[str, str] = {}
    for match in METHOD.finditer(source):
        name = match.group(1)
        if name in KEYWORDS or name == stem:
            continue
        matched = match.group(0)
        if len(matched) - len(matched.lstrip()) > 4:
            continue
        start = source.rfind("\n", 0, match.start()) + 1
        body = source.find("{", match.end())
        semi = source.find(";", match.end())
        if body < 0 or (semi >= 0 and semi < body):
            continue
        depth = 0
        for index in range(body, len(source)):
            char = source[index]
            if char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    found.setdefault(name, source[start : index + 1].rstrip())
                    break
    return found


def selector_for(java: Path) -> str:
    text = java.read_text()
    package = re.search(r"(?m)^package\s+([\w.]+)\s*;", text)
    stem = java.stem
    if package:
        return "L" + package.group(1).replace(".", "/") + "/" + stem + ";"
    return "L" + stem + ";"


def dump(binary: str, slug: str) -> str:
    directory = QUALITY / "fixtures" / slug
    java_files = sorted(directory.glob("*.java"))
    dex = directory / "classes.dex"
    if not java_files or not dex.is_file():
        raise SystemExit(f"{slug}: need a .java and classes.dex (run quality/build.sh)")
    java = java_files[0]
    selector = selector_for(java)
    result = subprocess.run(
        [binary, "get-class", str(dex), selector], capture_output=True, text=True
    )
    if result.returncode != 0:
        raise SystemExit(f"{slug}: {result.stderr.strip()}")

    reference = methods(java.read_text(), java.stem)
    produced = methods(result.stdout)
    lines = [
        f"# Review: {slug}",
        "",
        f"- fixture: `quality/fixtures/{slug}/`",
        f"- selector: `{selector}`",
        f"- binary: `{binary}`",
        "",
        "Read each pair below. For every method decide:",
        "",
        "1. **semantics** - does the decompiled method do the same thing (values,",
        "   effects, call order, exceptions, locks)?",
        "2. **facts** - are the original calls, constants and member identities kept?",
        "3. **shape** - are the reference constructs recovered (`for`, enhanced `for`,",
        "   `do/while`, ternary, `&&`/`||`, `switch`, `try (`, `synchronized`, `+=`)?",
        "4. **notation** - does anything only a decompiler would print appear in the body",
        "   (`test {`, `graph<`, `goto`, `capture_exception`, `via `, `dex_flags<`)?",
        "",
        f"Write the findings as a new round in `quality/fixtures/{slug}/review.md`; do not score them.",
        "",
    ]
    for name, source in reference.items():
        lines += [f"## `{name}`", "", "Reference (Java):", "", "```java", source, "```", ""]
        actual = produced.get(name)
        if actual is None:
            lines += ["Diana: **method missing from the output**", ""]
            continue
        lines += ["Diana:", "", "```java", actual, "```", ""]
    extras = sorted(set(produced) - set(reference))
    if extras:
        lines += ["## Methods only in the output", "", ", ".join(f"`{name}`" for name in extras), ""]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default=str(ROOT / "target/release/diana"))
    parser.add_argument("--slug")
    parser.add_argument("-o", "--output")
    args = parser.parse_args()

    slugs = (
        [args.slug]
        if args.slug
        else sorted(path.name for path in (QUALITY / "fixtures").iterdir() if path.is_dir())
    )
    out_dir = ROOT / ".cache/review"
    for slug in slugs:
        text = dump(args.binary, slug)
        if args.output:
            target = Path(args.output)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text)
            print(f"wrote {target}")
        elif args.slug:
            print(text)
        else:
            out_dir.mkdir(parents=True, exist_ok=True)
            target = out_dir / f"{slug}.md"
            target.write_text(text)
            print(f"wrote {target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
