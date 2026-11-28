"""Check DEX branch instructions, authored JVM traces and exact Diana output."""

from argparse import ArgumentParser
from hashlib import sha256
import json
import math
from pathlib import Path
from subprocess import SubprocessError
from tempfile import TemporaryDirectory

from dex_calls import method_code, method_ref
from dex_truth import Dex, U16, U32
from dex_witness import decode_instructions, register_words, signed, static_definition, static_field
from tool_capture import run


ROOT = Path(__file__).resolve().parent
HASH = "a8fecb9c2df9b68f476756c64f61da1698e4e530a8538fdf8130a8bfe413b12c"
OWNER = "Lfixture/BranchFloat;"
METHODS = {
    "dFalls": (0, "(DD)V", 4), "dJoinFalls": (1, "(DD)V", 4),
    "fFalls": (2, "(FF)V", 2), "fJoinBoolean": (3, "(FFZ)V", 3),
    "fJoinFalls": (4, "(FF)V", 2), "iJoinFalls": (5, "(II)V", 2), "mark": (6, "(I)V", 1),
}
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData", "-Xmx128m"]
EXECUTION_ERRORS = (OSError, ValueError, KeyError, SubprocessError)


def selected(dex, name):
    index, proto, args = METHODS[name]
    code = method_code(dex, 0, index, f"{OWNER}->{name}{proto}")
    if not static_definition(dex, 0, index):
        raise ValueError(f"{name} should be static")
    dex.data_range(code, 16, "fixture method header")
    registers, inputs, tries = (U16.unpack_from(dex.data, code + i)[0] for i in (0, 2, 6))
    units = U32.unpack_from(dex.data, code + 12)[0]
    if tries or inputs != args or not inputs <= registers <= 8 or not 0 < units <= 64:
        raise ValueError(f"unexpected original code shape for {name}")
    dex.data_range(code + 16, 2 * units, "fixture code")
    return registers - inputs, decode_instructions(dex, code + 16, units)


def validate_marker(dex, mark):
    start, code = mark
    operations = list(code.values())
    if [op.opcode for op in operations] != [0x60, 0xda, 0x90, 0x67, 0x0e]:
        raise ValueError("marker bytecode shape differs")
    get, mul, add, store, _ = operations
    if (mul.operands != (0x0a00,) or add.operands != (start << 8,)
            or any(insn.word >> 8 for insn in (get, mul, add, store))
            or any(static_field(dex, insn.operands[0]) != (OWNER, "trace", "I") for insn in (get, store))):
        raise ValueError("marker changes the expected value transfer")


def replay(dex, decoded, name, arguments):
    start, code = decoded[name]
    is_double = name.startswith("d")
    right = start + (2 if is_double else 1)
    number = int if name == "iJoinFalls" else float
    slots = {start: number(arguments[0]), right: number(arguments[1])}
    if len(arguments) == 3:
        slots[right + 1] = int(arguments[2] == "true")
    pc = 0
    trace = []
    events = []
    for _ in range(32):
        insn = code[pc]
        op, register = insn.opcode, insn.word >> 8
        target = insn.next
        if op in (0x2e, 0x2f):
            lhs, rhs = insn.operands[0] & 0xff, insn.operands[0] >> 8
            if (lhs, rhs) != (start, right) or op != (0x2f if is_double else 0x2e):
                raise ValueError("compare width or argument binding differs")
            a, b = slots[lhs], slots[rhs]
            unordered = math.isnan(a) or math.isnan(b)
            cmp = (-1 if is_double else 1) if unordered else (a > b) - (a < b)
            slots[register] = cmp
            trace.append({"pc": pc, "op": "cmpl-double" if is_double else "cmpg-float",
                          "unordered": unordered, "value": cmp})
        elif op == 0x35:
            lhs, rhs = register & 0xf, register >> 4
            if name != "iJoinFalls" or (lhs, rhs) != (start, right):
                raise ValueError("integer comparison or argument binding differs")
            taken = slots[lhs] >= slots[rhs]
            target = insn.target if taken else insn.next
            trace.append({"pc": pc, "op": "if-ge", "taken": taken, "target": target})
        elif op in (0x38, 0x3b, 0x3d):
            test = slots[register]
            taken = test == 0 if op == 0x38 else test >= 0 if op == 0x3b else test <= 0
            target = insn.target if taken else insn.next
            trace.append({"pc": pc, "op": "branch", "taken": taken, "target": target})
        elif op == 0x28:
            target = insn.target
        elif op == 0x13:
            slots[register] = signed(insn.operands[0], 16)
        elif op == 0x71:
            if method_ref(dex, insn.operands[0]) != (OWNER, "mark", "(I)V"):
                raise ValueError("unexpected original marker target")
            arg, = register_words(insn)
            events.append(slots[arg])
            trace.append({"pc": pc, "op": "mark", "value": slots[arg]})
        elif op == 0x0e:
            return int("".join(map(str, events))), trace
        else:
            raise ValueError(f"unexpected fixture opcode {op:#04x}")
        pc = target
    raise ValueError("original branch fixture exceeded path bound")


