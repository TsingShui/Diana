"""Evaluate only the allocation fixture's small, deterministic recovery notation."""

import re
from pathlib import Path
import runpy
import subprocess
from tempfile import TemporaryDirectory

from corpus_models import MARKER, Model, execute, method_source
from corpus_oracles import Unavailable
from dex_witness import Fault, Raised, Ref


RAW = runpy.run_path(str(Path(__file__).with_name("raw_proof.py")))
Recorder = RAW["Recorder"]
OWNER = RAW["OWNER"]
ITEM = RAW["ITEM"]
INITIALIZER = RAW["ITEM_INIT"]

VALUES = ((7, 11), (71, 109))
NAME = r"[A-Za-z_$][A-Za-z0-9_$]*"
ITEM_TYPE = r"(?:fixture\.)?AllocationItem"
DECLARATION = re.compile(rf"(int|{ITEM_TYPE}|(?:java\.lang\.)?Throwable)\s+({NAME})(?:\s*=\s*(.*))?")
ASSIGNMENT = re.compile(rf"({NAME})\s*=\s*(.*)")
CALL = re.compile(rf"(?:(?:fixture\.)?AllocationStages\.)?({NAME})\s*\((.*)\)")
ALLOCATE = re.compile(rf"allocate-instance\s+{ITEM_TYPE}")
INIT = re.compile(r"init-direct\s+(.+?),\s*(L[^;\s]+;-><init>\([A-Z]*\)V),\s*\[(.*)\]")
CATCH = re.compile(
    rf"\}}\s*catch\s*\(\s*(?:java\.lang\.)?(Throwable|Exception|RuntimeException|Error)\s+({NAME})\s*\)\s*\{{"
)
HOOKS = {
    "prepare": ("()I", 0), "argument": ("(I)I", 1),
    "publish": ("(Lfixture/AllocationItem;)V", 1), "caught": ("(ILjava/lang/Throwable;)V", 2),
}


class InvalidOperation(ValueError):
    """A parsed operation violates the fixture's known typed operand semantics."""


def split_arguments(text):
    if not text.strip():
        return []
    parts = []
    start, depth = 0, 0
    for index, char in enumerate(text):
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth < 0:
                raise Unavailable("unbalanced fixture operands")
        elif char == "," and not depth:
            parts.append(text[start:index])
            start = index + 1
    if depth:
        raise Unavailable("unbalanced fixture operands")
    parts.append(text[start:])
    return parts


def expr(text, depth=0):
    if depth > 8:
        raise Unavailable("fixture expression exceeded depth bound")
    value = text.strip()
    if re.fullmatch(r"-?[0-9]{1,10}", value):
        number = int(value)
        if -(1 << 31) <= number < (1 << 31):
            return "constant", number
        raise Unavailable("integer operand lies outside the fixture word range")
    if value == "null":
        return "constant", None
    if re.fullmatch(NAME, value):
        return "name", value
    if ALLOCATE.fullmatch(value):
        return "allocate",
    if value.startswith("(") and value.endswith(")"):
        return expr(value[1:-1], depth + 1)
    call = CALL.fullmatch(value)
    if call is not None and call[1] in HOOKS:
        params = split_arguments(call[2])
        if len(params) == HOOKS[call[1]][1]:
            return "call", call[1], tuple(expr(arg, depth + 1) for arg in params)
    raise Unavailable(f"unsupported fixture expression {value!r}")


def stmt(line):
    if not line.endswith(";"):
        raise Unavailable(f"unsupported fixture statement {line!r}")
    text = line[:-1].strip()
    if text == "return":
        return "return",
    matched = INIT.fullmatch(text)
    if matched is not None:
        args = split_arguments(matched[3])
        return "init", expr(matched[1]), matched[2], tuple(expr(arg) for arg in args)
    matched = DECLARATION.fullmatch(text)
    if matched is not None:
        return "declare", matched[1].split(".")[-1], matched[2], expr(matched[3]) if matched[3] is not None else None
    matched = ASSIGNMENT.fullmatch(text)
    if matched is not None:
        return "assign", matched[1], expr(matched[2])
    effect = expr(text)
    if effect[0] in ("call", "allocate"):
        return "effect", effect
    raise Unavailable(f"unsupported fixture statement {line!r}")


