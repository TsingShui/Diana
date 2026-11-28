"""Check authored, straight-line field effects and value flow against Diana text."""

import json
import os
from pathlib import Path
import re
import subprocess
from tempfile import TemporaryDirectory
from zipfile import BadZipFile
from zlib import error as DeflateError

import dex_truth
import local_types
from evidence import file_digest, run_cli

PARSER = Path(__file__).with_name("EffectOrder.java")
SOURCE_LIMIT = 4 * 1024 * 1024
CLASS_OWNER = re.compile(r"L(?:[\w$]+/)*([\w$]+);")
JVM_CLASS = re.compile(r"j(0|[1-9][0-9]*)")
JVM_VARIANTS = {"debug": "-g", "stripped": "-g:none"}


def check(status, expected=None, observed=None, detail=None):
    result = {"id": "straight-line", "status": status, "expected": expected, "observed": observed}
    if detail:
        result["detail"] = detail
    return result


def selected_methods(artifact, owner, class_at, targets, reference, variant, input_sha256, timeout):
    """Bind the source witness to the prepared bytes before reading Diana."""
    if artifact.suffix.lower() == ".dex":
        if not isinstance(input_sha256, str) or file_digest(artifact) != input_sha256:
            raise ValueError("the authored DEX witness requires a matching input_sha256")
        return set(dex_truth.methods(artifact))
    position = JVM_CLASS.fullmatch(class_at)
    if position is None:
        raise local_types.Unsupported("effect witnesses cover standalone and flat JVM classes or bare DEX")
    indices = set()
    for method in targets:
        locator = local_types.LOCATOR.fullmatch(method["at"])
        if locator is None or locator[1] != position[1]:
            raise ValueError("selected methods do not belong to the authored class")
        indices.add(int(locator[2]))
    raw = local_types.read_artifact(artifact, int(position[1]))
    if len(raw) > local_types.CLASS_LIMIT:
        raise local_types.Unsupported("class exceeds the effect witness's byte bound")
    original_owner, originals = local_types.read_methods(raw, indices)
    if owner != original_owner:
        raise ValueError("authored class differs from the independently selected class")
    verify_jvm_reference(reference, raw, owner, variant, timeout)
    return {(f"{class_at}:m{index}", item[0]) for index, item in originals.items()}


def authored_source(directory, filename):
    source = (directory / filename).resolve()
    if not source.is_relative_to(directory) or not source.is_file():
        raise ValueError("missing authored effect witness source")
    with source.open("rb") as stream:
        data = stream.read(SOURCE_LIMIT + 1)
    if len(data) > SOURCE_LIMIT:
        raise ValueError("authored source exceeds the effect witness's byte bound")
    return data


def publication_text(binary, artifact, owner, class_at, timeout):
    capture = run_cli([str(binary), "get-class", str(artifact), class_at, "--format", "json"], timeout)
    document = capture.get("publication", {})
    selection, result = document.get("selection"), document.get("result")
    if (
        capture["exit_code"] not in (0, 1)
        or document.get("schema_version") != 5
        or document.get("command") != "get-class"
        or not isinstance(selection, dict)
        or not isinstance(selection.get("class_at"), str)
        or selection["class_at"].split("::", 1)[0] != class_at
        or isinstance(selection.get("class"), str) and selection["class"] != owner
        or not isinstance(result, dict)
        or not isinstance(result.get("text"), str)
    ):
        raise local_types.Unsupported(capture.get("error", "selected class publication is unavailable"))
    recovered = result["text"].encode("utf-8")
    if len(recovered) > SOURCE_LIMIT:
        raise local_types.Unsupported("selected publication exceeds the effect witness's byte bound")
    return recovered


