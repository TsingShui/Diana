"""Verify generic argument binding on original and recompiled JVM sources."""

import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import subprocess
from tempfile import TemporaryDirectory, mkdtemp
from zipfile import BadZipFile, ZipFile, ZipInfo


FIXTURE = Path(__file__).resolve().parent
JAVA = ["java", "-XX:-UsePerfData"]
JAVAC = [
    *JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main",
    "-source", "17", "-target", "17", "-Xlint:-options", "-proc:none", "-g",
]
JAVAP = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-c", "-s"]
ENV = {key: value for key, value in os.environ.items() if key not in (
    "CLASSPATH", "JDK_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS",
)}


@dataclass(frozen=True)
class Case:
    at: str
    owner: str
    name: str
    descriptor: str
    calls: tuple[str, ...]
    casts: tuple[str, ...]
    scenarios: tuple[tuple[tuple[str, ...], str], ...]


def array_case(at, name, parameter, casts, values):
    inputs = values + (
        (2, "nested-fault/12"), (3, "capture-fault/123"),
        (4, "leading-fault/1"), (5, "leading-fault/1"),
    )
    return Case(
        at, "GenericArrayOperand", name, "(I)Ljava/lang/String;",
        ("GenericArrayEffects.leading:()I", "GenericArrayEffects.nested:(I)I",
         "GenericArrayEffects.capture:(I)Ljava/lang/Object;",
         f"GenericArrayEffects.choose:(I{parameter})Ljava/lang/String;"),
        casts, tuple((("CheckGenericArray", name, str(mode)), result) for mode, result in inputs),
    )


CASES = (
    Case(
        "j0:m1", "GenericOperand", "<clinit>", "()V",
        ("GenericEffects.capture:()Ljava/lang/Object;",
         "GenericEffects.select:(Ljava/lang/Object;)Ljava/lang/String;"),
        (), ((("CheckGeneric",), "object/12"),),
    ),
    array_case(
        "j1:m1", "object", "Ljava/lang/Object;", (),
        ((0, "object-null/1234"), (1, "object-value/1234"), (6, "object-scalar/1234")),
    ),
    array_case(
        "j1:m2", "array", "[Ljava/lang/String;", ("[Ljava/lang/String;",),
        ((0, "array-null/1235"), (1, "array-value/1235"), (6, "cast-fault/123")),
    ),
)
OWNERS = tuple(dict.fromkeys(case.owner for case in CASES))
SUPPORT = ("GenericEffects.java", "GenericArrayEffects.java")
RUNNERS = ("CheckGeneric.java", "CheckGenericArray.java")
ENTRIES = [f"{owner}.class" for owner in OWNERS]


def run(args, timeout):
    process = subprocess.run(args, text=True, capture_output=True, timeout=timeout, env=ENV)
    if process.returncode:
        raise ValueError(f"{args[0]} exited with {process.returncode}\n{process.stdout}{process.stderr}")
    return process.stdout


def classpath(*paths):
    return ["-cp", os.pathsep.join(map(str, paths))]


def compile_java(destination, paths, timeout, *dependencies):
    destination.mkdir(parents=True, exist_ok=True)
    scopes = classpath(*dependencies) if dependencies else []
    run([*JAVAC, *scopes, "-d", str(destination), *map(str, paths)], timeout)


def compile_fixture(directory, timeout):
    original = directory / "original"
    support = directory / "support"
    compile_java(support, [FIXTURE / name for name in SUPPORT], timeout)
    compile_java(original, [FIXTURE / f"{owner}.java" for owner in OWNERS], timeout, support)
    return original, support


def write_jar(artifact, original):
    with ZipFile(artifact, "x") as jar:
        for entry in ENTRIES:
            jar.writestr(ZipInfo(entry), (original / entry).read_bytes())


