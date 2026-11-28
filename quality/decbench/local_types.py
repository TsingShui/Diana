"""Compare independently anchored JVM debug-local types and recovered declarations.

The expected types come from the input class's LocalVariableTable (JVMS §4.7.13).
The recovered declarations come from JavacTask.parse, without symbol attribution
or executing, compiling, or rewriting the recovered source.
"""

from __future__ import annotations

from base64 import urlsafe_b64decode
from pathlib import Path
import json
import os
import re
import stat
import subprocess
import tempfile
from zipfile import BadZipFile, ZIP_DEFLATED, ZIP_STORED, ZipFile
from zlib import error as DeflateError

HELPER = Path(__file__).with_name("LocalTypes.java")
CLASS_LIMIT = 16 * 1024 * 1024
ARCHIVE_LIMIT = 256 * 1024 * 1024
SOURCE_LIMIT = 16 * 1024 * 1024
LOCATOR = re.compile(r"j(0|[1-9][0-9]*):m(0|[1-9][0-9]*)")
PRIMITIVES = dict(zip(
    ("boolean", "byte", "char", "short", "int", "long", "float", "double", "void"),
    "ZBCSIJFDV",
))
REFERENCE = re.compile(r"L[^/.;\[()\x00]+(?:/[^/.;\[()\x00]+)*;")
TYPE = re.compile(r"\[*(?:[ZBCSIJFD]|" + REFERENCE.pattern + ")")
CP_WIDTHS = {3: 4, 4: 4, 5: 8, 6: 8, 8: 2, 9: 4, 10: 4, 11: 4,
             12: 4, 15: 3, 16: 2, 17: 4, 18: 4, 19: 2, 20: 2}


class Unsupported(ValueError):
    """The judge cannot establish a type binding for this evidence."""


class ClassFormat(ValueError):
    """The selected JVM class could not furnish a usable debug table."""


class Reader:
    def __init__(self, data: bytes | memoryview):
        self.data = memoryview(data)
        self.position = 0

    def take(self, count: int) -> memoryview:
        end = self.position + count
        if end > len(self.data):
            raise ClassFormat("truncated class file or subattribute")
        result = self.data[self.position:end]
        self.position = end
        return result

    def u1(self) -> int:
        return self.take(1)[0]

    def u2(self) -> int:
        return int.from_bytes(self.take(2), "big")

    def u4(self) -> int:
        return int.from_bytes(self.take(4), "big")

    def end(self) -> None:
        if self.position != len(self.data):
            raise ClassFormat("trailing content in class file or subattribute")


def utf8(raw: bytes) -> str:
    # Class-file modified UTF-8 encodes NUL and supplementary planes through
    # individual UTF-16 code units.
    if any(octet == 0 or octet >= 0xF0 for octet in raw):
        raise ClassFormat("invalid class-file modified UTF-8")
    decoded = raw.replace(b"\xc0\x80", b"\0").decode("utf-8", "surrogatepass")
    return decoded.encode("utf-16-be", "surrogatepass").decode("utf-16-be", "surrogatepass")


def read_pool(reader: Reader) -> list:
    slots = reader.u2()
    pool: list = [None]
    while len(pool) < slots:
        kind = reader.u1()
        if kind == 1:
            pool.append((kind, reader.take(reader.u2())))
        elif kind == 7:
            pool.append((kind, reader.u2()))
        elif kind in CP_WIDTHS:
            reader.take(CP_WIDTHS[kind])
            pool.append(None)
            if kind in (5, 6):
                pool.append(None)
        else:
            raise ClassFormat(f"unsupported constant-pool tag {kind}")
    if len(pool) != slots:
        raise ClassFormat("invalid constant-pool slot count")
    return pool


def pool_entry(pool: list, index: int, kind: int):
    if not 0 < index < len(pool) or pool[index] is None or pool[index][0] != kind:
        raise ClassFormat(f"invalid constant-pool index {index}")
    return pool[index][1]


def string(pool: list, index: int) -> str:
    return utf8(bytes(pool_entry(pool, index, 1)))


def attributes(reader: Reader, pool: list):
    for _ in range(reader.u2()):
        name = string(pool, reader.u2())
        yield name, reader.take(reader.u4())


