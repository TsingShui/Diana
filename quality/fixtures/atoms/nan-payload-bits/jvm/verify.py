"""Verify executable NaN literals and the display of encoded JVM field preparation."""

from argparse import ArgumentParser
from hashlib import sha256
import json
from os import pathsep
from pathlib import Path
import re
import subprocess
import sys
from tempfile import mkdtemp

from prepare import nan_fields, nan_methods


ROOT = Path(__file__).resolve().parent
JAVA = ["java", "-XX:-UsePerfData"]
JAVAC = [*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none"]
JAVAP = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-v"]
METHODS = {
    "ldc-float": "7fc01234",
    "ldc-double": "fff8000000005678",
    "select-float-true": "7fc01234",
    "select-float-false": "ffc05678",
    "select-double-true": "fff8000000005678",
    "select-double-false": "7ff8000000004321",
    "negative-zero": "80000000",
    "positive-infinity": "7ff0000000000000",
}
EARLY = {
    "active": "3",
    "parent": "7fc01234,fff8000000005678",
    "fields": "7fc01234,fff8000000005678",
    "controls": "80000000,7ff0000000000000",
}
LATE = {**EARLY, "parent": "00000000,0000000000000000"}
NAN_FIELDS = {"F_NAN": ("float", "7fc01234"), "D_NAN": ("double", "fff8000000005678")}


def invoke(args):
    process = subprocess.run(args, capture_output=True, text=True, timeout=20)
    if process.returncode:
        raise ValueError(f"{args[0]} exited with {process.returncode}\n{process.stdout}{process.stderr}")
    return process.stdout


def execute(classes, runner, witness):
    output = invoke([*JAVA, "-cp", pathsep.join(map(str, (classes, runner))), witness])
    (classes / f"{witness}.tsv").write_text(output, encoding="utf-8")
    result = {}
    for line in output.splitlines():
        name, observation = line.split("\t")
        if name in result:
            raise ValueError(f"duplicate observation {name}")
        result[name] = observation
    return result


def javap(class_file):
    listing = invoke([*JAVAP, str(class_file)])
    class_file.with_suffix(".javap").write_text(listing, encoding="utf-8")
    return listing


def method_shapes(class_file):
    listing = javap(class_file)
    expected = {
        "floatNaN": ("()F", ["ldc", "freturn"]),
        "doubleNaN": ("()D", ["ldc2_w", "dreturn"]),
        "selectFloatBits": ("(Z)I", ["iload_0", "ifeq", "ldc", "goto", "ldc", "invokestatic", "ireturn"]),
        "selectDoubleBits": ("(Z)J", ["iload_0", "ifeq", "ldc2_w", "goto", "ldc2_w", "invokestatic", "lreturn"]),
    }
    shapes = {}
    for name, (descriptor, expected_opcodes) in expected.items():
        method = re.search(rf"(?m)^  public static \S+ {name}\([^\n]*\);\n"
                           rf"    descriptor: {re.escape(descriptor)}\n((?:    [^\n]+\n)+)", listing)
        operations = re.findall(r"(?m)^\s+\d+:\s+(\w+)", method[1]) if method else []
        if operations != expected_opcodes:
            raise ValueError(f"original bytecode shape changed for {name}{descriptor}")
        shapes[name + descriptor] = operations
    return shapes


def preparation_shape(class_file):
    listing = javap(class_file)
    fields = {}
    for ty, name, body in re.findall(
        r"(?m)^  public static final (float|double) (\w+);\n((?:    [^\n]+\n)+)", listing,
    ):
        fields[name] = bool(re.search(rf"(?m)^    ConstantValue: {ty} ", body))
    return dict(fields=fields, clinit=bool(re.search(r"(?m)^  static \{\};$", listing)))


def field_display(source):
    facts = {}
    for name, (ty, bits) in NAN_FIELDS.items():
        match = re.search(rf"(?m)^[ \t]*public static final {ty} {name}[ \t]*=[ \t]*([^\n]+)$", source)
        declaration = match[0].strip() if match else ""
        expected = f"{ty}-bits(0x{bits}) /* JVM ConstantValue */;"
        facts[name] = dict(declaration=declaration, expected_initializer=expected,
                           matches=match is not None and match[1].strip() == expected)
    clinit = bool(re.search(r"(?m)^\s*static\s*\{", source))
    return dict(fields=facts, clinit=clinit)


def publication(binary, original, directory, owner):
    raw = invoke([str(binary), "get-class", str(original / f"{owner}.class"), "j0", "--format", "json"])
    (directory / f"{owner}.json").write_text(raw, encoding="utf-8")
    payload = json.loads(raw)
    if payload["selection"]["class"] != f"L{owner};" or payload["issues"]:
        raise ValueError(f"missing complete {owner} publication")
    source = directory / f"{owner}.java"
    source.write_text(payload["result"]["text"], encoding="utf-8")
    return source


def digest(path):
    hasher = sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def originals(evidence):
    original = evidence / "original"
    original.mkdir()
    (original / "NaNWork.class").write_bytes(nan_methods())
    (original / "NaNChild.class").write_bytes(nan_fields())
    runner = evidence / "runner"
    runner.mkdir()
    invoke([*JAVAC, "-d", str(runner), str(ROOT / "ObservingParent.java")])
    invoke([*JAVAC, "-cp", pathsep.join(map(str, (original, runner))), "-d", str(runner),
            str(ROOT / "CheckNaNPayloads.java")])
    late = evidence / "runtime-fields"
    late.mkdir()
    invoke([*JAVAC, "-cp", str(runner), "-d", str(late), str(ROOT / "runtime-fields/NaNChild.java")])
    return original, runner, late


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    options = parser.parse_args()
    output = options.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(mkdtemp(prefix=f"{output.stem}-", dir=output.parent))
    report = dict(evidence=str(evidence), status="SETUP_ERROR")
    try:
        original, runner, late = originals(evidence)
        methods = execute(original, runner, "CheckNaNPayloads")
        early = execute(original, runner, "CheckNaNPreparation")
        runtime = execute(late, runner, "CheckNaNPreparation")
        early_shape = preparation_shape(original / "NaNChild.class")
        late_shape = preparation_shape(late / "NaNChild.class")
        report.update(original_methods=methods, original_preparation=early, original_shape=early_shape,
                      original_method_shapes=method_shapes(original / "NaNWork.class"),
                      runtime_control=runtime, runtime_control_shape=late_shape,
                      input_sha256={name: digest(original / f"{name}.class") for name in ("NaNWork", "NaNChild")})
        if methods != METHODS or early != EARLY or runtime != LATE:
            raise ValueError("original or runtime field control disagrees with its independent expectation")
        if early_shape != dict(fields={"F_NAN": True, "D_NAN": True, "F_ZERO": True, "D_INF": True},
                               clinit=False):
            raise ValueError("original fields lack the expected ConstantValue preparation")
        if late_shape != dict(fields={"F_NAN": False, "D_NAN": False, "F_ZERO": True, "D_INF": True},
                              clinit=True):
            raise ValueError("runtime field control did not produce the expected late initializer")
        display_failures = []
        if options.binary:
            binary = options.binary.resolve(strict=True)
            report.update(binary_version=invoke([str(binary), "--version"]).strip(), binary_sha256=digest(binary))
            recovered = evidence / "recovered"
            recovered.mkdir()
            work = publication(binary, original, recovered, "NaNWork")
            child = publication(binary, original, recovered, "NaNChild")
            invoke([*JAVAC, "-d", str(recovered), str(work)])
            methods = execute(recovered, runner, "CheckNaNPayloads")
            display = field_display(child.read_text(encoding="utf-8"))
            report.update(recovered_methods=methods, field_display=display)
            display_failures = [name for name, row in display["fields"].items() if not row["matches"]]
            if display["clinit"]:
                display_failures.append("unexpected class initializer")
        if methods.keys() != METHODS.keys():
            raise ValueError("compiled method publication changed the scenario identities")
        method_failures = [name for name, value in METHODS.items() if methods[name] != value]
        report.update(status="FAIL" if method_failures or display_failures else "PASS",
                      method_failures=method_failures, display_failures=display_failures)
        print(f"{len(METHODS) - len(method_failures)}/{len(METHODS)} method cases passed. Receipt at {output}")
        for name in method_failures:
            print(f"{name} expected {METHODS[name]}, observed {methods[name]}")
        if display_failures:
            print(f"Field preparation display differs for {', '.join(display_failures)}")
    except (KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
        print(f"Proof setup failed. {error}. Receipt at {output}", file=sys.stderr)
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return {"PASS": 0, "FAIL": 1, "SETUP_ERROR": 2}[report["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
