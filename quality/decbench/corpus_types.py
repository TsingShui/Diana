"""Compare explicit primitive declaration slots with a verified original DEX method."""

from corpus_discover import PRIMITIVES, TYPE_KEYWORDS, declaration_arguments, declaration_type, qualified, remove_annotations
from corpus_oracles import BYTE_LIMIT, IDENTIFIER, Unavailable, View, member_parts


STATEMENT_WORDS = frozenset({
    "assert", "break", "case", "catch", "continue", "default", "do", "else", "false", "finally", "for",
    "if", "import", "instanceof", "new", "null", "package", "return", "super", "switch", "this", "throw",
    "throws", "true", "try", "while", "yield",
})


def array_suffix(words):
    end = len(words)
    while words[end - 2:end] == ("[", "]"):
        end -= 2
    return words[:end], words[end:]


def primitive(words, *, parameter):
    if not words:
        return None
    stem, dimensions = array_suffix(words)
    varargs = stem[-3:] == (".", ".", ".")
    if varargs:
        if not parameter or dimensions:
            return None
        stem, dimensions = array_suffix(stem[:-3])
    if len(stem) != 1 or stem[0] not in PRIMITIVES:
        return None
    base = PRIMITIVES[stem[0]]
    if base == "V" and (parameter or dimensions):
        return None
    return "[" * (len(dimensions) // 2 + varargs) + base


def after_type_parameters(words):
    depth = 0
    for index, token in enumerate(words):
        if token == "<":
            depth += 1
        elif token == ">":
            depth -= 1
            if not depth:
                return words[index + 1:]
    return ()


def parameter_type(argument):
    clean = remove_annotations(argument)
    if not clean:
        return None
    declaration, dimensions = array_suffix(clean)
    if (not declaration or not IDENTIFIER.fullmatch(declaration[-1])
            or declaration[-1] in STATEMENT_WORDS or declaration[-1] in PRIMITIVES):
        return None
    return primitive(declaration[:-1] + dimensions, parameter=True)


def class_scope(view, owner):
    """Select the exact top-level owner, or allow a fragment without an owner wrapper."""
    words = view.words
    parts = owner[1:-1].split("/")
    package = ""
    classes = []
    saw_wrapper = False
    cursor = 0
    while cursor < len(words):
        token = words[cursor]
        if token in ("package", "import"):
            saw_wrapper = True
            try:
                stop = words.index(";", cursor + 1)
            except ValueError as error:
                raise Unavailable("the Java owner wrapper has an unfinished declaration") from error
            if token == "package":
                declared = qualified(words[cursor + 1:stop])
                if not declared or package:
                    raise Unavailable("the Java owner wrapper has no unique package")
                package = declared
            cursor = stop + 1
            continue
        if (token in TYPE_KEYWORDS and words[cursor - 1:cursor] != (".",)
                and cursor + 1 < len(words) and IDENTIFIER.fullmatch(words[cursor + 1])):
            saw_wrapper = True
            start = cursor + 2
            while start < len(words) and words[start] not in ("{", ";", "(", "="):
                start += 1
            if start in view.ends and words[start] == "{":
                if words[cursor + 1] == parts[-1]:
                    classes.append(start)
                cursor = view.ends[start] + 1
                continue
        cursor = view.ends.get(cursor, cursor) + 1
    if not saw_wrapper:
        return None
    if package != ".".join(parts[:-1]) or len(classes) != 1:
        raise Unavailable("the selected declaration has no unique, exact top-level owner")
    return classes[0]


def terminal(view, cursor, stop):
    words = view.words
    before = cursor
    while words[cursor:cursor + 2] == ("[", "]"):
        cursor += 2
    dimensions = words[before:cursor]
    if words[cursor:cursor + 1] == ("throws",):
        cursor += 1
        start = cursor
        while cursor < stop and words[cursor] not in ("{", ";"):
            token = words[cursor]
            if token == ",":
                if not qualified(words[start:cursor]):
                    return None
                start = cursor + 1
            elif token != "." and (token in STATEMENT_WORDS or not IDENTIFIER.fullmatch(token)):
                return None
            cursor += 1
        if not qualified(words[start:cursor]):
            return None
    if cursor < stop and words[cursor] in ("{", ";"):
        return dimensions, view.ends.get(cursor, cursor) + 1
    return None


def select_declaration(view, owner, name, arity):
    owner_scope = class_scope(view, owner)
    start = 0 if owner_scope is None else owner_scope + 1
    stop = len(view.words) if owner_scope is None else view.ends[owner_scope]
    candidates = []
    words = view.words
    boundary = cursor = start
    while cursor < stop:
        token = words[cursor]
        end = view.ends.get(cursor, cursor)
        if token in ("{", ";"):
            boundary = end + 1
        if token == "(" and cursor > boundary + 1 and words[cursor - 1] == name:
            arguments = declaration_arguments(view, cursor)
            tail = terminal(view, end + 1, stop)
            if arguments is not None and len(arguments) == arity and tail is not None:
                dimensions, next_member = tail
                prefix = words[boundary:cursor - 1]
                clean = remove_annotations(prefix)
                shape = declaration_type(prefix)
                typed_prefix = (shape is not None and shape[1] not in STATEMENT_WORDS) or clean == ("?",)
                generic_prefix = clean and clean[0] == "<"
                if (typed_prefix or generic_prefix) and (owner_scope is not None or (boundary == 0 and next_member == stop)):
                    returns = after_type_parameters(clean) if generic_prefix else clean
                    result = primitive(returns + dimensions, parameter=False)
                    candidates.append((result, tuple(parameter_type(arg) for arg in arguments)))
        cursor = end + 1
    if len(candidates) != 1:
        raise Unavailable("could not isolate one displayed declaration by owner, name and arity")
    return candidates[0]


def measure_slots(case):
    owner, name, arguments = member_parts(case["member"])
    result = case["member"].rsplit(")", 1)[1]
    expected = {"return": result, **{f"parameter-{index}": value for index, value in enumerate(arguments, 1)}}
    wanted = {slot: descriptor for slot, descriptor in expected.items()
              if descriptor.lstrip("[") in PRIMITIVES.values()}
    return owner, name, len(arguments), wanted


def receipt(slot, expected, observed=None, *, status, detail=None):
    result = {"id": f"{slot}-type", "metric": "declaration_types", "status": status,
              "expected": {"slot": slot, "descriptor": expected},
              "observed": {"slot": slot, "descriptor": observed} if observed is not None else None}
    if detail:
        result["detail"] = detail
    return result


def compare(case: dict, publication: dict) -> list[dict]:
    """Use the exact-identity publication to compare primitive slots independently of its body."""
    try:
        owner, name, arity, wanted = measure_slots(case)
    except (KeyError, ValueError):
        return []
    if not wanted:
        return []
    if case.get("original_verified") is not True:
        return [receipt(slot, expected, status="SETUP_ERROR",
                        detail=case.get("original_error") or "the original DEX declaration was not verified")
                for slot, expected in wanted.items()]
    try:
        text = publication.get("text")
        if not isinstance(text, str) or not text.strip():
            raise Unavailable(publication.get("error") or "the selected method publication is unavailable")
        if len(text.encode("utf-8")) > BYTE_LIMIT:
            raise Unavailable("the selected Java publication exceeds the declaration bound")
        result, arguments = select_declaration(View(text), owner, name, arity)
    except (Unavailable, UnicodeError) as error:
        return [receipt(slot, expected, status="UNCHECKED", detail=str(error)) for slot, expected in wanted.items()]

    projected = {"return": result, **{f"parameter-{index}": value for index, value in enumerate(arguments, 1)}}
    receipts = []
    for slot, expected in wanted.items():
        observed = projected[slot]
        if observed is None:
            receipts.append(receipt(slot, expected, status="UNCHECKED",
                                    detail="the slot does not explicitly spell a supported primitive type"))
        elif observed != expected:
            receipts.append(receipt(slot, expected, observed, status="FAIL",
                                    detail="the displayed primitive slot differs from the original DEX descriptor"))
        else:
            receipts.append(receipt(slot, expected, observed, status="PASS"))
    return receipts