def parse(text, member):
    if len(text) > 64 * 1024 or len(text.encode()) > 64 * 1024:
        raise Unavailable("fixture publication exceeds its byte bound")
    selected = method_source(text, member)
    lines = [line.strip() for line in selected.splitlines() if line.strip()]
    if (len(lines) < 2 or len(lines) > 64 or not re.fullmatch(r"public\s+static\s+void\s+run\(\)\s*\{", lines[0])
            or lines[-1] != "}"):
        raise Unavailable("unexpected fixture method envelope")
    tries = [index for index, line in enumerate(lines) if re.fullmatch(r"try\s*\{", line)]
    catches = [(index, CATCH.fullmatch(line)) for index, line in enumerate(lines) if line.startswith("}") and "catch" in line]
    if len(tries) != 1 or len(catches) != 1 or catches[0][1] is None:
        raise Unavailable("fixture replay requires one try and one recognized catch")
    start, (capture, clause) = tries[0], catches[0]
    stops = [index for index in range(capture + 1, len(lines) - 1) if lines[index] == "}"]
    if start >= capture or len(stops) != 1:
        raise Unavailable("unexpected fixture catch boundary")
    end = stops[0]
    scopes = lines[1:start], lines[start + 1:capture], lines[capture + 1:end], lines[end + 1:-1]
    return tuple(tuple(map(stmt, scope)) for scope in scopes), clause.groups()


class Replay:
    def __init__(self, recorder):
        self.recorder = recorder
        self.frames = [{}]

    def binding(self, name):
        for frame in reversed(self.frames):
            if name in frame:
                return frame
        raise InvalidOperation(f"unknown fixture local {name}")

    def eval(self, node):
        match node:
            case ("constant", value):
                return value
            case ("name", name):
                _, value = self.binding(name)[name]
                if value is UNSET:
                    raise InvalidOperation(f"uninitialized fixture local {name}")
                return value
            case ("allocate",):
                return self.recorder.allocate(ITEM)
            case ("call", name, operands):
                values = tuple(map(self.eval, operands))
                validate_hook(name, values)
                return self.recorder.call((OWNER, name, HOOKS[name][0]), None, values)
        raise InvalidOperation(f"unknown fixture operand {node!r}")

    def block(self, code):
        for op in code:
            match op:
                case ("return",):
                    return True
                case ("declare", kind, name, value):
                    if any(name in frame for frame in self.frames):
                        raise InvalidOperation(f"duplicate fixture local {name}")
                    self.frames[-1][name] = (kind, UNSET) if value is None else typed(kind, self.eval(value))
                case ("assign", name, value):
                    frame = self.binding(name)
                    kind, _ = frame[name]
                    frame[name] = typed(kind, self.eval(value))
                case ("effect", effect):
                    self.eval(effect)
                case ("init", receiver, target, operands):
                    ref = self.eval(receiver)
                    if target != f"{ITEM}-><init>(III)V":
                        raise InvalidOperation(f"unexpected fixture initializer {target}")
                    values = tuple(map(self.eval, operands))
                    if len(values) != 3 or any(type(value) is not int for value in values):
                        raise InvalidOperation("initializer requires three numeric operands")
                    if ref in self.recorder.constructed or ref not in self.recorder.allocations:
                        raise InvalidOperation("initializer requires the original uninitialized receiver")
                    self.recorder.call(INITIALIZER, ref, values)
        return False

    def scope(self, code, bindings=None):
        self.frames.append(bindings or {})
        try:
            return self.block(code)
        finally:
            self.frames.pop()

    def run(self, parsed):
        (prefix, protected, handler, suffix), (kind, exception) = parsed
        try:
            if self.block(prefix):
                return
            try:
                if self.scope(protected):
                    return
            except Raised as pending:
                identity = f"Ljava/lang/{kind};"
                if identity not in (pending.fault.descriptor, *pending.fault.supertypes):
                    raise
                if self.scope(handler, {exception: ("Throwable", pending.fault)}):
                    return
            self.block(suffix)
        except Raised as escaped:
            self.recorder.events.append(f"escape:{escaped.fault.name}")


