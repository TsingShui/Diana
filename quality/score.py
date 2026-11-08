#!/usr/bin/env python3
"""Score how much of the reference structure each fixture method recovered.

The Java reference is the expectation: no hand-written predicates. For every
fixture method the script reports

* **structure** - similarity of the normalised token streams (identifiers -> ID,
  string/char literals -> STR/CHR, comments dropped, keywords/operators/numbers
  kept). 100% means the decompiled body is the same sequence of constructs,
  operators and literals as the reference;
* **naming** - share of the reference identifiers that appear in the output;
* **first divergence** - the first token replacement/insertion, which is what one
  reads to see *which* structure was lost.

It is a measurement, not a verdict: a 100% match can still be wrong (operands
swapped without changing tokens) and a low match can be an equivalent shape. The
written review in `quality/fixtures/<slug>/review.md` stays authoritative.

    python3 quality/score.py            # every fixture
    python3 quality/score.py --slug branch-if2
"""

from __future__ import annotations

import argparse
import difflib
import statistics
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "quality" / "fixtures"


def selector(java: Path) -> str:
    import re

    text = java.read_text()
    package = re.search(r"(?m)^package\s+([\w.]+)\s*;", text)
    stem = java.stem
    if package:
        return "L" + package.group(1).replace(".", "/") + "/" + stem + ";"
    return "L" + stem + ";"


def score_fixture(binary: str, slug: str) -> tuple[list[float], list[float]] | None:
    directory = FIXTURES / slug
    java_files = sorted(directory.glob("*.java"))
    dex = directory / "classes.dex"
    if not java_files or not dex.is_file():
        print(f"{slug}: need a .java and classes.dex", file=sys.stderr)
        return None
    java = java_files[0]
    result = subprocess.run(
        [binary, "get-class", str(dex), selector(java)], capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"{slug}: {result.stderr.strip()}", file=sys.stderr)
        return None

    reference = lib.methods(java.read_text(), java.stem)
    produced = lib.methods(result.stdout)
    print(f"\n== {slug}")
    print(f"{'method':22} {'structure':>10} {'naming':>7}  first divergence")
    print("-" * 80)
    structure: list[float] = []
    naming: list[float] = []
    for name, source in reference.items():
        output = produced.get(name)
        if output is None:
            print(f"{name:22} {'-':>10} {'-':>7}  missing from the output")
            continue
        left, right = lib.tokens(source), lib.tokens(output)
        matcher = difflib.SequenceMatcher(None, left, right)
        value = matcher.ratio()
        names = lib.identifiers(source)
        name_value = len([word for word in names if word in lib.identifiers(output)]) / max(
            1, len(names)
        )
        structure.append(value)
        naming.append(name_value)
        divergence = "-"
        for tag, i1, i2, j1, j2 in matcher.get_opcodes():
            if tag == "equal":
                continue
            divergence = (
                f"{tag}: ref={' '.join(left[i1:i2][:6]) or '-'} "
                f"ours={' '.join(right[j1:j2][:6]) or '-'}"
            )
            break
        print(f"{name:22} {100 * value:9.1f}% {100 * name_value:6.0f}%  {divergence}")
    return structure, naming


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default=str(ROOT / "target/release/diana"))
    parser.add_argument("--slug")
    args = parser.parse_args()

    slugs = (
        [args.slug]
        if args.slug
        else sorted(path.name for path in FIXTURES.iterdir() if path.is_dir())
    )
    all_structure: list[float] = []
    all_naming: list[float] = []
    for slug in slugs:
        scored = score_fixture(args.binary, slug)
        if scored is None:
            continue
        structure, naming = scored
        all_structure += structure
        all_naming += naming

    if not all_structure:
        print("no fixture scored", file=sys.stderr)
        return 1
    print("\n" + "=" * 80)
    print(
        f"methods {len(all_structure)}   structure {100 * statistics.mean(all_structure):.1f}%"
        f"   naming {100 * statistics.mean(all_naming):.1f}%"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
