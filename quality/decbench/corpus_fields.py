"""Check direct instance-field accessors against pinned DEX and source evidence."""

from __future__ import annotations

from corpus_oracles import BYTE_LIMIT, IDENTIFIER, Unavailable, View, member_parts, verify_source
from dex_calls import LOCATOR, method_code
from dex_truth import METHOD, U16, U32, Dex, DexUnsupported

FIELD_ACCESS = {
    "Z": (0x55, 0x0f), "B": (0x56, 0x0f), "C": (0x57, 0x0f), "S": (0x58, 0x0f),
    "I": (0x52, 0x0f), "F": (0x52, 0x0f), "J": (0x53, 0x10), "D": (0x53, 0x10),
}
STATIC = 0x0008
VOLATILE = 0x0040
SYNCHRONIZED = 0x0020 | 0x20000


def field_ref(dex: Dex, index: int) -> tuple[str, str, str]:
    owner, kind, name = METHOD.unpack_from(dex.data, dex.fields.offset(index))
    return dex.descriptor(owner), dex.string(name), dex.descriptor(kind)


def class_data(dex: Dex, class_index: int, method_index: int, owner: str) -> tuple[dict, int]:
    """Read declared instance fields, excluding ambiguous or volatile names."""
    definition = dex.classes.offset(class_index)
    start = U32.unpack_from(dex.data, definition + 24)[0]
    counts = []
    for _ in range(4):
        count, start = dex.uleb(start)
        counts.append(count)
    fields, methods = sum(counts[:2]), sum(counts[2:])
    if fields > dex.fields.count or methods > dex.method_ids.count:
        raise ValueError("encoded members exceed the declaration tables")
    dex.data_range(start, 2 * fields + 3 * methods, "encoded declarations")
    named = {}
    for static, count in ((True, counts[0]), (False, counts[1])):
        index = 0
        for ordinal in range(count):
            delta, start = dex.uleb(start)
            flags, start = dex.uleb(start)
            index += delta
            declaring, name, kind = field_ref(dex, index)
            if (ordinal and delta == 0) or declaring != owner or bool(flags & STATIC) != static:
                raise ValueError("invalid original field declaration")
            eligible = not static and not flags & VOLATILE and IDENTIFIER.fullmatch(name)
            named[name] = kind if eligible and name not in named else None
    for count in counts[2:]:
        index = 0
        for _ in range(count):
            delta, start = dex.uleb(start)
            flags, start = dex.uleb(start)
            _, start = dex.uleb(start)
            index += delta
            if index == method_index:
                return {name: kind for name, kind in named.items() if kind is not None}, flags
    raise ValueError("selected DEX method has no declaration flags")


def original_accessor(dex: Dex, class_index: int, method_index: int, member: str) -> tuple[dict, dict]:
    owner, _, parameters = member_parts(member)
    returns = member.rsplit(")", 1)[-1]
    code = method_code(dex, class_index, method_index, member)
    fields, flags = class_data(dex, class_index, method_index, owner)
    if flags & (STATIC | SYNCHRONIZED):
        raise Unavailable("the original method has no plain implicit instance receiver")
    if code % 4:
        raise ValueError("unaligned selected DEX code item")
    dex.data_range(code, 16, "code item")
    registers = U16.unpack_from(dex.data, code)[0]
    incoming = U16.unpack_from(dex.data, code + 2)[0]
    tries = U16.unpack_from(dex.data, code + 6)[0]
    size = U32.unpack_from(dex.data, code + 12)[0]
    if tries or size != 3 or incoming > registers:
        raise Unavailable("the original is outside the one-access straight-line DEX subset")
    dex.data_range(code + 16, 6, "direct field accessor instructions")
    access, field_index, terminal = (U16.unpack_from(dex.data, code + offset)[0] for offset in (16, 18, 20))
    opcode = access & 255
    if not 0x52 <= opcode <= 0x5f:
        raise Unavailable("the original has no single direct instance-field instruction")
    declared_owner, name, descriptor = field_ref(dex, field_index)
    if declared_owner != owner or fields.get(name) != descriptor:
        raise Unavailable("the original field has no unique nonvolatile declaration on this owner")
    if descriptor.startswith(("L", "[")):
        iget, ret = 0x54, 0x11
    elif descriptor in FIELD_ACCESS:
        iget, ret = FIELD_ACCESS[descriptor]
    else:
        raise ValueError("invalid selected original field type")
    receiver, value = access >> 12, (access >> 8) & 15
    wide = descriptor in ("D", "J")
    width = 2 if wide else 1
    if receiver != registers - incoming or value + width > registers:
        raise Unavailable("the original access does not use the entry receiver and bounded value registers")
    if (opcode == iget and not parameters and returns == descriptor and incoming == 1
            and terminal == (value << 8 | ret)):
        kind = "read"
    elif (opcode == iget + 7 and parameters == [descriptor] and returns == "V"
          and incoming == 1 + width and value == receiver + 1 and terminal == 0x0e):
        kind = "write"
    else:
        raise Unavailable("the original is neither a direct field return nor a direct parameter store")
    return {"kind": kind, "receiver": "this", "owner": owner, "name": name, "descriptor": descriptor, "pc": 0}, fields