def prepare(destination, timeout):
    destination.mkdir(parents=True, exist_ok=True)
    artifact = destination / "bindings.jar"
    with TemporaryDirectory(prefix="diana-generic-overload-prepare-") as scratch:
        original, _ = compile_fixture(Path(scratch), timeout)
        write_jar(artifact, original)
    return artifact


def verify_jar(artifact, original):
    try:
        with ZipFile(artifact) as jar:
            if jar.namelist() != ENTRIES:
                raise ValueError("unexpected prepared fixture entries")
            for entry in jar.infolist():
                authored = (original / entry.filename).read_bytes()
                with jar.open(entry) as source:
                    if entry.file_size != len(authored) or source.read(len(authored) + 1) != authored:
                        raise ValueError(f"{entry.filename} differs from the authored compilation")
    except BadZipFile as error:
        raise ValueError(f"invalid fixture JAR: {error}") from error


def dump(owner, classes, evidence, timeout):
    text = run([*JAVAP, str(classes / f"{owner}.class")], timeout)
    evidence.write_text(text, encoding="utf-8")
    return text


def raw_method(text, case):
    methods = re.findall(
        r"^  ([^\n]+);\n    descriptor: (\S+)\n(.*?)(?=^  \S|\Z)",
        text, re.MULTILINE | re.DOTALL,
    )
    for declaration, descriptor, body in methods:
        name = "static {}" if case.name == "<clinit>" else case.name
        matches_name = (declaration == name if case.name == "<clinit>"
                        else re.search(rf"\b{re.escape(name)}\(", declaration))
        if descriptor != case.descriptor or not matches_name:
            continue
        calls = re.findall(r"invokestatic\s+#\d+\s+// Method (\S+)", body)
        casts = re.findall(r'\bcheckcast\s+#\d+\s+// class "?([^"\s]+)', body)
        return tuple(calls), tuple(casts)
    raise ValueError(f"missing recompiled or original method {case.owner}.{case.name}{case.descriptor}")


def run_observer(classes, support, args, timeout):
    return run([*JAVA, *classpath(classes, support), *args], timeout).strip()


def recover(binary, artifact, directory, support, cases, selected, timeout):
    owner = cases[0].owner
    class_at = cases[0].at.split(":", 1)[0]
    command, locator = ("get-method", cases[0].at) if selected else ("get-class", class_at)
    directory.mkdir()
    text = run([str(binary), command, str(artifact), locator, "--format", "json"], timeout)
    (directory / "publication.json").write_text(text, encoding="utf-8")
    publication = json.loads(text)
    selection = publication["selection"]
    result = publication["result"]
    members = {(row["at"], row["name"], row["prototype"]) for row in result["methods"]}
    expected = {(case.at, case.name, case.descriptor) for case in cases}
    if (
        publication["issues"] or selection["class"] != f"L{owner};"
        or selection["class_at"] != class_at or not expected.issubset(members)
        or (selected and (selection["method_at"] != locator or members != expected))
    ):
        raise ValueError(f"{locator}: incomplete or incorrectly selected publication")
    source = result["text"]
    if selected and cases[0].name == "<clinit>":
        # A method-only publication omits the original sibling field declaration.
        source, count = re.subn(r"(\bclass GenericOperand\s*\{)",
                                r"\1\n    public static String RESULT;", source, count=1)
        if count != 1:
            raise ValueError(f"{locator}: selected initializer has no class wrapper")
    path = directory / f"{owner}.java"
    path.write_text(source, encoding="utf-8")
    compile_java(directory, [path], timeout, support)
    return dump(owner, directory, directory / "recompiled.javap", timeout)


def row(identifier, expected, observed, error=None):
    status = "UNCHECKED" if error is not None else "PASS" if observed == expected else "FAIL"
    data = {"id": identifier, "expected": expected, "observed": observed, "status": status}
    if error is not None:
        data["detail"] = str(error)
    return data


