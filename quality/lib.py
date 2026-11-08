"""Shared extraction for the quality tools.

`methods` splits one rendered class into methods. `tokens` normalises a method to
what a structural comparison may look at: comments dropped, string/char literals
reduced to STR/CHR, identifiers reduced to ID, keywords, operators and numeric
literals kept.

Naming is measured separately from structure: `identifiers` returns the source
names, which is the other half of the output quality question.
"""

from __future__ import annotations

import difflib
import re
from pathlib import Path

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


def methods_list(source: str, stem: str = "") -> list[tuple[str, str]]:
    """Every top-level method in source order, keeping duplicate names.

    An obfuscator may map several methods to one short name; the pairing step
    needs the occurrences, not a name-keyed dict.
    """
    found: list[tuple[str, str]] = []
    for match in METHOD.finditer(source):
        name = match.group(1)
        if name in KEYWORDS or name == stem:
            continue
        matched = match.group(0)
        if is_constructor(matched, name):
            continue
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
                    found.append((name, source[start : index + 1].rstrip()))
                    break
    return found


def _short_type(text: str) -> str:
    """`java.lang.String[]` -> `String[]`, `Map<K,V>` -> `Map`."""
    text = text.split("<")[0].strip()
    return text.split(".")[-1]


def mapping_signatures(mapping: Path | None, text: str | None = None) -> dict[str, list[tuple[str, tuple[str, ...]]]]:
    """Original name -> [(obfuscated name, parameter type names)], from a mapping."""
    if text is None:
        text = mapping.read_text() if mapping is not None else ""
    out: dict[str, list[tuple[str, tuple[str, ...]]]] = {}
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or " -> " not in stripped:
            continue
        source, _, target = stripped.partition(" -> ")
        if "(" not in source:
            continue
        signature, _, _ = source.partition("(")
        parameters = source.split("(", 1)[1].rsplit(")", 1)[0].strip()
        types = tuple(_short_type(part) for part in parameters.split(",")) if parameters else ()
        out.setdefault(signature.split()[-1], []).append((target.rstrip(":"), types))
    return out


def body_parameters(body: str) -> tuple[str, ...]:
    """Parameter type names from a rendered method's declaration line."""
    header = body.split("{", 1)[0]
    inside = header.split("(", 1)[1].rsplit(")", 1)[0].strip()
    if not inside:
        return ()
    parts = []
    for piece in inside.split(","):
        words = piece.split()
        if len(words) >= 2:
            parts.append(_short_type(" ".join(words[:-1])))
        elif words:
            parts.append(_short_type(words[0]))
    return tuple(parts)


def align(
    reference: list[tuple[str, str]],
    produced: list[tuple[str, str]],
    mapping: dict[str, set[str]] | None = None,
    signatures: dict[str, list[tuple[str, tuple[str, ...]]]] | None = None,
) -> list[tuple[str, str, str, float]]:
    """Pair reference methods with produced bodies, each used at most once.

    Without a mapping the names must agree. With a mapping the obfuscated names
    are the candidates; when several methods share one name (ProGuard reuses short
    names), the pairing is resolved by body similarity, best pair first.
    """
    pairs: list[tuple[float, int, int]] = []
    for left, (name, body) in enumerate(reference):
        wanted = (signatures or {}).get(name)
        if wanted:
            candidates = []
            for index, (out_name, out_body) in enumerate(produced):
                actual = body_parameters(out_body)
                for obfuscated, types in wanted:
                    if out_name == obfuscated and (not types or actual == types):
                        candidates.append(index)
                        break
        elif mapping:
            candidates = [
                index
                for index, (out_name, _) in enumerate(produced)
                if out_name in mapping.get(name, ())
            ]
        else:
            candidates = [
                index for index, (out_name, _) in enumerate(produced) if out_name == name
            ]
        left_tokens = tokens(body)
        for index in candidates:
            right_tokens = tokens(produced[index][1])
            ratio = difflib.SequenceMatcher(None, left_tokens, right_tokens).ratio()
            pairs.append((ratio, left, index))
    pairs.sort(reverse=True)
    used_left: set[int] = set()
    used_right: set[int] = set()
    result: dict[int, tuple[str, str, float]] = {}
    for ratio, left, right in pairs:
        if left in used_left or right in used_right:
            continue
        used_left.add(left)
        used_right.add(right)
        result[left] = (produced[right][0], produced[right][1], ratio)
    return [
        (name, *result[index]) if index in result else (name, "", "", 0.0)
        for index, (name, _) in enumerate(reference)
    ]


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