UNSET = object()


def typed(kind, value):
    if kind == "int" and type(value) is int:
        return kind, value
    if kind == "AllocationItem" and (value is None or isinstance(value, Ref) and value.descriptor == ITEM):
        return kind, value
    if kind == "Throwable" and (value is None or isinstance(value, Fault)):
        return kind, value
    raise InvalidOperation(f"value lies outside fixture local type {kind}")


def validate_hook(name, values):
    if name == "argument" and type(values[0]) is not int:
        raise InvalidOperation("argument phase requires an integer")
    if name == "publish" and not isinstance(values[0], Ref):
        raise InvalidOperation("publish requires the original receiver")
    if name == "caught" and (type(values[0]) is not int or not isinstance(values[1], Fault)):
        raise InvalidOperation("catch requires the live stage and actual fault identity")


def run_scenario(parsed, scenario, values):
    mode, calls = scenario["arguments"]
    recorder = Recorder(mode, values)
    try:
        for invocation in range(2 if calls == "repeat" else 1):
            recorder.operations.append({"op": "entry", "invocation": invocation + 1})
            Replay(recorder).run(parsed)
            recorder.mode = "none"
    except (InvalidOperation, ValueError) as error:
        return recorder, str(error)
    return recorder, None


def expected_java(fixture, member, scenarios, timeout):
    reference_path = fixture / "AllocationStages.java"
    reference = reference_path.read_text(encoding="utf-8")
    original = method_source(reference, member)
    template = reference.replace(original, MARKER, 1)
    model = Model(fixture / "scenarios.json", reference_path, template, reference_path.stem, scenarios, None)
    with TemporaryDirectory(prefix="decbench-allocation-source-") as scratch:
        return execute(model, original, Path(scratch) / "source", timeout)


def compare_explicit(fixture, artifact, manifest, scenarios, observation, timeout):
    member = manifest["methods"][0]["member"]
    record = lambda spec, status, detail: {
        "id": spec["id"], "status": status, "expected": spec["expected"], "observed": None, "detail": detail,
    }
    if observation.get("issues"):
        return [record(spec, "UNCHECKED", "the recovered method has Diana diagnostics") for spec in scenarios]
    try:
        parsed = parse(observation["text"], member)
    except (Unavailable, ValueError) as error:
        return [record(spec, "UNCHECKED", str(error)) for spec in scenarios]
    try:
        raw = RAW["prove"](artifact, manifest, scenarios, values=VALUES)
        source = expected_java(fixture, member, scenarios, timeout)
    except (OSError, ValueError, AssertionError, subprocess.SubprocessError) as error:
        return [record(spec, "SETUP_ERROR", str(error)) for spec in scenarios]

    rows = []
    for scenario, source_row, first, second in zip(scenarios, source, *raw["checks"], strict=True):
        if source_row[0] != scenario["expected"] or source_row[1]:
            rows.append(record(scenario, "SETUP_ERROR", source_row[1] or "the authored Java control disagrees"))
            continue
        if any(proof["status"] != "PASS" for proof in (first, second)):
            rows.append(record(scenario, "SETUP_ERROR", "the raw DEX path disagrees with the authored effects"))
            continue
        primary, error = run_scenario(parsed, scenario, VALUES[0])
        alternate, alternate_error = run_scenario(parsed, scenario, VALUES[1])
        events_match = primary.events == first["observed"] and alternate.events == second["observed"]
        operations_match = primary.operations == first["operations"] and alternate.operations == second["operations"]
        matches = events_match and operations_match and not (error or alternate_error)
        row = {
            "id": scenario["id"], "status": "PASS" if matches else "FAIL",
            "expected": scenario["expected"], "observed": primary.events,
            "operations": primary.operations, "original_dex": first,
            "alternate_operands": {
                "values": VALUES[1], "observed": alternate.events,
                "operations": alternate.operations, "original_dex": second,
            },
        }
        if error or alternate_error:
            row["detail"] = error or alternate_error
        elif events_match and not operations_match:
            row["detail"] = "allocation or call operations differ from the original DEX"
        rows.append(row)
    return rows
