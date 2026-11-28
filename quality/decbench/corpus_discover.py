"""Discover bounded, source-aligned invocation evidence from a pinned public APK."""

from __future__ import annotations

from collections import Counter, defaultdict, deque
from dataclasses import dataclass
from hashlib import sha256
from itertools import islice
from pathlib import Path, PurePosixPath
from tarfile import open as open_tar
from urllib.parse import quote, urlsplit
from zipfile import ZipFile

import corpus_fields
from corpus_oracles import (
    BYTE_LIMIT, IDENTIFIER, MAX_ARCHIVE_ENTRIES, MAX_METHODS,
    REVISION, SOURCE_ARCHIVE_LIMIT, SOURCE_SCAN_LIMIT,
    Unavailable, View, instructions, literal_value, logical_path, member_parts, parameter_types, read_dex,
    target_key, validate_cases, verify_cases, verify_obligation, verify_revision,
)
from dex_calls import method_code
from dex_truth import APK_FILE_BYTES, APK_ROOT, APK_TOTAL_ROOT_BYTES, U32
from evidence import file_digest

SOURCE_SET = "src/main/java"
MODIFIERS = frozenset({
    "abstract", "default", "final", "native", "private", "protected",
    "public", "static", "strictfp", "synchronized", "transient", "volatile",
})
TYPE_KEYWORDS = frozenset({"class", "interface", "enum", "record"})
PRIMITIVES = {
    "boolean": "Z", "byte": "B", "char": "C", "double": "D", "float": "F",
    "int": "I", "long": "J", "short": "S", "void": "V",
}
JAVA_LANG = frozenset({
    "Boolean", "Byte", "CharSequence", "Character", "Class", "Comparable",
    "Double", "Enum", "Exception", "Float", "IllegalArgumentException",
    "Integer", "Iterable", "Long", "Number", "Object", "Runnable",
    "RuntimeException", "Short", "String", "StringBuilder", "Throwable", "Void",
})
COMPLEX_SOURCE = frozenset({
    "::", "->", "class", "enum", "interface", "record",
})
CONTROL_FORMS = (
    ("exceptions", {"try", "catch", "finally", "throw"}),
    ("switch", {"switch"}),
    ("synchronization", {"synchronized"}),
    ("loops", {"for", "while", "do"}),
    ("branches", {"if"}),
)
MAX_GENERATED_TARGETS = 6


@dataclass(frozen=True)
class SourceFile:
    path: str
    module: str
    owner: str
    content: bytes


@dataclass(frozen=True)
class Declaration:
    name: str
    returns: tuple[str, ...]
    arguments: tuple[tuple[str, ...], ...]
    first_line: int
    last_line: int
    bounds: tuple[int, int]


def source_files(path: Path, root: str) -> dict[str, SourceFile]:
    """Collect unique Java source files selected by their module and package layout."""
    if not path.is_file() or path.stat().st_size > SOURCE_ARCHIVE_LIMIT:
        raise ValueError("source archive is unavailable or exceeds the scan bound")
    prefix = logical_path(root) + "/"
    scan = 0
    files = {}
    duplicates = set()
    marker = "/" + SOURCE_SET + "/"
    with open_tar(path, mode="r|gz") as source:
        for number, entry in enumerate(source, 1):
            scan += entry.size
            if scan > SOURCE_SCAN_LIMIT or number > MAX_ARCHIVE_ENTRIES:
                raise ValueError("uncompressed source archive exceeds the scan bound")
            if not entry.name.startswith(prefix):
                continue
            relative = logical_path(entry.name[len(prefix):])
            module, separator, package_path = ("/" + relative).partition(marker)
            if not separator or not package_path.endswith(".java"):
                continue
            names = PurePosixPath(package_path[:-5]).parts
            if not names or not all(IDENTIFIER.fullmatch(name) and "$" not in name for name in names):
                continue
            owner = "L" + "/".join(names) + ";"
            if owner in files:
                duplicates.add(owner)
            if not entry.isfile() or entry.size > BYTE_LIMIT:
                duplicates.add(owner)
                continue
            with source.extractfile(entry) as stream:
                content = stream.read(BYTE_LIMIT + 1)
            if len(content) != entry.size:
                duplicates.add(owner)
                continue
            files[owner] = SourceFile(relative, module.removeprefix("/") or ".", owner, content)
    return {owner: item for owner, item in files.items() if owner not in duplicates}


