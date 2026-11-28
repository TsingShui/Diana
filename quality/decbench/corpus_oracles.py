"""Compare independently selected source and original call-site facts for APK methods."""

from __future__ import annotations

from collections import Counter
from contextlib import ExitStack
from dataclasses import dataclass, replace
from hashlib import sha256
import json
from pathlib import Path, PurePosixPath
import re
from tarfile import TarError, open as open_tar
from zipfile import BadZipFile, ZIP_DEFLATED, ZIP_STORED, ZipFile
from zlib import error as DeflateError

from dex_calls import LOCATOR, INVOKES, MAX_CODE_UNITS, WIDTHS, method_code, method_ref
from dex_truth import APK_FILE_BYTES, APK_ROOT, APK_ROOT_BYTES, Dex, U16, U32
from evidence import file_digest

BYTE_LIMIT = 4 * 1024 * 1024
SOURCE_ARCHIVE_LIMIT = 64 * 1024 * 1024
SOURCE_SCAN_LIMIT = 256 * 1024 * 1024
MAX_ARCHIVE_ENTRIES = 50_000
MAX_METHODS = 512
MAX_CALL_SITES = 2_048
MAX_CONSTRUCTOR_HEAD = 256
SHA256 = re.compile(r"[a-f0-9]{64}")
REVISION = re.compile(r"[a-f0-9]{40}")
IDENTIFIER = re.compile(r"[A-Za-z_$][\w$]*")
DESCRIPTOR = re.compile(r"\[*(?:[ZBSCIJFDV]|L[^.;\[\]()]+;)")
# DEX name characters include Unicode space separators from version 040.
DEX_SIMPLE_NAME = (
    r"[A-Za-z0-9 $_\u00a0-\u1fff\u2000-\u200a\u2010-\u2027\u202f"
    r"\u2030-\ud7ff\ue000-\uffef\U00010000-\U0010ffff-]+"
)
OBJECT_TYPE = rf"L{DEX_SIMPLE_NAME}(?:/{DEX_SIMPLE_NAME})*;"
CALL_OWNER = re.compile(rf"(?:{OBJECT_TYPE}|\[{{1,255}}(?:[ZBSCIJFD]|{OBJECT_TYPE}))")
INIT_MEMBER = rf"(?P<init_owner>{OBJECT_TYPE})-><init>(?P<init_descriptor>\((?:\[{{0,255}}(?:[ZBSCIJFD]|{OBJECT_TYPE}))*\)V)"
TOKEN = re.compile(
    r"""(?P<skip>\s+|//[^\n]*|/\*[\s\S]*?\*/)|"(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*'|"""
    + INIT_MEMBER + r"|[\w$]+|[^\s]"
)
KEYWORDS = frozenset({"if", "for", "while", "switch", "try", "catch", "synchronized", "return", "throw"})
BRACKETS = dict(zip("([{", ")]}"))
PRIMITIVE_NAMES = {"Z": "boolean", "B": "byte", "S": "short", "C": "char", "I": "int",
                   "J": "long", "F": "float", "D": "double", "V": "void"}
EXTRA_WIDTHS = {
    0x1d: 1, 0x1e: 1, 0x26: 3, 0x27: 1, 0x28: 1, 0x29: 2,
    0x2a: 3, 0x2b: 3, 0x2c: 3, **dict.fromkeys(range(0x32, 0x3e), 2),
    0xfa: 4, 0xfb: 4, 0xfc: 3, 0xfd: 3,
}


class Unavailable(ValueError):
    """The selected representation does not establish the required fact."""


def logical_path(value):
    path = PurePosixPath(value)
    if not value or path.is_absolute() or ".." in path.parts or str(path) != value:
        raise ValueError(f"expected a logical relative path: {value!r}")
    return value


def parameter_types(descriptor):
    arguments, separator, result = descriptor.removeprefix("(").partition(")")
    if not descriptor.startswith("(") or not separator or not DESCRIPTOR.fullmatch(result):
        raise ValueError(f"invalid method prototype: {descriptor!r}")
    types = DESCRIPTOR.findall(arguments)
    if "".join(types) != arguments or any(kind.endswith("V") for kind in types):
        raise ValueError(f"invalid parameter descriptor: {descriptor!r}")
    return types


def member_parts(member):
    owner, arrow, declaration = member.partition("->")
    name, parenthesis, rest = declaration.partition("(")
    if not arrow or not re.fullmatch(r"L[^.;\[\]()]+;", owner) or not IDENTIFIER.fullmatch(name) or not parenthesis:
        raise ValueError(f"invalid selected method identity: {member!r}")
    return owner, name, parameter_types(parenthesis + rest)


def exact_type_words(descriptor):
    base = descriptor.lstrip("[")
    name = PRIMITIVE_NAMES.get(base)
    if name is not None:
        tokens = (name,)
    else:
        qualified = base[1:-1].split("/")
        tokens = tuple(word for item in qualified for word in (item, "."))[:-1]
    return tokens + ("[", "]") * descriptor.count("[")


def load_cases(path: Path) -> dict:
    with path.open("rb") as stream:
        contents = stream.read(BYTE_LIMIT + 1)
    if len(contents) > BYTE_LIMIT:
        raise ValueError(f"{path}: case manifest exceeds the supported bound")
    return validate_cases(json.loads(contents), path)


