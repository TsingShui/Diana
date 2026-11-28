"""Compare original straight-line invocation evidence with attributed Java call sites."""

from __future__ import annotations

from pathlib import Path
import re
import subprocess
from tempfile import TemporaryDirectory
from zipfile import BadZipFile
from zlib import error as DeflateError

from dex_calls import read_calls
from dex_truth import DexUnsupported
from evidence import run_cli
from local_types import Unsupported as UnsupportedJvm
from local_types import decode_fields, read_artifact, read_methods, run

HELPER = Path(__file__).with_name("CallBindings.java")
SOURCE_BYTES = 4 * 1024 * 1024
CLASS_OWNER = re.compile(r"L(?:[^.;/\[\]()\\\x00]+/)*[^.;/\[\]()\\\x00]+;")
JVM_LOCATOR = re.compile(r"j(0|[1-9][0-9]*):m(0|[1-9][0-9]*)")
JAVAP_METHOD = re.compile(r"^    descriptor: (\([^)]*\)\S+)$")
JAVAP_INSTRUCTION = re.compile(r"^\s*(\d+):\s+(\w+)(.*)$")
JAVAP_INVOKE = re.compile(r"\s*#\d+(?:,\s*\d+)?\s*// (?:InterfaceMethod|Method) (\S+):(\([^)]*\)\S+)\s*$")
JVM_RETURNS = {"return", "ireturn", "lreturn", "freturn", "dreturn", "areturn"}


class Unchecked(ValueError):
    """A call-order or source binding proof could not be established."""


def jvm_disassembly(artifact: Path, entry: int, selected: set[int], temp: Path, timeout: int) -> dict:
    data = read_artifact(artifact, entry)
    owner, declarations = read_methods(data, selected)
    path = temp / "Original.class"
    path.write_bytes(data)
    result = run(["java", "-Xmx256m", "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-c", "-s", str(path)],
                 temp, timeout)
    if result.returncode:
        raise Unchecked(result.stderr.strip() or "JDK disassembler failed")
    instruction_sets: list[dict] = []
    previous = ""
    for line in result.stdout.splitlines():
        declaration = JAVAP_METHOD.fullmatch(line)
        if declaration is not None:
            name = previous.partition("(")[0].strip().rsplit(" ", 1)[-1] if "(" in previous else ""
            if previous == "  static {};":
                name = "<clinit>"
            elif name == owner[1:-1].replace("/", "."):
                name = "<init>"
            instruction_sets.append({
                "member": f"{owner}->{name}{declaration[1]}", "instructions": [], "exception_table": False,
            })
        elif instruction_sets:
            match = JAVAP_INSTRUCTION.fullmatch(line)
            if match and instruction_sets[-1].get("in_code"):
                instruction_sets[-1]["instructions"].append((int(match[1]), match[2], match[3]))
            elif line == "    Code:":
                instruction_sets[-1]["in_code"] = True
            elif line == "    Exception table:":
                instruction_sets[-1]["exception_table"] = True
                instruction_sets[-1]["in_code"] = False
            elif not line.startswith("       "):
                instruction_sets[-1]["in_code"] = False
        previous = line
    records = {}
    for index, (member, _, _) in declarations.items():
        if index < len(instruction_sets) and member == instruction_sets[index]["member"]:
            records[index] = (member, owner, instruction_sets[index])
    return records


def jvm_dispatch(opcode: str, name: str) -> str:
    if name == "<init>":
        return "constructor"
    if opcode == "invokespecial":
        return "special"
    if opcode == "invokestatic":
        return "static"
    return "instance"