def local_table(code: Reader, pool: list) -> list[dict]:
    code.u2()  # max_stack
    max_locals = code.u2()
    code_length = code.u4()
    if not 0 < code_length < 65536:
        raise ClassFormat("invalid Code attribute length")
    code.take(code_length)
    code.take(8 * code.u2())
    result = []
    for name, value in attributes(code, pool):
        if name != "LocalVariableTable":
            continue
        local = Reader(value)
        for _ in range(local.u2()):
            start = local.u2()
            length = local.u2()
            variable = string(pool, local.u2())
            descriptor = string(pool, local.u2())
            slot = local.u2()
            if not TYPE.fullmatch(descriptor):
                raise ClassFormat("invalid debug-local descriptor")
            width = 2 if descriptor in ("J", "D") else 1
            if start >= code_length or start + length > code_length or slot + width > max_locals:
                raise ClassFormat("debug-local interval or slot exceeds its method")
            result.append({"name": variable, "slot": slot, "start_pc": start,
                           "end_pc": start + length, "descriptor": descriptor})
        local.end()
    code.end()
    return result


def read_methods(data: bytes, selected: set[int]) -> tuple[str, dict]:
    reader = Reader(data)
    if reader.u4() != 0xCAFEBABE:
        raise ClassFormat("invalid JVM class magic")
    reader.take(4)  # minor and major
    pool = read_pool(reader)
    reader.u2()  # access flags
    owner = "L" + string(pool, pool_entry(pool, reader.u2(), 7)) + ";"
    reader.u2()  # super_class
    reader.take(2 * reader.u2())
    for _ in range(reader.u2()):
        reader.take(6)  # access flags, name, descriptor
        for _ in attributes(reader, pool):
            pass
    result = {}
    for index in range(reader.u2()):
        reader.u2()
        member = owner + "->" + string(pool, reader.u2()) + string(pool, reader.u2())
        records = []
        error = None
        has_code = False
        for name, attribute in attributes(reader, pool):
            if name == "Code" and index in selected:
                if has_code:
                    error = "method has more than one Code attribute"
                    continue
                has_code = True
                try:
                    records.extend(local_table(Reader(attribute), pool))
                except (ClassFormat, UnicodeError) as trouble:
                    error = str(trouble)
        if index in selected:
            result[index] = (member, records, error)
    for _ in attributes(reader, pool):
        pass
    reader.end()
    return owner, result


def read_artifact(path: Path, archive_position: int) -> bytes:
    info = path.stat()
    if not stat.S_ISREG(info.st_mode):
        raise Unsupported("the benchmark local-type oracle requires a regular file")
    extension = path.suffix.lower()
    if extension == ".class":
        if archive_position != 0:
            raise Unsupported("a standalone JVM class has locator j0")
        if info.st_size > CLASS_LIMIT:
            raise Unsupported("class exceeds the local-type oracle data limit")
        with path.open("rb") as stream:
            return stream.read(CLASS_LIMIT + 1)
    if extension != ".jar":
        raise Unsupported("local debug-type evidence covers JVM class files and flat JAR entries")
    if info.st_size > ARCHIVE_LIMIT:
        raise Unsupported("JAR exceeds the local-type oracle archive limit")
    with ZipFile(path) as jar:
        entries = jar.infolist()
        if archive_position >= len(entries):
            raise ClassFormat("physical JAR class position is absent")
        chosen = entries[archive_position]
        if not chosen.orig_filename.endswith(".class") or chosen.file_size > CLASS_LIMIT:
            raise Unsupported("the selected physical JAR entry is not a supported class")
        if chosen.flag_bits & 1 or chosen.compress_type not in (ZIP_STORED, ZIP_DEFLATED):
            raise Unsupported("encrypted or unsupported JVM class compression")
        with jar.open(chosen) as stream:
            data = stream.read(CLASS_LIMIT + 1)
        if len(data) != chosen.file_size:
            raise ClassFormat("physical JAR class size does not match the central directory")
        return data