def validate_cases(result: dict, path: Path | str, *, allow_empty: bool = False) -> dict:
    """Validate manifests from checked-in cases and generated discovery alike."""
    if not isinstance(result, dict) or result.get("schema_version") != 1:
        raise ValueError(f"{path}: expected a version-1 case manifest")
    for key, pattern in (("input_sha256", SHA256), ("source_revision", REVISION)):
        if not isinstance(result.get(key), str) or not pattern.fullmatch(result[key]):
            raise ValueError(f"{path}: invalid {key}")
    if "revision_entry" in result:
        logical_path(result["revision_entry"])
    methods = result.get("methods")
    minimum = 0 if allow_empty else 1
    if not isinstance(methods, list) or not minimum <= len(methods) <= MAX_METHODS:
        raise ValueError(f"{path}: expected a bounded list of selected methods")
    locators = set()
    for method in methods:
        if not isinstance(method, dict) or not isinstance(method.get("member"), str):
            raise ValueError(f"{path}: each case must be an object with an original method")
        at = method.get("at")
        if not isinstance(at, str) or at in locators or not LOCATOR.fullmatch(at):
            raise ValueError(f"{path}: invalid or repeated method locator {at!r}")
        locators.add(at)
        member_parts(method["member"])
        source = method.get("source")
        if not isinstance(source, dict) or not isinstance(source.get("path"), str):
            raise ValueError(f"{path}: missing source path for {at}")
        logical_path(source["path"])
        if (not isinstance(source.get("sha256"), str) or not SHA256.fullmatch(source["sha256"])
                or any(type(source.get(field)) is not int for field in ("start_line", "end_line"))
                or not 1 <= source["start_line"] <= source["end_line"]):
            raise ValueError(f"{path}: invalid source fingerprint or lines for {at}")
        obligations = method.get("obligations")
        if not isinstance(obligations, list) or not obligations:
            raise ValueError(f"{path}: missing obligations for {at}")
        ids = set()
        for obligation in obligations:
            if not isinstance(obligation, dict):
                raise ValueError(f"{path}: invalid obligation for {at}")
            kind = obligation.get("kind")
            identifier = obligation.get("id")
            if (not isinstance(identifier, str) or not IDENTIFIER.fullmatch(identifier.replace("-", "_"))
                    or identifier in ids or kind not in ("call_sequence", "call_presence", "char_arguments",
                                                         "argument_values", "field_access")
                    or not isinstance(obligation.get("description"), str)):
                raise ValueError(f"{path}: invalid obligation for {at}")
            ids.add(identifier)
            if kind == "field_access":
                continue
            targets = obligation.get("targets")
            if not isinstance(targets, list) or not targets:
                raise ValueError(f"{path}: missing call targets in {at}/{identifier}")
            keys = set()
            for target in targets:
                if (not isinstance(target, dict) or not isinstance(target.get("owner"), str)
                        or not CALL_OWNER.fullmatch(target["owner"])):
                    raise ValueError(f"{path}: invalid call owner in {at}/{identifier}")
                if not all(isinstance(target.get(field), str) and target[field] for field in ("name", "descriptor")):
                    raise ValueError(f"{path}: invalid call target in {at}/{identifier}")
                key = target_key(target)
                if key in keys:
                    raise ValueError(f"{path}: ambiguous call name and arity in {at}/{identifier}")
                keys.add(key)
                if kind == "char_arguments" and "C" not in parameter_types(target["descriptor"]):
                    raise ValueError(f"{path}: char argument check needs a char parameter")
                if kind == "argument_values":
                    indexes = obligation.get("argument_indexes")
                    if (not isinstance(indexes, list) or not indexes or any(
                        type(index) is not int or not 0 <= index < key[1] for index in indexes
                    )):
                        raise ValueError(f"{path}: invalid pinned argument indexes")
    return result


@dataclass(frozen=True)
class Token:
    text: str
    line: int


@dataclass(frozen=True)
class Site:
    name: str
    arguments: tuple[tuple[str, ...], ...]
    line: int
    choices: tuple[tuple[int, bool], ...] = ()
    constructor: str | None = None
    direct_target: tuple[str, str] | None = None

    @property
    def key(self):
        return self.name, len(self.arguments)