def binding(endpoint, case, disassembly, recovery_error):
    expected = case.calls[-1]
    target = expected.partition(":")[0]
    if recovery_error is not None:
        return row(endpoint, expected, None, recovery_error)
    calls, _ = raw_method(disassembly, case)
    matching = [call for call in calls if call.partition(":")[0] == target]
    if len(matching) != 1:
        return row(endpoint, expected, None, "no unique recompiled invocation of the original consumer")
    return row(endpoint, expected, matching[0])


def verify(binary, artifact, evidence, timeout):
    original, support = compile_fixture(evidence, timeout)
    if artifact is None:
        artifact = evidence / "bindings.jar"
        write_jar(artifact, original)
    else:
        verify_jar(artifact, original)
    compile_java(support, [FIXTURE / name for name in RUNNERS], timeout, original, support)
    checks = {}
    for owner in OWNERS:
        cases = [case for case in CASES if case.owner == owner]
        original_dump = dump(owner, original, evidence / f"{owner}.original.javap", timeout)
        recovered = evidence / owner
        recovered_dump = recovery_error = None
        if binary is not None:
            try:
                recovered_dump = recover(binary, artifact, recovered, support, cases, False, timeout)
            except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
                recovery_error = error
        for case in cases:
            if raw_method(original_dump, case) != (case.calls, case.casts):
                raise ValueError(f"{case.at}: unexpected original call descriptors or casts")
            rows = checks[case.at] = []
            for args, expected in case.scenarios:
                identifier = f"runtime-{args[-1]}" if len(args) > 1 else "runtime-value"
                observed = run_observer(original, support, args, timeout)
                if observed != expected:
                    raise ValueError(f"{case.at}/{identifier}: original {observed}, expected {expected}")
                if binary is None:
                    rows.append(row(identifier, expected, observed))
                    continue
                try:
                    observed = (run_observer(recovered, support, args, timeout)
                                if recovery_error is None else None)
                    rows.append(row(identifier, expected, observed, recovery_error))
                except (OSError, ValueError, subprocess.SubprocessError) as error:
                    rows.append(row(identifier, expected, None, error))
            if binary is not None:
                try:
                    rows.append(binding("class-overload", case, recovered_dump, recovery_error))
                except ValueError as error:
                    rows.append(row("class-overload", case.calls[-1], None, error))
                selected = evidence / case.at.replace(":", "-")
                try:
                    selected_dump = recover(binary, artifact, selected, support, [case], True, timeout)
                    rows.append(binding("method-overload", case, selected_dump, None))
                except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
                    rows.append(row("method-overload", case.calls[-1], None, error))
    return checks


def probe(binary, artifact, timeout):
    with TemporaryDirectory(prefix="diana-generic-overload-") as scratch:
        return verify(binary, artifact, Path(scratch), timeout)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.prepare and not (args.binary or args.output):
        print(prepare(args.prepare.resolve(), 20))
        return 0
    if not args.output or args.prepare:
        parser.error("use --prepare DIR or --output PATH with an optional --binary PATH")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(mkdtemp(prefix=f"{args.output.stem}-", dir=args.output.parent))
    receipt = {"evidence": str(evidence.resolve()), "status": "SETUP_ERROR"}
    try:
        binary = args.binary.resolve(strict=True) if args.binary else None
        if binary:
            receipt["version"] = run([str(binary), "--version"], 20).strip()
        checks = verify(binary, None, evidence, 20)
        receipt["checks"] = checks
        outcomes = [check["status"] for rows in checks.values() for check in rows]
        receipt["status"] = next(status for status in ("FAIL", "UNCHECKED", "PASS") if status in outcomes)
        for at, rows in checks.items():
            for check in rows:
                if check["status"] != "PASS":
                    print(at, check["id"], check["status"], check.get("detail") or check["observed"])
        print(f"{outcomes.count('PASS')}/{len(outcomes)} checks passed")
    except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
        receipt["detail"] = str(error)
        print(f"Proof setup failed. {error}")
    args.output.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    return {"PASS": 0, "FAIL": 1, "UNCHECKED": 2, "SETUP_ERROR": 2}[receipt["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