def run(argv: list[str], cwd: Path, timeout: int) -> subprocess.CompletedProcess:
    environment = {
        key: value for key, value in os.environ.items()
        if not key.startswith("DEXDEC_")
        and key not in ("CLASSPATH", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS")
    }
    return subprocess.run(
        argv, cwd=cwd, env=environment, encoding="utf-8", errors="replace",
        capture_output=True, timeout=timeout, check=False,
    )


def decode_fields(encoded: list[str]) -> list[str]:
    return [urlsafe_b64decode(value + "=" * (-len(value) % 4)).decode("utf-8")
            for value in encoded]


def parse_output(text: str) -> tuple[dict, list[str], set[str]]:
    methods = {}
    imports = []
    shadowing = set()
    for line in text.splitlines():
        event, *raw = line.split("\t")
        if event == "UNPARSEABLE":
            raise Unsupported("recovered method uses syntax outside the Java declaration parser")
        data = decode_fields(raw)
        if event == "I":
            name, static = data
            if static == "false":
                imports.append(name)
        elif event == "S":
            shadowing.add(data[0])
        elif event == "M":
            index, owner, name, returns = data
            methods[index] = {"owner": owner, "name": name, "returns": returns,
                              "variables": [], "return_values": []}
        elif event == "V":
            index, kind, name, spelling = data
            methods[index]["variables"].append({"kind": kind, "name": name, "spelling": spelling})
        elif event == "R":
            index, name = data
            methods[index]["return_values"].append(name)
        else:
            raise ValueError("unexpected event from the local-type declaration parser")
    return methods, imports, shadowing


def resolve_type(spelling: str, config: dict, imports: list[str], shadowing: set[str]) -> str | None:
    dimensions = 0
    while spelling.endswith("[]"):
        dimensions += 1
        spelling = spelling[:-2]
    if dimensions > 255 or (dimensions and spelling == "void"):
        return None
    if spelling in PRIMITIVES:
        return "[" * dimensions + PRIMITIVES[spelling]
    # Import or type-parameter shadowing invalidates an authored simple-name
    # alias. Fully qualified names and uncertain lookup are kept distinct.
    aliases = config.get("type_aliases", {})
    if "." not in spelling:
        if spelling in shadowing:
            return None
        explicit = [name for name in imports if name.rsplit(".", 1)[-1] == spelling]
        if len(explicit) > 1:
            return None
        if explicit:
            spelling = explicit[0]
        elif any(name.endswith(".*") for name in imports):
            return None
    descriptor = aliases.get(spelling)
    if not isinstance(descriptor, str) or not REFERENCE.fullmatch(descriptor):
        return None
    return "[" * dimensions + descriptor


def parsed_method(text: str, member: str, config: dict) -> tuple[dict, list[str], set[str]]:
    methods, imports, shadowing = parse_output(text)
    owner, signature = member.split("->", 1)
    dotted_owner = owner[1:-1].replace("/", ".")
    name, rest = signature.split("(", 1)
    prototype = "(" + rest
    candidates = []
    for method in methods.values():
        if method["owner"] != dotted_owner or method["name"] != name:
            continue
        parameters = [
            resolve_type(item["spelling"], config, imports, shadowing)
            for item in method["variables"] if item["kind"] == "parameter"
        ]
        returns = resolve_type(method["returns"], config, imports, shadowing)
        if returns is not None and all(item is not None for item in parameters):
            if "(" + "".join(parameters) + ")" + returns == prototype:
                candidates.append(method)
    if len(candidates) != 1:
        raise Unsupported("could not uniquely match the recovered declaration's owner and prototype")
    return candidates[0], imports, shadowing


def original_local(records: list[dict], anchor: dict) -> dict:
    matches = [
        record for record in records if record["name"] == anchor["name"]
        and record["slot"] == anchor["slot"]
        and ("start_pc" not in anchor or record["start_pc"] == anchor["start_pc"])
    ]
    if len(matches) != 1:
        if not records:
            raise Unsupported("original LocalVariableTable is absent (possibly stripped)")
        raise Unsupported("original debug-local anchor is absent or ambiguous")
    return matches[0]


def recovered_local(method: dict, anchor: dict) -> dict:
    kind = anchor["kind"]
    if kind == "returned-local":
        values = set(method["return_values"])
        if len(values) != 1 or not next(iter(values), ""):
            raise Unsupported("returns do not select one local declaration")
        name = next(iter(values))
        kind = "local"
    else:
        name = anchor["name"]
    matches = [item for item in method["variables"] if item["name"] == name]
    if len(matches) != 1 or matches[0]["kind"] != kind:
        raise Unsupported("recovered local anchor is absent or ambiguous")
    return matches[0]


def outcome(check: dict, status: str, expected=None, observed=None, detail=None) -> dict:
    result = {"id": check["id"], "status": status, "expected": expected, "observed": observed}
    if detail is not None:
        result["detail"] = detail
    return result


def recovery(binary: Path, artifact: Path, at: str, member: str,
             class_at: str, cwd: Path, timeout: int) -> str:
    process = run([str(binary), "get-method", str(artifact), at, "--format", "json"], cwd, timeout)
    if process.returncode not in (0, 1):
        raise Unsupported(process.stderr.strip() or "selected method publication is unavailable")
    try:
        reply = json.loads(process.stdout)
        selection = reply["selection"]
        declared = reply["result"]["methods"]
        text = reply["result"]["text"]
        owner, signature = member.split("->", 1)
        if (reply["schema_version"] != 5 or reply["command"] != "get-method"
            or selection["method_at"].split("::", 1)[0] != at or selection["class"] != owner
            or selection["class_at"].split("::", 1)[0] != class_at
            or len(declared) != 1 or declared[0]["at"] != at
            or declared[0]["name"] + declared[0]["prototype"] != signature
            or not isinstance(text, str) or len(text.encode("utf-8")) > SOURCE_LIMIT):
            raise ValueError("selected method or recovered source does not match the requested scope")
        return text
    except (ValueError, KeyError, TypeError, AttributeError) as error:
        raise Unsupported(f"unavailable selected method evidence: {error}") from error


def compare_method(binary: Path, artifact: Path, spec: dict, original: tuple, config: dict,
                   cwd: Path, timeout: int) -> list[dict]:
    at, member = spec["at"], spec["member"]
    member_from_class, records, error = original
    checks = spec["local_types"]
    if member_from_class != member or error:
        detail = error or "manifest identity differs from the original method table"
        return [outcome(check, "SETUP_ERROR", detail=detail) for check in checks]

    expected = []
    result = []
    for check in checks:
        try:
            expected.append((check, original_local(records, check["original"])))
        except Unsupported as trouble:
            result.append(outcome(check, "UNCHECKED", detail=str(trouble)))
    if not expected:
        return result

    try:
        text = recovery(binary, artifact, at, member, config["class_at"], cwd, timeout)
        path = cwd / "Recovered.java"
        path.write_text(text, encoding="utf-8")
        process = run(["java", str(HELPER), str(path)], cwd, timeout)
        if process.returncode:
            return result + [
                outcome(check, "SETUP_ERROR", reference,
                        detail=process.stderr.strip() or "unable to start local-type declaration parser")
                for check, reference in expected
            ]
        parsed, imports, shadowing = parsed_method(process.stdout, member, config)
    except (Unsupported, subprocess.TimeoutExpired) as trouble:
        return result + [
            outcome(check, "UNCHECKED", reference, detail=str(trouble)) for check, reference in expected
        ]
    except (OSError, ValueError) as trouble:
        return result + [
            outcome(check, "SETUP_ERROR", reference, detail=str(trouble)) for check, reference in expected
        ]

    for check, reference in expected:
        try:
            declaration = recovered_local(parsed, check["recovered"])
        except Unsupported as trouble:
            result.append(outcome(check, "UNCHECKED", reference, detail=str(trouble)))
            continue
        descriptor = resolve_type(declaration["spelling"], config, imports, shadowing)
        observed = {**declaration, "descriptor": descriptor}
        if descriptor is None:
            result.append(outcome(check, "UNCHECKED", reference, observed,
                                  "recovered declaration type binding is not established"))
        else:
            status = "PASS" if descriptor == reference["descriptor"] else "FAIL"
            result.append(outcome(check, status, reference, observed))
    return result


def compare(binary: Path, artifact: Path, manifest: dict, timeout: int) -> dict[str, list[dict]]:
    config = manifest["local_types"]
    targets = [method for method in manifest["methods"] if method.get("local_types")]
    if not targets:
        return {}
    try:
        locators = {}
        for spec in targets:
            at = spec["at"]
            match = LOCATOR.fullmatch(at)
            if not match or f"j{match[1]}" != config["class_at"]:
                raise Unsupported("local-type checks require one original JVM class selection")
            locators[at] = (int(match[1]), int(match[2]))
        position = next(iter(locators.values()))[0]
        data = read_artifact(artifact, position)
        if len(data) > CLASS_LIMIT:
            raise Unsupported("class exceeds the local-type oracle data limit")
        owner, original = read_methods(data, {method_index for _, method_index in locators.values()})
        if owner != config["class_member"]:
            raise ClassFormat("manifest class differs from the original class")
    except Unsupported as trouble:
        return {spec["at"]: [outcome(check, "UNCHECKED", detail=str(trouble))
                             for check in spec["local_types"]] for spec in targets}
    except (OSError, ClassFormat, UnicodeError, BadZipFile, NotImplementedError,
            DeflateError, EOFError) as trouble:
        return {spec["at"]: [outcome(check, "SETUP_ERROR", detail=str(trouble))
                             for check in spec["local_types"]] for spec in targets}

    results = {}
    with tempfile.TemporaryDirectory(prefix="diana-local-types-") as directory:
        cwd = Path(directory)
        for spec in targets:
            at = spec["at"]
            method = original.get(locators[at][1])
            if method is not None:
                results[at] = compare_method(binary, artifact, spec, method, config, cwd, timeout)
            else:
                results[at] = [outcome(check, "SETUP_ERROR", detail="original method ordinal is absent")
                               for check in spec["local_types"]]
    return results