class View:
    def __init__(self, text, first_line=1, *, locate_tokens=False):
        self.tokens = []
        self.offsets = [] if locate_tokens else None
        self.ends = {}
        self.scopes = []
        direct_members = []
        stack = []
        scopes = []
        line = first_line
        for match in TOKEN.finditer(text):
            value = match[0]
            if not match["skip"]:
                if match["init_owner"]:
                    direct_members.append((len(self.tokens), match["init_owner"], match["init_descriptor"]))
                if self.offsets is not None:
                    self.offsets.append(match.span())
                self.scopes.append(scopes[-1] if scopes else None)
                if value == "{":
                    scopes.append(len(self.tokens))
                elif value == "}" and scopes:
                    scopes.pop()
                if value in BRACKETS:
                    stack.append((value, len(self.tokens)))
                elif value in BRACKETS.values():
                    if not stack or BRACKETS[stack[-1][0]] != value:
                        raise Unavailable("unbalanced brackets in selected text")
                    _, start = stack.pop()
                    self.ends[start] = len(self.tokens)
                self.tokens.append(Token(value, line))
            line += value.count("\n")
        if stack:
            raise Unavailable("unbalanced brackets in selected text")
        self.words = tuple(token.text for token in self.tokens)
        self.constructors = {}
        self.constructor_heads = {}
        self.overlong_constructor_heads = []
        for index, word in enumerate(self.words):
            if word != "new":
                continue
            try:
                head = self.constructor_head(index)
            except Unavailable:
                self.overlong_constructor_heads.append(index)
                continue
            if head is not None:
                opening, name, owner = head
                self.constructors[opening] = (name, owner)
                self.constructor_heads[index] = opening
        self.direct_initializers = self.read_direct_initializers(direct_members)

    def read_direct_initializers(self, members):
        if not members:
            return ()
        result = []
        openings = {closing: opening for opening, closing in self.ends.items() if self.words[opening] != "{"}
        for member, owner, descriptor in members:
            opening = member + 2
            if (self.words[member - 1:member] != (",",) or self.words[member + 1:opening + 1] != (",", "[")
                    or opening not in self.ends):
                raise Unavailable("invalid explicit initializer operands")
            closing = self.ends[opening]
            if self.words[closing + 1:closing + 2] != (";",):
                raise Unavailable("missing explicit initializer statement boundary")
            cursor = member - 2
            while cursor >= 0 and self.words[cursor] not in (";", "{", "}"):
                cursor = openings.get(cursor, cursor) - 1
            start = cursor + 1
            if self.words[start:start + 3] != ("init", "-", "direct") or start + 3 >= member - 1:
                raise Unavailable("explicit initializer has no receiver")
            arguments = self.arguments(opening)
            target = {"name": "<init>", "owner": owner, "descriptor": descriptor}
            name, arity = target_key(target)
            if len(arguments) != arity or any(not argument for argument in arguments):
                raise Unavailable("explicit initializer arity differs from its descriptor")
            site = Site(name, arguments, self.tokens[start].line, direct_target=(owner, descriptor))
            result.append((start, closing, site))
        return result

    def constructor_head(self, start):
        """Read the erased name and argument opening of a displayed `new` expression."""
        stop = min(len(self.words), start + MAX_CONSTRUCTOR_HEAD)
        cursor = start + 1
        if cursor < stop and self.words[cursor] == "<":
            cursor = self.constructor_type_arguments(cursor, stop)
        head = self.constructor_named_type(cursor, stop, diamond=True)
        if head is None:
            return None
        cursor, names = head
        if cursor >= stop:
            return self.constructor_limit(stop)
        if self.words[cursor] == "(":
            return cursor, names[-1], ".".join(names)
        return None

    def constructor_named_type(self, cursor, stop, *, diamond=False):
        names = []
        if cursor is None:
            return None
        while cursor < stop:
            name = self.words[cursor]
            if (not IDENTIFIER.fullmatch(name)
                    or name in KEYWORDS or name in ("new", "extends", "super", "null", "true", "false")):
                return None
            names.append(name)
            cursor += 1
            if cursor < stop and self.words[cursor] == "<":
                cursor = self.constructor_type_arguments(cursor, stop, diamond=diamond)
            if cursor is None:
                return None
            if cursor >= stop or self.words[cursor] != ".":
                return cursor, names
            cursor += 1
        return self.constructor_limit(stop)

    def constructor_type_arguments(self, opening, stop, *, diamond=False):
        """Consume type arguments only within a bounded head anchored by `new`."""
        cursor = opening + 1
        if diamond and cursor < stop and self.words[cursor] == ">":
            return cursor + 1
        while cursor < stop:
            has_type = True
            if self.words[cursor] == "?":
                cursor += 1
                has_type = cursor < stop and self.words[cursor] in ("extends", "super")
                if has_type:
                    cursor += 1
            if has_type:
                item = self.constructor_named_type(cursor, stop)
                if item is None:
                    return None
                cursor, _ = item
                while cursor + 1 < stop and self.words[cursor:cursor + 2] == ("[", "]"):
                    cursor += 2
            if cursor >= stop:
                break
            if self.words[cursor] == ">":
                return cursor + 1
            if self.words[cursor] != ",":
                return None
            cursor += 1
        return self.constructor_limit(stop)

    def constructor_limit(self, stop):
        if stop < len(self.words):
            raise Unavailable("constructor head exceeds its token bound")

    def arguments(self, opening):
        closing = self.ends[opening]
        if closing == opening + 1:
            return ()
        result = []
        start = cursor = opening + 1
        while cursor < closing:
            token = self.words[cursor]
            if token == ",":
                result.append(self.words[start:cursor])
                start = cursor + 1
            cursor = self.constructor_heads.get(cursor, cursor)
            cursor = self.ends.get(cursor, cursor) + 1
        result.append(self.words[start:closing])
        return tuple(result)

    def method_layouts(self, member):
        _, name, parameters = member_parts(member)
        for opening, closing in self.ends.items():
            if not opening or self.words[opening - 1:opening + 1] != (name, "("):
                continue
            arguments = self.arguments(opening)
            if len(arguments) != len(parameters):
                continue
            if any(not parameter_matches(argument, kind) for argument, kind in zip(arguments, parameters)):
                continue
            body = closing + 1
            if body < len(self.words) and self.words[body] == "throws":
                while body < len(self.words) and self.words[body] not in ("{", ";", "("):
                    body += 1
            if body < len(self.words) and self.words[body] in ("{", ";"):
                yield opening - 1, body

    def method_body(self, member):
        bodies = [(body, self.ends[body]) for _, body in self.method_layouts(member) if self.words[body] == "{"]
        if len(bodies) != 1:
            raise Unavailable("could not select one displayed body by method name and parameter types")
        return bodies[0]

    def declared_without_body(self, member, owner_scope):
        _, _, parameters = member_parts(member)
        result = member.rsplit(")", 1)[1]
        trailing_type = exact_type_words(result)
        declarations = []
        for name, ending in self.method_layouts(member):
            if self.scopes[name] != owner_scope:
                continue
            arguments = self.arguments(name + 1)
            if any(arg[:-1] not in (kind, ("final", *kind))
                   for arg, kind in zip(arguments, map(exact_type_words, parameters))):
                continue
            start_of_type = name - len(trailing_type)
            if (start_of_type >= 0 and self.words[start_of_type:name] == trailing_type
                    and (start_of_type == 0 or self.words[start_of_type - 1] != ".")):
                declarations.append(ending)
        return len(declarations) == 1 and self.words[declarations[0]] == ";"

    def choices(self, bounds):
        start, stop = bounds
        alternatives = []
        for index in range(start + 1, stop):
            if self.words[index:index + 2] != ("if", "("):
                continue
            consequent = self.ends[index + 1] + 1
            if consequent not in self.ends or self.words[consequent] != "{":
                continue
            alternate = self.ends[consequent] + 2
            if (self.words[alternate - 1:alternate + 1] != ("else", "{")
                    or alternate not in self.ends):
                continue
            alternatives.append((consequent, self.ends[consequent], alternate, self.ends[alternate]))
        return alternatives

    def explicit_transfers(self, bounds):
        start, stop = bounds
        return [
            {"line": self.tokens[index].line, "target": self.words[index + 1]}
            for index in range(start + 1, stop - 2)
            if self.words[index] == "goto" and IDENTIFIER.fullmatch(self.words[index + 1])
            and self.words[index + 2] == ";"
        ]

    def calls(self, bounds):
        start, stop = bounds
        if any(start < index < stop for index in self.overlong_constructor_heads):
            raise Unavailable("constructor head exceeds its token bound")
        branches = self.choices(bounds)
        def choices_at(index):
            return tuple((left, left < index < left_end) for left, left_end, right, right_end in branches
                         if left < index < left_end or right < index < right_end)

        result = [(closing, replace(site, choices=choices_at(opening)))
                  for opening, closing, site in self.direct_initializers if start < opening < closing < stop]
        if len(result) > MAX_CALL_SITES:
            raise Unavailable("the selected display exceeds the call-site bound")
        for opening, closing in self.ends.items():
            if not start < opening < closing < stop or self.words[opening] != "(":
                continue
            if len(result) >= MAX_CALL_SITES:
                raise Unavailable("the selected display exceeds the call-site bound")
            constructor = None
            if opening in self.constructors:
                name, constructor = self.constructors[opening]
                name = "new " + name
            else:
                name = self.words[opening - 1]
                if not IDENTIFIER.fullmatch(name) or name in KEYWORDS:
                    continue
            site = Site(name, self.arguments(opening), self.tokens[opening - 1].line, choices_at(opening), constructor)
            result.append((closing, site))
        return [site for _, site in sorted(result, key=lambda item: item[0])]

    def argument_type(self, argument, bounds, member):
        if len(argument) > 3 and argument[:3] in (("(", "char", ")"), ("(", "int", ")")):
            return argument[1]
        if len(argument) == 1 and re.fullmatch(r"""'(?:\\(?:[btnfrs"'\\]|[0-7]{1,3}|u[0-9a-fA-F]{4})|[^'\\])'""", argument[0]):
            return "char"
        if len(argument) == 1 and re.fullmatch(r"(?:[0-9][0-9_]*|0[xX][0-9a-fA-F_]+)", argument[0]):
            return "int"
        opening = argument.index("[") if "[" in argument and argument[-1] == "]" else len(argument)
        prefix = argument[:opening]
        if not prefix or not IDENTIFIER.fullmatch(prefix[-1]) or any(
            word != "." if index % 2 else not IDENTIFIER.fullmatch(word)
            for index, word in enumerate(prefix)
        ):
            return None
        owner, _, _ = member_parts(member)
        own_class = owner[1:-1].rsplit("/", 1)[-1]
        qualified = len(prefix) != 1
        if qualified and prefix[-3] not in ("this", own_class):
            return None
        name = prefix[-1]
        dimension = opening != len(argument)
        first, last = bounds
        class_body = self.scopes[first]
        # The audited expression subset uses explicit primitive declarations. If
        # shadowing yields contradictory declarations, abstain instead of guessing.
        kinds = set()
        for index, word in enumerate(self.words):
            if word != name or not index:
                continue
            class_field = class_body is not None and self.scopes[index] == class_body
            if not (class_field or not qualified and first < index < last):
                continue
            declaration = self.words[max(index - 3, 0):index]
            if dimension and declaration[-2:] == ("[", "]"):
                kinds.add(declaration[0] if len(declaration) == 3 else None)
            elif not dimension and declaration[-1] in {"char", "int", "byte", "short", "long", "float", "double"}:
                kinds.add(declaration[-1])
        return next(iter(kinds)) if len(kinds) == 1 else None