def jvm_invokes(owner: str, code: dict) -> list[dict]:
    instructions = code["instructions"]
    if not instructions or code["exception_table"]:
        raise Unchecked("call alignment requires an exception-free straight-line JVM method")
    calls = []
    for index, (pc, opcode, operand) in enumerate(instructions):
        if (
            opcode.startswith(("if", "goto", "jsr", "ret")) and opcode not in JVM_RETURNS
            or opcode in {"tableswitch", "lookupswitch", "athrow", "monitorenter", "monitorexit", "invokedynamic"}
            or opcode in JVM_RETURNS and index != len(instructions) - 1
        ):
            raise Unchecked(f"call alignment cannot model JVM opcode {opcode} at byte {pc}")
        if not opcode.startswith("invoke"):
            continue
        match = JAVAP_INVOKE.fullmatch(operand)
        if match is None:
            raise Unchecked(f"unresolved JVM invoke operand at byte {pc}")
        declaring_class, delimiter, name = match[1].rpartition(".")
        if not delimiter:
            declaring_class, name = owner[1:-1], match[1]
        name = name.strip('"')
        symbolic_owner = declaring_class.strip('"')
        signature_polymorphic = (
            symbolic_owner == "java/lang/invoke/MethodHandle" and name in {"invoke", "invokeExact"}
            or symbolic_owner == "java/lang/invoke/VarHandle"
        )
        calls.append({
            "pc": pc, "owner": symbolic_owner if symbolic_owner.startswith("[") else f"L{symbolic_owner};",
            "name": name, "descriptor": match[2],
            "dispatch": "polymorphic" if signature_polymorphic else jvm_dispatch(opcode, name),
        })
    if instructions[-1][1] not in JVM_RETURNS:
        raise Unchecked("straight-line JVM implementation does not end in a return")
    return calls


def originals(artifact: Path, class_at: str, class_owner: str, methods: list[dict], temp: Path, timeout: int) -> dict:
    result = {}
    jvm = artifact.suffix.lower() in (".class", ".jar")
    records = {}
    if jvm:
        locators = [JVM_LOCATOR.fullmatch(method["at"]) for method in methods]
        positions = {int(locator[1]) for locator in locators if locator is not None}
        if len(positions) != 1:
            raise Unchecked("JVM invocation oracle requires methods in one physical class")
        records = jvm_disassembly(artifact, positions.pop(), {
            int(locator[2]) for locator in locators if locator is not None
        }, temp, timeout)
    for method in methods:
        at = method["at"]
        try:
            if at.rsplit(":m", 1)[0] != class_at or not method["member"].startswith(class_owner + "->"):
                raise ValueError("method belongs to a different physical class")
            if jvm:
                locator = JVM_LOCATOR.fullmatch(at)
                if locator is None or int(locator[2]) not in records:
                    raise Unchecked("JDK disassembly did not confirm the original method name and prototype")
                member, owner, code = records[int(locator[2])]
                if member != method["member"]:
                    raise ValueError("selected JVM declaration differs from the authored member")
                result[at] = jvm_invokes(owner, code)
            else:
                result[at] = read_calls(artifact, at, method["member"])
        except (Unchecked, DexUnsupported, UnsupportedJvm) as error:
            result[at] = check("coverage", "UNCHECKED", detail=str(error))
        except (OSError, ValueError, BadZipFile, DeflateError) as error:
            result[at] = check("coverage", "SETUP_ERROR", detail=str(error))
    return result


def attribute(text: str, member: str, methods: list[str], temp: Path, timeout: int) -> dict:
    if len(text.encode("utf-8")) > SOURCE_BYTES:
        raise Unchecked("recovered class exceeds the source attribution bound")
    source = temp / (member[1:-1].rsplit("/", 1)[-1] + ".java")
    source.write_text(text, encoding="utf-8")
    process = run(["java", "-Xmx256m", str(HELPER), str(source), *methods], temp, timeout)
    if process.returncode:
        raise ValueError(process.stderr.strip() or "JavacTask attribution worker failed")
    result = {method: [] for method in methods}
    completed = set()
    for line in process.stdout.splitlines():
        event, *encoded = line.split("\t")
        fields = decode_fields(encoded)
        if not fields or fields[0] not in result or fields[0] in completed:
            raise ValueError("unexpected method identity in the Java attribution response")
        method, *data = fields
        if event == "call" and len(data) == 8:
            lexical, owner, name, descriptor, dispatch, row, column, reason = data
            result[method].append({
                "anchor": lexical, "owner": owner, "name": name, "descriptor": descriptor,
                "dispatch": dispatch, "line": int(row), "column": int(column), "detail": reason,
            })
        elif event == "complete" and not data:
            completed.add(method)
        elif event == "unchecked" and len(data) == 1:
            result[method] = data[0]
            completed.add(method)
        else:
            raise ValueError("invalid event in the Java attribution response")
    for method in set(methods) - completed:
        result[method] = "no uniquely attributed recovered declaration with the original prototype"
    return result