def java_environment():
    return {
        name: value for name, value in os.environ.items()
        if name not in {"CLASSPATH", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS", "JDK_JAVAC_OPTIONS"}
    }


def verify_jvm_reference(reference, original, owner, variant, timeout):
    if variant not in JVM_VARIANTS:
        raise ValueError("unknown effect witness javac variant")
    relative = owner[1:-1]
    with TemporaryDirectory(prefix="decbench-effects-original-") as directory:
        scratch = Path(directory)
        source = scratch / (relative.rsplit("/", 1)[-1] + ".java")
        source.write_bytes(reference)
        classes = scratch / "classes"
        process = subprocess.run(
            ["java", "-Xmx256m", "-m", "jdk.compiler/com.sun.tools.javac.Main",
             "-source", "17", "-target", "17", "-Xlint:-options", JVM_VARIANTS[variant],
             "-proc:none", "-implicit:none", "-classpath", str(scratch), "-sourcepath", str(scratch),
             "-encoding", "UTF-8", "-d", str(classes), str(source)],
            env=java_environment(), check=False, encoding="utf-8", capture_output=True, timeout=timeout,
        )
        if process.returncode:
            raise ValueError(process.stderr.strip() or "unable to compile the authored JVM witness")
        generated = classes / (relative + ".class")
        if generated.read_bytes() != original:
            raise ValueError("prepared class differs from the authored JVM witness compiled with the selected javac profile")


def parse(authored, recovered, owner, basename, signatures, timeout):
    with TemporaryDirectory(prefix="decbench-effects-") as directory:
        scratch = Path(directory)
        expected = scratch / "reference" / f"{basename}.java"
        observed = scratch / "observed" / f"{basename}.java"
        for path, data in ((expected, authored), (observed, recovered)):
            path.parent.mkdir()
            path.write_bytes(data)
        process = subprocess.run(
            ["java", "-Xmx256m", str(PARSER), str(expected), str(observed), str(scratch), owner, *signatures],
            env=java_environment(), check=False, encoding="utf-8", capture_output=True, timeout=timeout,
        )
    if process.returncode:
        raise ValueError(process.stderr.strip() or "unable to start the effect witness parser")
    sides = [{}, {}]
    for line in process.stdout.splitlines():
        side, index, outcome, payload = line.split("\t", 3)
        i, j, value = int(side), int(index), json.loads(payload)
        if i not in (0, 1) or not 0 <= j < len(signatures) or j in sides[i]:
            raise ValueError("unexpected effect witness report identity")
        if (outcome == "witness" and isinstance(value, list) or
                outcome == "unchecked" and isinstance(value, str)):
            sides[i][j] = (outcome, value)
        else:
            raise ValueError("unexpected effect witness contents")
    if any(len(side) != len(signatures) for side in sides):
        raise ValueError("effect witness report is incomplete")
    return sides


def describe(event):
    if event is None:
        return "end of witness"
    return event["kind"] if "member" not in event else f"{event['kind']} {event['member']}"


def int_form(value, reads):
    """Reduce modeled int expressions modulo the JVM's 32-bit int range."""
    origin = value["origin"]
    if "literal" in origin:
        number = origin["literal"] % (1 << 32)
        return {("literal",): number} if number else {}
    if "parameter" in origin:
        return {("parameter", origin["parameter"]): 1}
    if "read" in origin:
        return reads[origin["read"]]
    first, second = origin["operands"]
    result = dict(int_form(first, reads))
    scale = 1 if origin["operation"] == "+" else -1
    for name, coefficient in int_form(second, reads).items():
        result[name] = (result.get(name, 0) + coefficient * scale) % (1 << 32)
        if not result[name]:
            del result[name]
    return result


def compare_witnesses(expected, observed):
    fields = {}
    reads = {}
    unknown = None
    for index in range(max(len(expected), len(observed))):
        before = expected[index] if index < len(expected) else None
        after = observed[index] if index < len(observed) else None
        if (not before or not after or before["kind"] != after["kind"]
                or before.get("member") != after.get("member")):
            detail = f"event {index + 1}: authored {describe(before)}, recovered {describe(after)}"
            return check("FAIL", expected, observed, detail)

        kind, member = before["kind"], before.get("member")
        if kind == "read" and member.endswith(":I"):
            reads[index] = fields.get(member, {("field", member): 1})
        left, right = before.get("value"), after.get("value")
        if left == right:
            if kind == "write" and left["type"] == "I":
                fields[member] = int_form(left, reads)
            continue
        detail = f"event {index + 1}: changed value provenance for {describe(before)}"
        if left and right and left["type"] == right["type"] == "I":
            term = int_form(left, reads)
            if term != int_form(right, reads):
                return check("FAIL", expected, observed, detail)
            if kind == "write":
                fields[member] = term
        elif unknown is None:
            unknown = detail

    if unknown is not None:
        return check("UNCHECKED", expected, observed,
                     f"{unknown}; equivalence of the recovered values is outside the effect model")
    return check("PASS", expected, observed)


def compare(binary: Path, artifact: Path, manifest: dict, timeout: int) -> dict[str, list[dict]]:
    targets = [method for method in manifest["methods"] if method.get("effect_order")]
    if not targets:
        return {}
    config = manifest["effect_order"]
    owner, class_at = config["class_member"], config["class_at"]
    results = {}
    try:
        basename = CLASS_OWNER.fullmatch(owner)
        if basename is None or "$" in basename[1]:
            raise local_types.Unsupported("effect witnesses require one named top-level class")
        reference = authored_source(Path(manifest["manifest"]).resolve().parent, config["reference"])
        originals = selected_methods(artifact, owner, class_at, targets, reference,
                                     config.get("variant", "debug"), manifest.get("input_sha256"), timeout)
        applicable = []
        for target in targets:
            at, member = target["at"], target["member"]
            if (member.startswith(owner + "->") and at.rsplit(":m", 1)[0] == class_at
                    and (at, member) in originals):
                applicable.append(target)
            else:
                results[at] = [check("SETUP_ERROR", detail="method is absent from original fixture declarations")]
        if not applicable:
            return results
    except local_types.Unsupported as error:
        return {target["at"]: [check("UNCHECKED", detail=str(error))] for target in targets}
    except (OSError, ValueError, BadZipFile, DeflateError, EOFError, subprocess.TimeoutExpired) as error:
        return {target["at"]: [check("SETUP_ERROR", detail=str(error))] for target in targets}

    try:
        recovered = publication_text(binary, artifact, owner, class_at, timeout)
        expected, observed = parse(reference, recovered, owner, basename[1],
                                   [item["member"].split("->", 1)[1] for item in applicable], timeout)
    except (local_types.Unsupported, subprocess.TimeoutExpired) as error:
        results.update({target["at"]: [check("UNCHECKED", detail=str(error))] for target in applicable})
        return results
    except (OSError, ValueError) as error:
        results.update({target["at"]: [check("SETUP_ERROR", detail=str(error))] for target in applicable})
        return results

    for index, target in enumerate(applicable):
        left_kind, left = expected[index]
        right_kind, right = observed[index]
        if left_kind != "witness":
            outcome = check("UNCHECKED", detail=f"authored effect witness unavailable: {left}")
        elif right_kind != "witness":
            outcome = check("UNCHECKED", expected=left, detail=right)
        else:
            outcome = compare_witnesses(left, right)
        results[target["at"]] = [outcome]
    return results