def qualified(words):
    if not words or len(words) % 2 == 0 or any(
        word != "." if index % 2 else not IDENTIFIER.fullmatch(word)
        for index, word in enumerate(words)
    ):
        return None
    return ".".join(words[::2])


def remove_annotations(words):
    index = 0
    result = []
    while index < len(words):
        word = words[index]
        if word != "@":
            if word not in MODIFIERS:
                result.append(word)
            index += 1
            continue
        index += 1
        if index >= len(words) or not IDENTIFIER.fullmatch(words[index]):
            return None
        index += 1
        while index + 1 < len(words) and words[index] == "." and IDENTIFIER.fullmatch(words[index + 1]):
            index += 2
        if index < len(words) and words[index] == "(":
            depth = 1
            index += 1
            while depth and index < len(words):
                depth += (words[index] == "(") - (words[index] == ")")
                index += 1
            if depth:
                return None
    return tuple(result)


def declaration_type(tokens):
    clean = remove_annotations(tokens)
    if not clean:
        return None
    erasure = []
    depth = 0
    for word in clean:
        if word == "<":
            if depth == 0 and not erasure:
                return None
            depth += 1
        elif word == ">":
            depth -= 1
            if depth < 0:
                return None
        elif depth == 0:
            erasure.append(word)
    if depth:
        return None
    rank = int(erasure[-3:] == [".", ".", "."])
    if rank:
        erasure = erasure[:-3]
    while erasure[-2:] == ["[", "]"]:
        rank += 1
        erasure = erasure[:-2]
    base = qualified(erasure)
    if base is None:
        return None
    return rank, base


def declaration_arguments(view, opening):
    first = opening + 1
    end = view.ends[opening]
    if first == end:
        return ()
    result = []
    cursor = first
    angles = 0
    while cursor < end:
        word = view.words[cursor]
        if word == "," and angles == 0:
            result.append(view.words[first:cursor])
            first = cursor + 1
        if word in ("<", ">"):
            angles += 1 if word == "<" else -1
        cursor = view.ends.get(cursor, cursor) + 1
    if angles:
        return None
    result.append(view.words[first:end])
    return tuple(result)


def source_environment(view, source):
    package = ""
    imports = {}
    wildcard = False
    words = view.words
    declarations = []
    for index, token in enumerate(words):
        if view.scopes[index] is not None:
            continue
        if token == "package":
            end = words.index(";", index + 1)
            package = qualified(words[index + 1:end])
        elif token == "import":
            end = words.index(";", index + 1)
            if words[index + 1] == "static":
                # A static single or on-demand import can introduce a nested type.
                wildcard = True
                continue
            imported = qualified(words[index + 1:end])
            if imported is None:
                wildcard = True
            else:
                alias = imported.rsplit(".", 1)[-1]
                imports[alias] = imported if alias not in imports else None
        elif token in TYPE_KEYWORDS and (index == 0 or words[index - 1] != "."):
            if index + 1 >= len(words) or not IDENTIFIER.fullmatch(words[index + 1]):
                continue
            body = index + 2
            while body < len(words) and words[body] not in ("{", ";"):
                body += 1
            if body in view.ends and words[body] == "{":
                declarations.append((words[index + 1], body, words[index + 2] == "<"))
    if len(declarations) != 1 or package is None:
        raise Unavailable("source file has no unique top-level owner")
    name, body, generic = declarations[0]
    owner = "L" + (package.replace(".", "/") + "/" if package else "") + name + ";"
    if owner != source.owner or generic:
        raise Unavailable("source owner does not match the file name or declares type parameters")
    nested_names = {words[index + 1] for index in range(body + 1, view.ends[body] - 1)
                    if words[index] in TYPE_KEYWORDS and words[index - 1] != "."}
    return package, imports, wildcard, nested_names, body