def check(identifier, status, expected=None, observed=None, detail=None):
    evidence = {"id": identifier, "status": status, "expected": expected, "observed": observed}
    if detail:
        evidence["detail"] = detail
    return evidence


def comparisons(before: list[dict], after: list[dict] | str) -> list[dict]:
    if isinstance(after, str):
        return [check("coverage", "UNCHECKED", expected=before, detail=after)]
    # Pair a whole lexical invocation sequence only when both sides are linear
    # and all anchors agree. Neither heuristic subsequences nor greedy matching
    # can establish an occurrence when a same-named candidate call is missing.
    if [call["name"] for call in before] != [call["anchor"] for call in after]:
        return [check("coverage", "UNCHECKED", before, after,
                      "straight-line callee occurrence sequence differs; call sites are not uniquely aligned")]
    if not before:
        return [check("coverage", "PASS", [], [], "no ordinary invokes in the selected method")]
    return [compare_call(original, recovered) for original, recovered in zip(before, after)]


def compare_call(original: dict, recovered: dict) -> dict:
    identifier = f"invoke-{original['pc']}"
    reason = recovered["detail"]
    if original["dispatch"] in {"special", "polymorphic"}:
        reason = reason or "original JVM dispatch is outside the ordinary Java binding model"
    if reason:
        return check(identifier, "UNCHECKED", original, recovered, reason)
    if any(original[key] != recovered[key] for key in ("name", "descriptor", "dispatch")):
        return check(identifier, "FAIL", original, recovered)
    if original["owner"] != recovered["owner"]:
        return check(identifier, "UNCHECKED", original, recovered,
                     "original symbolic owner differs from the Java declaring owner; inheritance is unresolved")
    return check(identifier, "PASS", original, recovered)


def recovered_class(binary: Path, artifact: Path, location: str, owner: str, timeout: int) -> str:
    capture = run_cli([str(binary.resolve()), "get-class", str(artifact.resolve()),
                       location, "--format", "json"], timeout)
    document = capture.get("publication", {})
    selection, result = document.get("selection"), document.get("result")
    if (
        capture["exit_code"] not in (0, 1)
        or document.get("schema_version") != 5
        or document.get("command") != "get-class"
        or not isinstance(selection, dict)
        or not (
            selection.get("class_at") == f"{location}::{owner}"
            or selection.get("class_at") == location and selection.get("class") == owner
        )
        or not isinstance(result, dict)
        or not isinstance(result.get("text"), str)
    ):
        raise Unchecked(capture.get("error", "recovered class publication is unavailable"))
    return result["text"]


def compare(binary: Path, artifact: Path, manifest: dict, timeout: int) -> dict[str, list[dict]]:
    methods = [method for method in manifest["methods"] if method.get("call_bindings")]
    if not methods:
        return {}
    location, owner = (manifest["call_bindings"][key] for key in ("class_at", "class_member"))
    if not CLASS_OWNER.fullmatch(owner):
        return {method["at"]: [check("coverage", "SETUP_ERROR", detail="invalid authored class identity")]
                for method in methods}
    before = {}
    try:
        with TemporaryDirectory(prefix="decbench-calls-") as directory:
            temp = Path(directory)
            before = originals(artifact, location, owner, methods, temp, timeout)
            selected = [method for method in methods if isinstance(before[method["at"]], list)]
            after = {}
            if selected:
                source = recovered_class(binary, artifact, location, owner, timeout)
                after = attribute(source, owner, [method["member"] for method in selected], temp, timeout)
    except (Unchecked, UnsupportedJvm, subprocess.TimeoutExpired) as error:
        status, detail = "UNCHECKED", str(error)
    except (OSError, ValueError, BadZipFile, DeflateError) as error:
        status, detail = "SETUP_ERROR", str(error)
    else:
        return {
            method["at"]: (
                comparisons(before[method["at"]], after[method["member"]])
                if isinstance(before[method["at"]], list) else [before[method["at"]]]
            )
            for method in methods
        }
    result = {}
    for method in methods:
        at = method["at"]
        original = before.get(at)
        result[at] = [original] if isinstance(original, dict) else [
            check("coverage", status, expected=original, detail=detail)
        ]
    return result
