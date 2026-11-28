"""Compare field-initializer evaluation schedules through the ordinary JVM CLI."""

import argparse
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
JAVAP = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-c", "-v"]
CLASSES = {
    "j0:m1": ("EarlierOperand", "earlier", ("capture", "earlier", "join"), {
        "normal": (0, "captured/123"),
        "competing-faults": (3, "capture-fault/1"),
    }),
    "j1:m0": ("ConditionalOperand", "conditional", ("capture", "flag", "fallback"), {
        "false-branch": (0, "other/123"),
        "true-branch": (8, "captured/12"),
        "capture-fails-before-false-branch": (1, "capture-fault/1"),
    }),
    "j2:m1": ("ShortOperand", "short", ("captureBoolean", "flag"), {
        "short-circuits": (0, "false/12"),
        "checks-both": (8, "true/12"),
        "capture-fails-before-short-circuit": (1, "capture-fault/1"),
    }),
    "j3:m1": ("ProtectedOperand", "protected", ("capture", "earlier", "join", "after", "record"), {
        "protected-normal": (0, "captured/1234"),
        "typed-catch": (4, "captured/12345"),
        "earlier-fault-escapes": (2, "earlier-fault/12"),
        "competing-faults-before-try": (3, "capture-fault/1"),
    }),
    "j4:m1": ("FirstOperand", "first", ("capture", "earlier", "joinFirst"), {
        "normal": (0, "captured/123"),
        "competing-faults": (3, "capture-fault/1"),
    }),
}
ORACLE_ENV = {key: value for key, value in os.environ.items() if key not in (
    "CLASSPATH", "JDK_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS",
)}


def run(args, timeout):
    process = subprocess.run(args, text=True, capture_output=True, timeout=timeout, env=ORACLE_ENV)
    if process.returncode:
        raise ValueError(f"{args[0]} exited with {process.returncode}\n{process.stdout}{process.stderr}")
    return process.stdout


def compile_java(output, sources, timeout, *dependencies):
    output.mkdir(parents=True, exist_ok=True)
    classpath = ["-cp", os.pathsep.join(map(str, dependencies))] if dependencies else []
    run([*JAVAC, *classpath, "-d", str(output), *map(str, sources)], timeout)


def compile_fixture(directory, timeout):
    original = directory / "original"
    support = directory / "support"
    compile_java(support, [FIXTURE / "ScheduleEffects.java"], timeout)
    compile_java(original, [FIXTURE / f"{name}.java" for name, *_ in CLASSES.values()],
                 timeout, support)
    return original, support


def jar_entries():
    return [f"{name}.class" for name, *_ in CLASSES.values()]


def write_fixture_jar(artifact, original):
    with ZipFile(artifact, "x") as archive:
        for entry in jar_entries():
            archive.writestr(ZipInfo(entry), (original / entry).read_bytes())


def prepare(destination, timeout):
    destination.mkdir(parents=True, exist_ok=True)
    artifact = destination / "schedule.jar"
    with TemporaryDirectory(prefix="diana-schedule-prepare-") as directory:
        original, _ = compile_fixture(Path(directory), timeout)
        write_fixture_jar(artifact, original)
    return artifact


def verify_fixture_jar(artifact, original):
    try:
        with ZipFile(artifact) as archive:
            if archive.namelist() != jar_entries():
                raise ValueError("prepared initializer entries are out of order")
            for entry in archive.infolist():
                authored = (original / entry.filename).read_bytes()
                with archive.open(entry) as source:
                    if entry.file_size != len(authored) or source.read(len(authored) + 1) != authored:
                        raise ValueError(f"{entry.filename} differs from the authored compilation")
    except BadZipFile as error:
        raise ValueError(f"invalid prepared initializer JAR: {error}") from error


def check_raw(name, expected_calls, class_file, raw_output, timeout):
    dump = run([*JAVAP, str(class_file)], timeout)
    raw_output.write_text(dump, encoding="utf-8")
    calls = re.findall(r"^\s*(\d+): invokestatic\s+#\d+\s+// Method "
                       r"ScheduleEffects\.(\w+):", dump, re.MULTILINE)
    if tuple(method for _, method in calls) != expected_calls:
        raise ValueError(f"{name}: unexpected original call order {calls}")
    positions = {method: int(pc) for pc, method in calls}
    if name in ("ConditionalOperand", "ShortOperand"):
        branch = re.search(r"^\s*(\d+): if(?:eq|ne)\b", dump, re.MULTILINE)
        if branch is None or int(calls[0][0]) >= int(branch.group(1)):
            raise ValueError(f"{name}: original capture no longer precedes the branch")
    if name == "ProtectedOperand":
        handler = re.search(r"Exception table:\s*from\s+to\s+target\s+type\s*"
                            r"(\d+)\s+(\d+)\s+(\d+)\s+Class java/lang/IllegalStateException", dump)
        if handler is None:
            raise ValueError("missing typed post-publication fault handler")
        store = re.search(r"^\s*(\d+): putstatic\s+#\d+\s+// Field RESULT:", dump, re.MULTILINE)
        if store is None:
            raise ValueError("missing protected initializer's result publication")
        start, end, target = map(int, handler.groups())
        if not (positions["capture"] < positions["earlier"] < int(store.group(1))
                < start <= positions["after"] < end <= target <= positions["record"]):
            raise ValueError("original effects do not match the typed handler's scope")


