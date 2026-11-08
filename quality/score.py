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

A fixture that carries a `mapping.txt` was produced by an obfuscator, so names no
longer match: methods are then paired by mapped name and, when that is ambiguous
(ProGuard reuses short names), by body similarity. Naming is reported as `obf`
because the obfuscator destroyed it, not the decompiler.

`quality/fixtures/<slug>/review.md` stays authoritative; this script is a
measurement, not a verdict.
"""

from __future__ import annotations

import argparse
import difflib
import re
import statistics
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "quality" / "fixtures"


def original_class(java: Path) -> str:
    text = java.read_text()
    package = re.search(r"(?m)^package\s+([\w.]+)\s*;?", text)
    return (package.group(1) + "." if package else "") + java.stem


def selector(java: Path) -> str:
    name = original_class(java)
    return "L" + name.replace(".", "/") + ";"


def obfuscated_selector(java: Path, mapping: Path) -> str | None:
    """The obfuscated class descriptor, read from the mapping's class lines."""
    original = original_class(java)
    for line in mapping.read_text().splitlines():
        if "(" in line or " -> " not in line:
            continue
        source, _, target = line.partition(" -> ")
        if source.strip() == original:
            return "L" + target.strip().rstrip(":").replace(".", "/") + ";"
    return None


def mapped_names(mapping: Path) -> dict[str, set[str]]:
    """Original method name -> obfuscated names, from an R8/ProGuard mapping."""
    names: dict[str, set[str]] = {}
    for line in mapping.read_text().splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or " -> " not in stripped:
            continue
        source, _, target = stripped.partition(" -> ")
        if "(" not in source:
            continue
        original = source.split("(")[0].split()[-1]
        names.setdefault(original, set()).add(target.rstrip(":"))
    return names


def score_fixture(binary: str, slug: str) -> tuple[list[float], list[float]] | None:
    directory = FIXTURES / slug
    java_files = sorted(directory.glob("*.java"))
    kotlin_files = sorted(directory.glob("*.kt"))
    dex = directory / "classes.dex"
    if not dex.is_file():
        print(f"{slug}: need a classes.dex (run quality/build.sh)", file=sys.stderr)
        return None
    if kotlin_files:
        print(f"\n== {slug}\n  Kotlin fixture: review only, no score (semantics judged by reading)")
        return None
    if not java_files:
        print(f"{slug}: no reference source", file=sys.stderr)
        return None

    java = java_files[0]
    mapping_file = directory / "mapping.txt"
    target = selector(java)
    if mapping_file.is_file():
        target = obfuscated_selector(java, mapping_file) or target
    result = subprocess.run(
        [binary, "get-class", str(dex), target], capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"{slug}: {result.stderr.strip()}", file=sys.stderr)
        return None

    reference_list = lib.methods_list(java.read_text(), java.stem)
    produced_list = lib.methods_list(result.stdout)
    mapping = mapped_names(mapping_file) if mapping_file.is_file() else None
    signatures = lib.mapping_signatures(mapping_file) if mapping_file.is_file() else None
    aligned = {
        name: (body, ratio)
        for name, _, body, ratio in lib.align(
            reference_list, produced_list, mapping, signatures
        )
    }

    print(f"\n== {slug}{'  (obfuscated)' if mapping else ''}")
    columns = f"{'method':24} {'structure':>10} {'naming':>7}  first divergence"
    print(columns)
    print("-" * 80)
    structure: list[float] = []
    naming: list[float] = []
    for name, source in reference_list:
        ref_tokens = lib.tokens(source)
        aligned_body, aligned_ratio = aligned.get(name, ("", 0.0))
        candidates = [aligned_body] if aligned_body else []
        if not candidates:
            print(f"{name:20} {'-':>10} {'-':>7}  missing from the output")
            continue
        best = candidates[0]
        out_tokens = lib.tokens(best)
        matcher = difflib.SequenceMatcher(None, ref_tokens, out_tokens)
        value = aligned_ratio
        structure.append(value)
        divergence = "-"
        for tag, i1, i2, j1, j2 in matcher.get_opcodes():
            if tag == "equal":
                continue
            divergence = (
                f"{tag}: ref={' '.join(ref_tokens[i1:i2][:6]) or '-'} "
                f"ours={' '.join(out_tokens[j1:j2][:6]) or '-'}"
            )
            break
        if mapping:
            print(f"{name:24} {100 * value:9.1f}% {'obf':>7}  {divergence}")
        else:
            names = lib.identifiers(source)
            name_value = len([word for word in names if word in lib.identifiers(best)]) / max(
                1, len(names)
            )
            naming.append(name_value)
            print(f"{name:24} {100 * value:9.1f}% {100 * name_value:6.0f}%  {divergence}")
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
    naming_note = (
        f"   naming {100 * statistics.mean(all_naming):.1f}%"
        if all_naming
        else "   naming n/a"
    )
    print(
        f"methods {len(all_structure)}   structure "
        f"{100 * statistics.mean(all_structure):.1f}%{naming_note}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