def method_declarations(view, class_body):
    words = view.words
    boundary = class_body + 1
    result = []
    for index in range(class_body + 1, view.ends[class_body]):
        word = words[index]
        scope = view.scopes[index]
        if word == "}" and scope is not None and view.scopes[scope] == class_body:
            boundary = index + 1
        if scope != class_body:
            continue
        if word == ";":
            boundary = index + 1
        if word != "(" or not IDENTIFIER.fullmatch(words[index - 1]):
            continue
        end = view.ends[index]
        body = end + 1
        if words[body] == "throws":
            while body < view.ends[class_body] and words[body] not in ("{", ";", "("):
                body += 1
        if body not in view.ends or words[body] != "{" or view.scopes[body] != class_body:
            continue
        arguments = declaration_arguments(view, index)
        if arguments is None or any(not argument or not IDENTIFIER.fullmatch(argument[-1]) for argument in arguments):
            continue
        returns = words[boundary:index - 1]
        if declaration_type(returns) is None:
            continue
        closing = view.ends[body]
        result.append(Declaration(words[index - 1], returns, tuple(arg[:-1] for arg in arguments),
                                  view.tokens[boundary].line, view.tokens[closing].line, (body, closing)))
    return result


def type_matches(tokens, descriptor, environment, owners):
    syntax = declaration_type(tokens)
    if syntax is None:
        return False
    rank, base = syntax
    if rank != descriptor.count("["):
        return False
    erased = descriptor[rank:]
    if base in PRIMITIVES:
        return PRIMITIVES[base] == erased
    if not erased.startswith("L") or "$" in erased:
        return False
    dotted = erased[1:-1].replace("/", ".")
    package, imports, wildcard, nested, _ = environment
    if base.split(".", 1)[0] in nested:
        return False
    if "." in base:
        return base == dotted and base.split(".", 1)[0] not in imports
    if base in imports:
        return imports[base] == dotted
    same_package = "L" + (package.replace(".", "/") + "/" if package else "") + base + ";"
    if same_package in owners:
        return erased == same_package
    return not wildcard and base in JAVA_LANG and dotted == "java.lang." + base


def source_match(declarations, signature, environment, owners):
    _, name, parameters = member_parts(signature)
    result = signature.split(")", 1)[1]
    matches = [method for method in declarations if method.name == name
               and len(method.arguments) == len(parameters)
               and type_matches(method.returns, result, environment, owners)
               and all(type_matches(words, descriptor, environment, owners)
                       for words, descriptor in zip(method.arguments, parameters))]
    return matches[0] if len(matches) == 1 else None


def unsupported_body(view, bounds):
    start, stop = bounds
    words = view.words[start + 1:stop]
    return any(word in COMPLEX_SOURCE for word in words) or any(
        left + right in {"::", "->"} for left, right in zip(words, words[1:])
    )


def control_forms(view, bounds):
    start, stop = bounds
    words = set(view.words[start + 1:stop])
    return [form for form, tokens in CONTROL_FORMS if words & tokens] or ["straight-line"]


def candidate_targets(source_sites, original_sites):
    source_counts = Counter(site.key for site in source_sites)
    original = defaultdict(list)
    for site in original_sites:
        original[target_key(site)].append(site)
    result = []
    for key in sorted(source_counts):
        if source_counts[key] != 1 or len(original[key]) != 1:
            continue
        site = original[key][0]
        # Preserve exact original symbols without claiming receiver attribution in source.
        result.append({name: site[name] for name in ("owner", "name", "descriptor")})
    return result[:MAX_GENERATED_TARGETS]


def roots(archive):
    total = 0
    for index, entry in enumerate(archive.infolist()):
        if APK_ROOT.fullmatch(entry.orig_filename):
            total += entry.file_size
            if total > APK_TOTAL_ROOT_BYTES:
                raise ValueError("APK DEX roots exceed the discovery scan bound")
            yield index, read_dex(archive, index)