def parameter_matches(argument, descriptor):
    base = descriptor.lstrip("[")
    expected = PRIMITIVE_NAMES.get(base, base.removeprefix("L").removesuffix(";").replace("$", "/").rsplit("/", 1)[-1])
    depth = 0
    heads = []
    for word in argument[:-1]:
        if word == "<":
            depth += 1
        elif word == ">":
            depth -= 1
        elif depth == 0:
            heads.append(word)
    return bool(argument and IDENTIFIER.fullmatch(argument[-1])
                and expected in heads and heads.count("[") == descriptor.count("["))


def target_key(target):
    name = target["name"]
    if name == "<init>":
        name = "new " + re.split(r"[/\$]", target["owner"].removesuffix(";"))[-1]
    return name, len(parameter_types(target["descriptor"]))


def signed(value, bits):
    return value - (1 << bits) if value >> (bits - 1) else value


def literal_value(words):
    if len(words) == 1 and re.fullmatch(r"(?:[0-9][0-9_]*|0[xX][0-9a-fA-F_]+)", words[0]):
        token = words[0]
        if token.endswith("_"):
            return None
        digits = token.replace("_", "")
        try:
            if digits.lower().startswith("0x"):
                number = int(digits[2:], 16)
            elif digits.startswith("0"):
                number = int(digits, 8)
            else:
                number = int(digits, 10)
        except ValueError:
            return None
        if number < 1 << 31:
            return ["integer", number]
    if len(words) >= 3 and len(words) % 2 and re.fullmatch(r"[A-Z][A-Z0-9_]*", words[-1]) and all(
        word == "." if index % 2 else IDENTIFIER.fullmatch(word)
        for index, word in enumerate(words)
    ):
        return ["field", words[-3], words[-1]]
    return None