def call(argv, directory, timeout):
    result = run(argv, cwd=directory, timeout=timeout, output_limit=65536, diagnostic_limit=65536)
    if result.returncode:
        detail = (result.stderr or result.stdout).decode("utf-8", "replace")
        raise ValueError(detail.replace(str(directory), "<fixture>")[:1500]
                         or f"{Path(argv[0]).name} exited with status {result.returncode}")
    return result.stdout.decode("utf-8", "strict")


def execute(source, directory, scenarios, timeout):
    directory.mkdir()
    java = directory / "BranchFloat.java"
    java.write_text(source, encoding="utf-8")
    call([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none", "-encoding",
          "UTF-8", "-d", directory, java, ROOT / "CheckBranchFloat.java"], directory, timeout)
    observed = {}
    for case, method, args, _ in scenarios:
        try:
            value = call([*JAVA, "-cp", directory, "fixture.CheckBranchFloat", method, *args],
                         directory, timeout)
            observed[case] = (int(value), None)
        except EXECUTION_ERRORS as error:
            observed[case] = (None, str(error))
    return observed


def original(artifact, scenarios):
    with artifact.open("rb") as stream:
        data = stream.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or sha256(data).hexdigest() != HASH:
        raise ValueError("original prepared DEX differs")
    dex = Dex(data)
    decoded = {name: selected(dex, name) for name in METHODS}
    validate_marker(dex, decoded["mark"])
    cases = [case for case, _, _, _ in scenarios]
    names = {name for _, name, _, _ in scenarios}
    if len(set(cases)) != len(cases) or names != METHODS.keys() - {"mark"}:
        raise ValueError("authored branch scenario contract differs")
    return {case: replay(dex, decoded, method, args) for case, method, args, _ in scenarios}


def class_source(binary, artifact, scratch, timeout):
    exact = call([binary, "get-class", artifact, OWNER, "--format", "json"], scratch, timeout)
    publication = json.loads(exact)
    if (publication["schema_version"] != 5 or publication["command"] != "get-class"
            or publication["selection"]["class_at"] != f"e0:l0:c0::{OWNER}" or publication["issues"]):
        raise ValueError("complete original BranchFloat publication unavailable")
    text = publication["result"]["text"]
    if not isinstance(text, str) or not text.strip():
        raise ValueError("original BranchFloat source absent")
    return text


def evaluate(binary, artifact, timeout):
    artifact = Path(artifact).resolve()
    binary = Path(binary).resolve() if binary is not None else None
    scenarios = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    raw = original(artifact, scenarios)
    published = None
    failure = None
    with TemporaryDirectory(prefix="float-tail-verify-") as path:
        scratch = Path(path)
        authored = execute((ROOT / "BranchFloat.java").read_text(encoding="utf-8"),
                           scratch / "authored", scenarios, timeout)
        recovered = authored
        if binary is not None:
            try:
                published = class_source(binary, artifact, scratch, timeout)
                recovered = execute(published, scratch / "recovered", scenarios, timeout)
            except EXECUTION_ERRORS as error:
                failure = str(error)
                recovered = {}
    rows = []
    for case, name, args, expected in scenarios:
        raw_value, trace = raw[case]
        reference, original_error = authored[case]
        observed, detail = recovered.get(case, (None, failure))
        index, descriptor, _ = METHODS[name]
        row = {"id": case, "at": f"e0:l0:c0:m{index}", "member": f"{OWNER}->{name}{descriptor}",
               "arguments": args, "expected": expected, "observed": observed,
               "original_dex": {"value": raw_value, "trace": trace}, "source_control": reference}
        if raw_value != expected or reference != expected:
            row.update(status="SETUP_ERROR", detail=original_error or "original controls differ")
        elif observed is None:
            row.update(status="UNCHECKED", detail=detail)
        else:
            row["status"] = "PASS" if expected == observed else "FAIL"
        rows.append(row)
    return {"checks": rows, "input_sha256": HASH, "publication": published}


def probe(binary, artifact, timeout):
    rows = evaluate(binary, artifact, timeout)["checks"]
    selected = {}
    for row in rows:
        selected.setdefault(row["at"], []).append(row)
    return selected


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    options = parser.parse_args()
    report = evaluate(options.binary, ROOT / "classes.dex", 15)
    options.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    for row in report["checks"]:
        print(row["id"], row["status"], row["expected"], row["observed"])
    outcomes = {row["status"] for row in report["checks"]}
    return 2 if outcomes & {"SETUP_ERROR", "UNCHECKED"} else int("FAIL" in outcomes)


if __name__ == "__main__":
    raise SystemExit(main())