def original_owners(archive, inputs):
    all_owners = set()
    matched = defaultdict(list)
    for position, dex in roots(archive):
        for index in range(dex.classes.count):
            offset = dex.classes.offset(index)
            owner_index = U32.unpack_from(dex.data, offset)[0]
            owner = dex.descriptor(owner_index)
            all_owners.add(owner)
            if owner in inputs:
                code = U32.unpack_from(dex.data, offset + 24)[0]
                matched[owner].append((position, dex, index, owner_index, code))
    return all_owners, matched


def project_case(file, at, member, declaration, targets, revision, repository):
    start, end = declaration.first_line, declaration.last_line
    return {
        "at": at, "member": member, "module": file.module,
        "source": {
            "path": file.path, "start_line": start, "end_line": end,
            "sha256": sha256(file.content).hexdigest(),
            "url": f"{repository}/blob/{revision}/{quote(file.path, safe='/')}#L{start}-L{end}",
        },
        "obligations": [{
            "id": "discovered-invocation-presence", "kind": "call_presence",
            "description": "Retain displayed names and argument counts for selected unique invocation sites in source and DEX.",
            "targets": targets,
        }] if targets else [],
    }


def invocation_arguments(case, targets, view, bounds, source_sites, dex_sites):
    """Add only independently established argument facts without losing the call witness."""
    source = {site.key: site for site in source_sites}
    for index, target in enumerate(targets):
        witnesses = []
        types = parameter_types(target["descriptor"])
        if "C" in types:
            witnesses.append({
                "kind": "char_arguments", "targets": [target], "id": f"discovered-char-arguments-{index}",
                "description": "Display the original char view at the selected invocation.",
            })
        site = source[target_key(target)]
        literal_indexes = [argument for argument, words in enumerate(site.arguments) if literal_value(words) is not None]
        if literal_indexes:
            witnesses.append({
                "kind": "argument_values", "targets": [target], "id": f"discovered-argument-values-{index}",
                "description": "Display the source literal or static field confirmed by the original DEX operand.",
                "argument_indexes": literal_indexes,
            })
        for witness in witnesses:
            checked = verify_obligation(witness, case, view, bounds, source_sites, dex_sites)
            if "expected" in checked and "status" not in checked:
                case["obligations"].append(witness)


def balanced_cases(candidates):
    """Round-robin across modules, control forms, and declaring classes."""
    modules = defaultdict(lambda: defaultdict(lambda: defaultdict(deque)))
    for case in sorted(candidates, key=lambda item: (item["module"], item["member"], item["at"])):
        primary = case.get("source_forms", ["straight-line"])[0]
        modules[case["module"]][primary][case["member"].split("->", 1)[0]].append(case)
    active = deque(deque(deque(classes.values()) for _, classes in sorted(forms.items()))
                   for _, forms in sorted(modules.items()))
    while active:
        forms = active.popleft()
        classes = forms.popleft()
        queue = classes.popleft()
        yield queue.popleft()
        if queue:
            classes.append(queue)
        if classes:
            forms.append(classes)
        if forms:
            active.append(forms)


@dataclass(frozen=True)
class DiscoveryPlan:
    artifact: Path
    source: Path
    source_root: str
    pins: dict
    candidates: tuple[dict, ...]

    def select(self, limit: int, shard: tuple[int, int]) -> dict:
        if not 0 < limit <= MAX_METHODS:
            raise ValueError(f"discovery limit must be between 1 and {MAX_METHODS}")
        number, count = shard
        if not 1 <= number <= count <= MAX_METHODS:
            raise ValueError(f"discovery shard must be 1-based and at most {MAX_METHODS} partitions")
        remaining = islice(self.candidates, number - 1, None, count)
        selected = []
        while len(selected) < limit:
            batch = list(islice(remaining, max(limit, 16)))
            if not batch:
                break
            verified = verify_cases(dict(self.pins, methods=batch), self.artifact, self.source, self.source_root)
            for case in batch:
                if all("status" not in item for item in verified[case["at"]]["verified"]):
                    selected.append(case)
                    if len(selected) == limit:
                        break
        discovery = {
            "kind": "source-dex-invocations-and-accessors",
            "limit": limit,
            "candidate_methods": len(self.candidates),
            "selected_modules": sorted({case["module"] for case in selected}),
            "shard": {"number": number, "count": count},
            "source_forms": dict(sorted(Counter(form for case in selected for form in case["source_forms"]).items())),
        }
        manifest = dict(self.pins, methods=selected, discovery=discovery)
        return validate_cases(manifest, "generated discovery", allow_empty=True)


