"""Shared extraction for the quality tools.

`methods` splits one rendered class into methods. `tokens` normalises a method to
what a structural comparison may look at: comments dropped, string/char literals
reduced to STR/CHR, identifiers reduced to ID, keywords, operators and numeric
literals kept.

Naming is measured separately from structure: `identifiers` returns the source
names, which is the other half of the output quality question.
"""

from __future__ import annotations

import re

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
STRING = re.compile(r'"(?:\\.|[^"\\])*"')
CHAR = re.compile(r"'(?:\\.|[^'\\])*'")
COMMENT = re.compile(r"/\*.*?\*/|//[^\n]*", re.S)
WORD = re.compile(r"[A-Za-z_$][\w$]*|\d+\.\d+|\d+|[^\sA-Za-z0-9_]")


def methods(source: str, stem: str = "") -> dict[str, str]:
    """Method name -> declaration + body, top-level members only."""
    found: dict[str, str] = {}
    for match in METHOD.finditer(source):
        name = match.group(1)
        if name in KEYWORDS or name == stem:
            continue
        matched = match.group(0)
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


def tokens(source: str) -> list[str]:
    """Structural token stream: identifiers are ID, literals keep their value."""
    text = COMMENT.sub("", source)
    text = STRING.sub("STR", text)
    text = CHAR.sub("CHR", text)
    out: list[str] = []
    for word in WORD.findall(text):
        if word in KEYWORDS:
            out.append(word)
        elif re.fullmatch(r"[A-Za-z_$][\w$]*", word):
            out.append("ID")
        else:
            out.append(word)
    return out


def identifiers(source: str) -> set[str]:
    return {
        word
        for word in re.findall(r"[A-Za-z_$][\w$]*", STRING.sub("", CHAR.sub("", source)))
        if word not in KEYWORDS and len(word) > 1
    }