def evaluate(classes, support, scenario, mode, timeout):
    classpath = os.pathsep.join(map(str, [classes, support]))
    return run([*JAVA, "-cp", classpath, "CheckSchedule", scenario, str(mode)], timeout).strip()


def recover_class(binary, artifact, at, name, recovered, support, timeout):
    locator = at.split(":", 1)[0]
    recovered.mkdir()
    output = run([str(binary), "get-class", str(artifact), locator, "--format", "json"], timeout)
    (recovered / "publication.json").write_text(output, encoding="utf-8")
    publication = json.loads(output)
    selection = publication["selection"]
    methods = publication["result"]["methods"]
    if (
        selection["class"] != f"L{name};" or selection["class_at"] != locator
        or publication["issues"] or not any(
            (method["at"], method["name"], method["prototype"]) == (at, "<clinit>", "()V")
            for method in methods
        )
    ):
        raise ValueError(f"{name}: incomplete or incorrectly selected class publication")
    source = recovered / f"{name}.java"
    source.write_text(publication["result"]["text"], encoding="utf-8")
    compile_java(recovered, [source], timeout, support)
    dump = run([*JAVAP, str(recovered / f"{name}.class")], timeout)
    (recovered / "recompiled.javap").write_text(dump, encoding="utf-8")
    return publication["result"]["text"]


def verify(binary, artifact, evidence, timeout):
    original, support = compile_fixture(evidence, timeout)
    if artifact is None:
        artifact = evidence / "schedule.jar"
        write_fixture_jar(artifact, original)
    else:
        verify_fixture_jar(artifact, original)
    compile_java(support, [FIXTURE / "CheckSchedule.java"], timeout, original, support)

    checks = {}
    for at, (name, scenario, expected_calls, expectations) in CLASSES.items():
        check_raw(name, expected_calls, original / f"{name}.class",
                  evidence / f"{name}.original.javap", timeout)
        rows = []
        for check, (mode, expected) in expectations.items():
            observed = evaluate(original, support, scenario, mode, timeout)
            if observed != expected:
                raise ValueError(f"{name}/{check}: original {observed!r}, expected {expected!r}")
            rows.append({"id": check, "expected": expected, "original": observed})
        checks[at] = rows
        if binary is None:
            for row in rows:
                row.update(observed=row["original"], status="PASS")
            continue

        recovered = evidence / name
        try:
            source = recover_class(binary, artifact, at, name, recovered, support, timeout)
        except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
            for row in rows:
                row.update(observed=None, status="UNCHECKED", detail=str(error))
            if name == "FirstOperand":
                rows.append({"id": "compact-first-operand", "expected": True, "observed": None,
                             "status": "UNCHECKED", "detail": str(error)})
            continue
        for row in rows:
            try:
                mode, _ = expectations[row["id"]]
                observed = evaluate(recovered, support, scenario, mode, timeout)
                row.update(observed=observed, status="PASS" if observed == row["expected"] else "FAIL")
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                row.update(observed=None, status="UNCHECKED", detail=str(error))
        if name == "FirstOperand":
            compact = re.search(
                r"\bObject\s+RESULT\s*=\s*ScheduleEffects\.joinFirst\("
                r"\s*(?:\((?:java\.lang\.)?Object\)\s*)?ScheduleEffects\.capture\(\),"
                r"\s*(?:\((?:java\.lang\.)?Object\)\s*)?ScheduleEffects\.earlier\(\)\s*\)",
                source,
            ) is not None
            rows.append({"id": "compact-first-operand", "expected": True, "observed": compact,
                         "status": "PASS" if compact else "FAIL"})
    return checks


def probe(binary, artifact, timeout):
    with TemporaryDirectory(prefix="diana-initializer-schedule-") as directory:
        return verify(binary, artifact, Path(directory), timeout)


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
            receipt["binary"] = str(binary)
            receipt["version"] = run([str(binary), "--version"], 20).strip()
        checks = verify(binary, None, evidence, 20)
        receipt["checks"] = checks
        rows = [row for method in checks.values() for row in method]
        failed = [row for row in rows if row["status"] == "FAIL"]
        unchecked = [row for row in rows if row["status"] == "UNCHECKED"]
        for at, method in checks.items():
            for row in method:
                if row["status"] == "FAIL":
                    print(f"{at} {row['id']}: expected {row['expected']}, observed {row['observed']}")
                if row["status"] == "UNCHECKED":
                    print(f"{at} {row['id']} unchecked. {row['detail']}")
        receipt["status"] = "FAIL" if failed else "UNCHECKED" if unchecked else "PASS"
        print(f"{len(rows) - len(failed) - len(unchecked)}/{len(rows)} checks passed")
    except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
        receipt["error"] = str(error)
        print(f"Proof setup failed. {error}")
    args.output.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    return {"PASS": 0, "FAIL": 1, "UNCHECKED": 2, "SETUP_ERROR": 2}[receipt["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