def register_arguments(word, trailing, opcode, descriptor, facts):
    if opcode >= 0x74:
        registers = range(trailing, trailing + (word >> 8))
    else:
        count = word >> 12
        if count > 5:
            raise ValueError("DEX invoke register count exceeds its encoding")
        registers = [((trailing >> (4 * index)) & 15) for index in range(4)] + [(word >> 8) & 15]
        registers = registers[:count]
    cursor = 0 if opcode in {0x71, 0x77} else 1
    types = parameter_types(descriptor)
    required = cursor + sum(2 if kind in {"J", "D"} else 1 for kind in types)
    if len(registers) != required:
        raise ValueError("DEX invoke register count differs from its prototype")
    result = []
    for kind in types:
        result.append(facts.get(registers[cursor]))
        cursor += 2 if kind in {"J", "D"} else 1
    return result


def code_instructions(dex, code):
    if code % 4:
        raise ValueError("unaligned selected DEX code item")
    dex.data_range(code, 16, "code item")
    count = U32.unpack_from(dex.data, code + 12)[0]
    if not count:
        raise Unavailable("the original code item has no instructions")
    start = code + 16
    dex.data_range(start, count * 2, "code instructions")
    return start, count


def instructions(dex, code):
    start, count = code_instructions(dex, code)
    if count > MAX_CODE_UNITS:
        raise Unavailable("selected DEX method exceeds the instruction bound")
    records = []
    branch_targets = set()
    values_supported = U16.unpack_from(dex.data, code + 6)[0] == 0
    pc = 0
    while pc < count:
        offset = start + pc * 2
        word = U16.unpack_from(dex.data, offset)[0]
        opcode = word & 255
        width = WIDTHS.get(opcode) or EXTRA_WIDTHS.get(opcode)
        if opcode == 0 and word:
            header = 4 if word == 0x300 else 2
            if pc + header > count or word not in (0x100, 0x200, 0x300):
                raise Unavailable(f"unsupported DEX payload at code unit {pc}")
            size = U16.unpack_from(dex.data, offset + 2)[0]
            if word == 0x300:
                elements = U32.unpack_from(dex.data, offset + 4)[0]
                width = 4 + (size * elements + 1) // 2
            else:
                width = 4 + 2 * size if word == 0x100 else 2 + 4 * size
        if width is None or pc + width > count:
            raise Unavailable(f"unsupported or truncated DEX opcode at code unit {pc}")
        if opcode == 0x28:
            branch_targets.add(pc + signed(word >> 8, 8))
        elif opcode == 0x29 or 0x32 <= opcode <= 0x3d:
            branch_targets.add(pc + signed(U16.unpack_from(dex.data, offset + 2)[0], 16))
        elif opcode == 0x2a:
            branch_targets.add(pc + signed(U32.unpack_from(dex.data, offset + 2)[0], 32))
        elif opcode in {0x2b, 0x2c}:
            values_supported = False
        records.append((pc, offset, word, opcode))
        pc += width

    values_supported = values_supported and branch_targets <= {record[0] for record in records}
    facts = {}
    for pc, offset, word, opcode in records:
        if pc in branch_targets:
            facts.clear()
        if opcode in INVOKES:
            target = U16.unpack_from(dex.data, offset + 2)[0]
            owner, name, descriptor = method_ref(dex, target)
            arguments = register_arguments(word, U16.unpack_from(dex.data, offset + 4)[0],
                                           opcode, descriptor, facts) if values_supported else []
            yield {"pc": pc, "owner": owner, "name": name, "descriptor": descriptor,
                   "arguments": arguments}
        if not values_supported:
            continue
        if opcode == 0x12:
            facts[(word >> 8) & 15] = ["integer", signed(word >> 12, 4)]
        elif opcode == 0x13:
            facts[word >> 8] = ["integer", signed(U16.unpack_from(dex.data, offset + 2)[0], 16)]
        elif opcode == 0x14:
            facts[word >> 8] = ["integer", signed(U32.unpack_from(dex.data, offset + 2)[0], 32)]
        elif 0x60 <= opcode <= 0x66:
            field = dex.fields.offset(U16.unpack_from(dex.data, offset + 2)[0])
            owner = dex.descriptor(U16.unpack_from(dex.data, field)[0])
            name = dex.string(U32.unpack_from(dex.data, field + 4)[0])
            facts[word >> 8] = ["field", owner.removesuffix(";").rsplit("/", 1)[-1], name]
        else:
            facts.clear()


