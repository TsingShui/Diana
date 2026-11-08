"""Fixture lookup and method extraction for LLM review dumps.

The helpers locate a fixture by its unique slug and extract top-level method
bodies. Semantic and readability judgments belong to the written review.
"""

from __future__ import annotations

import re
from pathlib import Path

FIXTURES = Path(__file__).resolve().parent / "fixtures"


def fixture_dir(slug: str):
    """The theme directory that holds `slug`; a missing or doubled slug is an error."""
    matches = sorted(path for path in FIXTURES.glob("*/" + slug) if path.is_dir())
    if len(matches) != 1:
        raise SystemExit(f"{slug}: expected one fixture directory, found {len(matches)}")
    return matches[0]


def fixture_slugs() -> list[str]:
    return sorted(path.name for path in FIXTURES.glob("*/*") if path.is_dir())


KEYWORDS = set(
    """abstract assert boolean break byte case catch char class const continue
    default do double else enum extends final finally float for goto if implements
    import instanceof int interface long native new package private protected public
    return short static strictfp super switch synchronized this throw throws
    transient try void volatile while true false var record yield""".split()
)

METHOD = re.compile(
    r"(?m)^[ \t]*(?:public|private|protected|static|final|abstract|synchronized|native|default)[ \t]+"
    r"[^;{}()\n=]*?(\w+)\s*\("
)
MODIFIERS = (
    "public",
    "private",
    "protected",
    "static",
    "final",
    "abstract",
    "synchronized",
    "native",
    "default",
)


def is_constructor(declaration: str, name: str) -> bool:
    """A constructor is modifiers plus a name, with no return type in between."""
    rest = declaration.strip()
    changed = True
    while changed:
        changed = False
        for modifier in MODIFIERS:
            if rest.startswith(modifier + " "):
                rest = rest[len(modifier) + 1 :].lstrip()
                changed = True
    return rest.rstrip("( \t") in ("", name)


def methods(source: str, stem: str = "") -> dict[str, str]:
    """Method name -> declaration + body, top-level members only."""
    found: dict[str, str] = {}
    for match in METHOD.finditer(source):
        name = match.group(1)
        if name in KEYWORDS or name == stem:
            continue
        matched = match.group(0)
        if is_constructor(matched, name):
            continue
        # Nested types belong to other classes: 4-space members only.
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