def field_name(words: tuple[str, ...]) -> str:
    if words[:2] == ("this", "."):
        words = words[2:]
    if len(words) != 1 or words[0] in {"this", "super", "null", "true", "false"} or not IDENTIFIER.fullmatch(words[0]):
        raise Unavailable("field spelling does not identify one direct this member")
    return words[0]


def require_owner(view: View, scope: int | None, owner: str) -> None:
    if scope is None or view.scopes[scope] is not None or "$" in owner:
        raise Unavailable("the accessor has no direct top-level owner")
    words = view.words
    types = [words[index:index + 2] for index in range(scope - 1)
             if view.scopes[index] is None and words[index] in {"class", "interface", "enum", "record"}
             and (index == 0 or words[index - 1] != ".")]
    package = ""
    if words[:1] == ("package",) and ";" in words:
        package = "".join(words[1:words.index(";")]).replace(".", "/") + "/"
    if not types or types[-1][0] != "class" or owner != "L" + package + types[-1][1] + ";":
        raise Unavailable("the accessor is not displayed under its original declaring owner")


def accessor(view: View, member: str, *, pinned_source: bool = False) -> dict:
    """Read a single direct field return/store with no local declarations."""
    owner, method_name, parameters = member_parts(member)
    opening, closing = view.method_body(member)
    scope = view.scopes[opening]
    if pinned_source or scope is not None:
        require_owner(view, scope, owner)
    header = next((start for start, end in view.ends.items()
                   if end + 1 == opening and start >= 2
                   and view.words[start - 1:start + 1] == (method_name, "(")), None)
    if header is None:
        raise Unavailable("the accessor declaration has no direct parameter list")
    boundary = header - 2
    while boundary > 0 and view.words[boundary] not in {";", "{", "}"}:
        boundary -= 1
    if {"static", "synchronized"}.intersection(view.words[boundary:header]):
        raise Unavailable("a static or synchronized declaration is outside the direct this model")
    body = view.words[opening + 1:closing]
    if not parameters and body[:1] == ("return",) and body[-1:] == (";",):
        return {"kind": "read", "name": field_name(body[1:-1])}
    args = view.arguments(header)
    if body[-2:] == ("return", ";"):
        body = body[:-2]
    if len(parameters) == 1 and body.count("=") == 1 and body[-1:] == (";",):
        assignment = body.index("=")
        left, value = body[:assignment], body[assignment + 1:-1]
        name = field_name(left)
        if value == (args[0][-1],) and (len(left) == 3 or name != value[0]):
            return {"kind": "write", "name": name}
    raise Unavailable("the Java method is outside the direct field accessor subset")


def verify(dex: Dex, case: dict, source: bytes) -> dict:
    location = LOCATOR.fullmatch(case["at"])
    if location is None:
        raise ValueError("field evidence requires an original physical DEX method locator")
    _, class_index, method_index = map(int, location.groups())
    member = case["member"]
    try:
        view, (opening, closing), _ = verify_source(case, source)
        expected, fields = original_accessor(dex, class_index, method_index, member)
    except DexUnsupported as error:
        raise Unavailable(str(error)) from error
    span = case["source"]
    if not span["start_line"] <= view.tokens[opening].line <= view.tokens[closing].line <= span["end_line"]:
        raise ValueError("verified source accessor lies outside its pinned source span")
    confirmed = accessor(view, member, pinned_source=True)
    if confirmed != {key: expected[key] for key in ("kind", "name")}:
        raise Unavailable("pinned source access and original DEX access do not establish the same field")
    return {"at": case["at"], "member": member, "original": expected, "fields": fields}


def compare(witness: dict, publication: dict) -> list[dict]:
    expected = witness["original"]
    row = {"metric": "field_bindings", "id": "field-access", "expected": expected, "observed": None}
    text = publication.get("text")
    try:
        if (publication.get("error") or publication.get("complete") is False
                or not isinstance(text, str) or not text.strip()):
            raise Unavailable(publication.get("error") or "the selected Java publication is unavailable")
        if len(text.encode("utf-8", "surrogatepass")) > BYTE_LIMIT:
            raise Unavailable("selected Java publication exceeds the field witness bound")
        observed = accessor(View(text), witness["member"])
        kind = witness["fields"].get(observed["name"])
        if kind is None:
            raise Unavailable("recovered field spelling does not match a unique original instance field")
    except Unavailable as error:
        return [{**row, "status": "UNCHECKED", "detail": str(error)}]
    projected = {**observed, "owner": expected["owner"], "descriptor": kind, "receiver": "this"}
    matches = all(expected[key] == value for key, value in projected.items())
    result = {**row, "status": "PASS" if matches else "FAIL", "observed": projected}
    if not matches:
        result["detail"] = "the direct displayed field access differs from the source-aligned original DEX witness"
    return [result]