def read_sources(archive, root, paths):
    mapping = {f"{logical_path(root)}/{path}": path for path in paths}
    files = {}
    if archive.stat().st_size > SOURCE_ARCHIVE_LIMIT:
        raise ValueError("source archive exceeds the size bound")
    total = 0
    with open_tar(archive, mode="r|gz") as source:
        for number, entry in enumerate(source, 1):
            total += entry.size
            if total > SOURCE_SCAN_LIMIT or number > MAX_ARCHIVE_ENTRIES:
                raise ValueError("uncompressed source archive exceeds the scan bound")
            if entry.name not in mapping:
                continue
            path = mapping[entry.name]
            if path in files or not entry.isfile() or entry.size > BYTE_LIMIT:
                files[path] = None
                continue
            with source.extractfile(entry) as stream:
                files[path] = stream.read(BYTE_LIMIT + 1)
    return files


def read_dex(archive, position):
    entries = archive.infolist()
    if position >= len(entries):
        raise ValueError("selected physical APK entry is absent")
    entry = entries[position]
    if (not APK_ROOT.fullmatch(entry.orig_filename) or not 0 < entry.file_size <= APK_ROOT_BYTES
            or entry.flag_bits & 1 or entry.compress_type not in (ZIP_STORED, ZIP_DEFLATED)):
        raise Unavailable("selected physical APK entry is outside the supported DEX roots")
    with archive.open(entry) as stream:
        data = stream.read(APK_ROOT_BYTES + 1)
    if len(data) != entry.file_size:
        raise ValueError("selected DEX entry size differs from the ZIP directory")
    return Dex(data)


def verify_revision(archive, cases):
    name = cases.get("revision_entry")
    if name is None:
        return
    entries = [entry for entry in archive.infolist() if entry.filename == name]
    if (len(entries) != 1 or not 0 < entries[0].file_size <= BYTE_LIMIT
            or entries[0].flag_bits & 1 or entries[0].compress_type not in (ZIP_STORED, ZIP_DEFLATED)):
        raise ValueError("missing or ambiguous embedded APK source revision")
    with archive.open(entries[0]) as stream:
        text = stream.read(BYTE_LIMIT + 1).decode("utf-8")
    revisions = re.findall(r'^\s*revision:\s*"([0-9a-f]{40})"\s*$', text, re.MULTILINE)
    if revisions != [cases["source_revision"]]:
        raise ValueError("embedded APK revision differs from the pinned source")


def check(obligation, status, expected=None, observed=None, detail=None):
    result = {"id": obligation["id"], "status": status, "expected": expected, "observed": observed,
              "obligation": obligation["description"], "kind": obligation["kind"]}
    if detail:
        result["detail"] = detail
    return result


def display(site):
    result = {"name": site.name, "arity": len(site.arguments), "line": site.line}
    if site.constructor and "$" in site.constructor:
        result["constructor_spelling"] = site.constructor
    if site.direct_target:
        result["direct_initializer"] = f"{site.direct_target[0]}-><init>{site.direct_target[1]}"
    return result


def select_sites(sites, targets):
    """Normalize a binary spelling only against its exact selected DEX owner."""
    aliases = {}
    basenames = set()
    for key, target in targets.items():
        owner = target["owner"][1:-1].replace("/", ".")
        short = owner.rsplit(".", 1)[-1]
        if target["name"] != "<init>" or "$" not in short:
            continue
        aliases[(owner, key[1])] = key[0]
        basenames.add((short, key[1]))
    selected = []
    ambiguous = []
    for site in sites:
        if site.direct_target:
            target = targets.get(site.key)
            if target and target["name"] == "<init>" and site.direct_target == (target["owner"], target["descriptor"]):
                selected.append(site)
        elif site.key in targets:
            selected.append(site)
        elif site.constructor:
            arity = len(site.arguments)
            name = aliases.get((site.constructor, arity))
            if name:
                selected.append(replace(site, name=name))
            elif (site.constructor.rsplit(".", 1)[-1], arity) in basenames:
                ambiguous.append(site)
    return selected, ambiguous


def projection(sites, mode):
    if mode == "call_presence":
        return sorted(set(sites))
    return list(sites)


def branch_rows(sites):
    return [list(map(list, site.choices)) for site in sites]


def occurrence_ids(calls):
    seen = Counter()
    identities = []
    for call in calls:
        key = tuple(call)
        identities.append((*key, seen[key]))
        seen[key] += 1
    return identities, seen


def reorder_detail(expected, observed):
    """Locate a changed pair that is not in opposing arms of a modeled branch."""
    left_sites, left_counts = occurrence_ids(expected["calls"])
    right_sites, right_counts = occurrence_ids(observed["calls"])
    left_arms = [dict(row) for row in expected["branch_arms"]]
    right_arms = [dict(row) for row in observed["branch_arms"]]

    if left_counts != right_counts:
        changed = {key for key in left_counts.keys() | right_counts.keys() if left_counts[key] != right_counts[key]}
        if any(site[:2] in changed and arms
               for sites, branches in ((left_sites, left_arms), (right_sites, right_arms))
               for site, arms in zip(sites, branches)):
            return None
        name, _ = sorted(changed)[0]
        return f"different displayed occurrence count of {name}"

    where = {site: index for index, site in enumerate(right_sites)}
    for first, site in enumerate(left_sites):
        for second in range(first + 1, len(left_sites)):
            following = left_sites[second]
            a, b = where[site], where[following]
            if a < b:
                continue
            if exclusive(left_arms[first], left_arms[second]) or exclusive(right_arms[a], right_arms[b]):
                continue
            return f"{site[0]} precedes {following[0]} in source and DEX, with the opposite displayed order"
    return None