def prepare_discovery(artifact: Path, source: Path, source_root: str, source_revision: str,
                      repository_url: str, *, input_sha256: str | None = None,
                      revision_entry: str | None = None, exclude=()) -> DiscoveryPlan:
    """Scan the pinned source and DEX once for any number of disjoint selections."""
    if not isinstance(source_revision, str) or not REVISION.fullmatch(source_revision):
        raise ValueError("discovery requires a pinned source revision")
    repository = repository_url.rstrip("/")
    parts = urlsplit(repository)
    if parts.scheme != "https" or not parts.netloc or not parts.path or parts.query or parts.fragment:
        raise ValueError("discovery requires an immutable HTTPS repository link base")
    if not artifact.is_file() or artifact.stat().st_size > APK_FILE_BYTES:
        raise ValueError("APK bytes are absent or exceed the discovery bound")
    checksum = file_digest(artifact)
    if input_sha256 is not None and input_sha256 != checksum:
        raise ValueError("APK bytes differ from the pinned discovery input")

    pins = {"schema_version": 1, "input_sha256": checksum, "source_revision": source_revision}
    if revision_entry is not None:
        pins["revision_entry"] = logical_path(revision_entry)
    inputs = source_files(source, source_root)
    excluded = set(exclude)
    candidates = []
    with ZipFile(artifact) as archive:
        verify_revision(archive, pins)
        dex_owners, originals = original_owners(archive, inputs)
        known_owners = dex_owners | inputs.keys()
        for owner, selections in sorted(originals.items()):
            if len(selections) != 1 or selections[0][-1] == 0:
                continue
            position, dex, class_index, owner_index, start = selections[0]
            file = inputs[owner]
            try:
                view = View(file.content.decode("utf-8"))
                environment = source_environment(view, file)
                declarations = method_declarations(view, environment[-1])
            except (UnicodeError, ValueError):
                continue
            definitions, _ = dex.class_methods(owner_index, start)
            seen = Counter(member for _, member in definitions)
            for index, member in definitions:
                at = f"e{position}:l0:c{class_index}:m{index}"
                if at in excluded or seen[member] != 1 or "-><" in member:
                    continue
                try:
                    declaration = source_match(declarations, member, environment, known_owners)
                    if declaration is None or unsupported_body(view, declaration.bounds):
                        continue
                    if view.method_body(member) != declaration.bounds:
                        continue
                    sites = view.calls(declaration.bounds)
                    raw = list(instructions(dex, method_code(dex, class_index, index, member)))
                    targets = candidate_targets(sites, raw)
                    case = project_case(file, at, member, declaration, targets, source_revision, repository)
                    case["source_forms"] = control_forms(view, declaration.bounds)
                    if targets:
                        invocation_arguments(case, targets, view, declaration.bounds, sites, raw)
                    elif not sites:
                        corpus_fields.verify(dex, case, file.content)
                        case["source_forms"].insert(0, "field-access")
                        case["obligations"].append({
                            "id": "discovered-field-access", "kind": "field_access",
                            "description": "Keep the original own-instance field read or write in the displayed accessor.",
                        })
                    if case["obligations"]:
                        candidates.append(case)
                except ValueError:
                    continue

    return DiscoveryPlan(artifact, source, source_root, pins, tuple(balanced_cases(candidates)))


def discover(artifact: Path, source: Path, source_root: str, source_revision: str,
             repository_url: str, *, limit: int = 100, input_sha256: str | None = None,
             revision_entry: str | None = None, exclude=(), shard: tuple[int, int] = (1, 1)) -> dict:
    """Build and verify a separately cacheable manifest before reading any decompiler."""
    plan = prepare_discovery(artifact, source, source_root, source_revision, repository_url,
                             input_sha256=input_sha256, revision_entry=revision_entry, exclude=exclude)
    return plan.select(limit, shard)