def exclusive(first, second):
    return any(branch in second and arm != second[branch] for branch, arm in first.items())


def verify_obligation(obligation, method, source, bounds, body, dex_sites):
    keys = {target_key(target): target for target in obligation["targets"]}
    targets = {(target["owner"], target["name"], target["descriptor"]) for target in keys.values()}
    original = [site for site in dex_sites if (site["owner"], site["name"], site["descriptor"]) in targets]
    source_sites, _ = select_sites(body, keys)
    mode = obligation["kind"]
    canonical = "call_sequence" if mode in {"char_arguments", "argument_values"} else mode
    expected = projection((target_key(site) for site in original), canonical)
    if set(expected) != keys.keys() or expected != projection((site.key for site in source_sites), canonical):
        return check(obligation, "SETUP_ERROR",
                     detail="selected source call evidence does not agree with original DEX sites")
    report = {"calls": [list(key) for key in expected],
              "source_sites": [display(site) for site in source_sites],
              "dex_sites": [{key: value for key, value in site.items() if key != "arguments"}
                            for site in original]}
    if canonical == "call_sequence":
        report["branch_arms"] = branch_rows(source_sites)
    if mode == "argument_values":
        indexes = obligation["argument_indexes"]
        source_values = [[literal_value(site.arguments[index]) for index in indexes] for site in source_sites]
        dex_values = [[site["arguments"][index] if site["arguments"] else None for index in indexes]
                      for site in original]
        if source_values != dex_values or any(value is None for group in dex_values for value in group):
            return check(obligation, "SETUP_ERROR", detail="source arguments do not agree with bounded DEX operands")
        report["argument_values"] = source_values
    elif mode == "char_arguments":
        report["arguments"] = []
        for site in source_sites:
            arguments = []
            for index, kind in enumerate(parameter_types(keys[site.key]["descriptor"])):
                if kind != "C":
                    continue
                if source.argument_type(site.arguments[index], bounds, method["member"]) != "char":
                    return check(obligation, "SETUP_ERROR", detail="source char argument is not established")
                arguments.append(index)
            report["arguments"].append(arguments)
    return {"expected": report}


def verify_source(method, data):
    source = method["source"]
    if data is None or len(data) > BYTE_LIMIT or sha256(data).hexdigest() != source["sha256"]:
        raise ValueError("selected source file is absent or differs from its pinned fingerprint")
    text = data.decode("utf-8")
    view = View(text)
    lines = text.splitlines()
    start, end = source["start_line"], source["end_line"]
    if end > len(lines):
        raise ValueError("selected source range is outside the pinned file")
    span = View("\n".join(lines[start - 1:end]), start)
    source_sites = span.calls(span.method_body(method["member"]))
    return view, view.method_body(method["member"]), source_sites


def original_cases(cases, artifact):
    """Independently locate the original member and read its selected invoke sites."""
    methods = cases["methods"]
    with ExitStack() as stack:
        try:
            if artifact.stat().st_size > APK_FILE_BYTES or file_digest(artifact) != cases["input_sha256"]:
                raise ValueError("APK bytes differ from the pinned assessment input")
            archive = stack.enter_context(ZipFile(artifact))
            verify_revision(archive, cases)
        except (OSError, ValueError, BadZipFile, DeflateError, RuntimeError, EOFError) as error:
            for method in methods:
                yield method, {"original_verified": False, "original_error": str(error)}, None, None, None
            return

        groups = {}
        for method in methods:
            entry, owner, member = map(int, LOCATOR.fullmatch(method["at"]).groups())
            groups.setdefault(entry, []).append((method, owner, member))
        for position, selected in groups.items():
            try:
                dex = read_dex(archive, position)
            except (OSError, ValueError, BadZipFile, DeflateError, RuntimeError, EOFError) as error:
                for method, _, _ in selected:
                    yield method, {"original_verified": False, "original_error": str(error)}, None, None, None
                continue
            declarations = {}
            for method, owner, member in selected:
                try:
                    if owner not in declarations:
                        offset = dex.classes.offset(owner)
                        owner_type = U32.unpack_from(dex.data, offset)[0]
                        start = U32.unpack_from(dex.data, offset + 24)[0]
                        methods_at_class = dex.class_methods(owner_type, start)[0] if start else []
                        defined = dict(methods_at_class)
                        if len(defined) != len(methods_at_class):
                            raise ValueError("original DEX class repeats a method definition")
                        declarations[owner] = defined
                    if declarations[owner].get(member) != method["member"]:
                        raise ValueError("selected original DEX method does not match the authored locator and descriptor")
                except ValueError as error:
                    yield method, {"original_verified": False, "original_error": str(error)}, None, None, None
                    continue
                try:
                    code = method_code(dex, owner, member, method["member"])
                    _, length = code_instructions(dex, code)
                except ValueError as error:
                    message = str(error)
                    yield method, {"original_verified": True, "original_body_error": message}, None, message, dex
                    continue
                provenance = {"original_verified": True, "original_code_units": length}
                try:
                    dex_sites = list(instructions(dex, code))
                except ValueError as error:
                    yield method, provenance, None, str(error), dex
                else:
                    yield method, provenance, dex_sites, None, dex


def verify_cases(cases: dict, artifact: Path, source: Path, source_root: str) -> dict:
    """Establish source and original APK provenance independently for each case."""
    try:
        files = read_sources(source, source_root, {method["source"]["path"] for method in cases["methods"]})
        archive_error = None
    except (OSError, ValueError, TarError, RuntimeError, EOFError) as error:
        files = {}
        archive_error = str(error)

    results = {}
    for method, provenance, dex_sites, evidence_error, dex in original_cases(cases, artifact):
        source_error = archive_error
        if source_error is None:
            try:
                view, bounds, source_sites = verify_source(method, files.get(method["source"]["path"]))
            except ValueError as error:
                source_error = str(error)
        provenance["source_verified"] = source_error is None
        if source_error is not None:
            provenance["source_error"] = source_error
        verified = []
        for obligation in method["obligations"]:
            detail = provenance.get("original_error") or source_error
            if detail is not None:
                verified.append(check(obligation, "SETUP_ERROR", detail=detail))
            elif obligation["kind"] == "field_access":
                verified.append(verify_field(obligation, dex, method, files[method["source"]["path"]]))
            elif evidence_error is not None:
                verified.append(check(obligation, "SETUP_ERROR", detail=evidence_error))
            else:
                verified.append(verify_obligation(obligation, method, view, bounds, source_sites, dex_sites))
        results[method["at"]] = dict(method, **provenance, verified=verified)
    return results


def verify_field(obligation, dex, method, data):
    from corpus_fields import verify

    try:
        witness = verify(dex, method, data)
    except Unavailable as error:
        return check(obligation, "UNCHECKED", detail=str(error))
    except ValueError as error:
        return check(obligation, "SETUP_ERROR", detail=str(error))
    return {"field_witness": witness, "expected": witness["original"]}


def compare(case: dict, publication: dict) -> list[dict]:
    """Compare displayed syntax within the witness's scope.

    Calls with equal names and arities are display observations, distinct from
    attributed argument, receiver, and runtime control-flow equivalence.
    """
    try:
        text = publication.get("text")
        if not isinstance(text, str) or len(text.encode("utf-8")) > BYTE_LIMIT:
            raise Unavailable(publication.get("error", "selected method publication is unavailable"))
        view = View(text)
        bounds = view.method_body(case["member"])
        sites = view.calls(bounds)
        transfers = view.explicit_transfers(bounds)
    except (Unavailable, UnicodeError) as error:
        return [item if "status" in item else check(obligation, "UNCHECKED", item["expected"], detail=str(error))
                for obligation, item in zip(case["obligations"], case["verified"])]

    results = []
    for obligation, item in zip(case["obligations"], case["verified"]):
        if "status" in item:
            results.append(item)
            continue
        if obligation["kind"] == "field_access":
            from corpus_fields import compare as compare_field

            result, = compare_field(item["field_witness"], publication)
            results.append({**result, "id": obligation["id"], "kind": obligation["kind"],
                            "obligation": obligation["description"]})
            continue
        expected = item["expected"]
        targets = {target_key(target): target for target in obligation["targets"]}
        selected, ambiguous = select_sites(sites, targets)
        observed = {"display_sites": [display(site) for site in selected]}
        if ambiguous:
            observed["ambiguous_constructor_sites"] = [display(site) for site in ambiguous]
        kind = obligation["kind"]
        mode = "call_sequence" if kind in {"char_arguments", "argument_values"} else kind
        observed["calls"] = [list(key) for key in projection((site.key for site in selected), mode)]
        same = expected["calls"] == observed["calls"]
        detail = None
        comparison_uncertainty = None
        if not same and ambiguous:
            comparison_uncertainty = "selected constructor owner could not be established from its displayed spelling"
        if mode == "call_sequence":
            observed["branch_arms"] = branch_rows(selected)
            if not same and transfers:
                observed["explicit_transfers"] = transfers
                comparison_uncertainty = "explicit control transfers prevent comparison of the selected call order"
            if not same and comparison_uncertainty is None:
                detail = reorder_detail(expected, observed)
                if detail is None:
                    comparison_uncertainty = "different call order across mutually exclusive displayed branches"
        unknown = False
        if same and kind == "char_arguments":
            positions = expected["arguments"]
            values = [[view.argument_type(site.arguments[index], bounds, case["member"]) for index in indexes]
                      for site, indexes in zip(selected, positions)]
            observed["argument_types"] = values
            unknown = any(value is None for group in values for value in group)
            same = values == [["char"] * len(group) for group in positions]
        elif same and kind == "argument_values":
            values = [[literal_value(site.arguments[index]) for index in obligation["argument_indexes"]]
                      for site in selected]
            observed["argument_values"] = values
            unknown = any(value is None for group in values for value in group)
            same = values == expected["argument_values"]
        elif kind == "char_arguments" and len(selected) > len(expected["calls"]):
            unknown = True
        if same:
            results.append(check(obligation, "PASS", expected, observed))
        elif comparison_uncertainty:
            results.append(check(obligation, "UNCHECKED", expected, observed, comparison_uncertainty))
        elif unknown or publication.get("issues") and expected["calls"] != observed["calls"]:
            results.append(check(obligation, "UNCHECKED", expected, observed,
                                 "the selected display cannot establish all facts required by this witness"))
        else:
            results.append(check(obligation, "FAIL", expected, observed,
                                 detail or "the selected display differs from the source-aligned DEX witness"))
    return results
