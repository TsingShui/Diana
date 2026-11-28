"""Run the original interface snapshots and recompiled Diana publications."""

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
    "j0": ("InterfaceSnapshot", {"basic": "true/true/1"}),
    "j1": ("ProtectedSnapshot", {"protected": "true/true/1", "fault": "true/true/124"}),
    "j2": ("StableSnapshot", {"stable": "true/true/31"}),
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
    compile_java(support, [FIXTURE / "SnapshotEffects.java"], timeout)
    compile_java(original, [FIXTURE / f"{name}.java" for name, _ in CLASSES.values()],
                 timeout, support)
    return original, support


def jar_entries():
    return [f"{name}.class" for name, _ in CLASSES.values()]


def write_fixture_jar(artifact, original):
    with ZipFile(artifact, "x") as archive:
        for entry in jar_entries():
            archive.writestr(ZipInfo(entry), (original / entry).read_bytes())


def prepare(destination, timeout):
    destination.mkdir(parents=True, exist_ok=True)
    artifact = destination / "snapshots.jar"
    with TemporaryDirectory(prefix="diana-snapshot-prepare-") as directory:
        original, _ = compile_fixture(Path(directory), timeout)
        write_fixture_jar(artifact, original)
    return artifact


def verify_fixture_jar(artifact, original):
    try:
        with ZipFile(artifact) as archive:
            if archive.namelist() != jar_entries():
                raise ValueError("prepared snapshot entries are out of order")
            for entry in archive.infolist():
                authored = (original / entry.filename).read_bytes()
                with archive.open(entry) as source:
                    if entry.file_size != len(authored) or source.read(len(authored) + 1) != authored:
                        raise ValueError(f"{entry.filename} differs from the authored compilation")
    except BadZipFile as error:
        raise ValueError(f"invalid prepared snapshot JAR: {error}") from error


def check_raw(name, class_file, raw_output, timeout):
    dump = run([*JAVAP, str(class_file)], timeout)
    raw_output.write_text(dump, encoding="utf-8")
    owner = "StableOwner" if name == "StableSnapshot" else "SnapshotEffects"
    capture = re.findall(rf"^\s*(\d+): getstatic\s+#\d+\s+// Field {owner}\.INSTANCE:"
                         r"Ljava/lang/Object;\s*$", dump, re.MULTILINE)
    stores = re.findall(r"^\s*(\d+): putstatic\s+#\d+\s+// Field RESULT:Ljava/lang/Object;\s*$",
                        dump, re.MULTILINE)
    replacements = re.findall(r"^\s*(\d+): invokestatic\s+#\d+\s+// Method "
                              r"SnapshotEffects\.replace:\(Ljava/lang/Object;Z\)V\s*$",
                              dump, re.MULTILINE)
    if len(capture) != 1 or len(stores) != 1 or len(replacements) != 1:
        raise ValueError(f"{name}: expected one capture, replacement call, and field write")
    capture_pc, replace_pc, store_pc = map(int, (capture[0], replacements[0], stores[0]))
    if not capture_pc < replace_pc < store_pc:
        raise ValueError(f"{name}: original initializer order changed")
    if name == "ProtectedSnapshot":
        handler = re.search(r"Exception table:\s*from\s+to\s+target\s+type\s*"
                            r"(\d+)\s+(\d+)\s+(\d+)\s+Class java/lang/IllegalStateException", dump)
        if handler is None:
            raise ValueError("missing typed replacement fault handler")
        start, end, target = map(int, handler.groups())
        if not capture_pc < start <= replace_pc < end <= target < store_pc:
            raise ValueError("capture or replacement moved across original handler boundary")


def evaluate(classes, support, scenario, timeout):
    classpath = os.pathsep.join(map(str, [classes, support]))
    return run([*JAVA, "-cp", classpath, "CheckSnapshot", scenario], timeout).strip()


def recover_class(binary, artifact, locator, name, recovered, support, timeout):
    recovered.mkdir()
    output = run([str(binary), "get-class", str(artifact), locator, "--format", "json"], timeout)
    (recovered / "publication.json").write_text(output, encoding="utf-8")
    publication = json.loads(output)
    selection = publication["selection"]
    methods = publication["result"]["methods"]
    if (
        selection["class"] != f"L{name};" or selection["class_at"] != locator
        or publication["issues"] or len(methods) != 1
        or (methods[0]["at"], methods[0]["name"], methods[0]["prototype"])
        != (f"{locator}:m0", "<clinit>", "()V")
    ):
        raise ValueError(f"{name}: incomplete or incorrectly selected class publication")
    source = recovered / f"{name}.java"
    source.write_text(publication["result"]["text"], encoding="utf-8")
    compile_java(recovered, [source], timeout, support)
    dump = run([*JAVAP, str(recovered / f"{name}.class")], timeout)
    (recovered / "recompiled.javap").write_text(dump, encoding="utf-8")


def verify(binary, artifact, evidence, timeout):
    original, support = compile_fixture(evidence, timeout)
    if artifact is None:
        artifact = evidence / "snapshots.jar"
        write_fixture_jar(artifact, original)
    else:
        verify_fixture_jar(artifact, original)
    compile_java(support, [FIXTURE / "CheckSnapshot.java"], timeout, original, support)

    checks = {}
    for locator, (name, expectations) in CLASSES.items():
        check_raw(name, original / f"{name}.class", evidence / f"{name}.original.javap", timeout)
        rows = []
        for scenario, expected in expectations.items():
            observed = evaluate(original, support, scenario, timeout)
            if observed != expected:
                raise ValueError(f"{scenario}: original {observed!r}, expected {expected!r}")
            rows.append({"id": scenario, "expected": expected, "original": observed})
        checks[f"{locator}:m0"] = rows
        if binary is None:
            for row in rows:
                row.update(observed=row["original"], status="PASS")
            continue

        recovered = evidence / name
        try:
            recover_class(binary, artifact, locator, name, recovered, support, timeout)
        except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
            for row in rows:
                row.update(observed=None, status="UNCHECKED", detail=str(error))
            continue
        for row in rows:
            try:
                observed = evaluate(recovered, support, row["id"], timeout)
                row.update(observed=observed, status="PASS" if observed == row["expected"] else "FAIL")
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                row.update(observed=None, status="UNCHECKED", detail=str(error))
    return checks


def probe(binary, artifact, timeout):
    with TemporaryDirectory(prefix="diana-interface-snapshot-") as directory:
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
        for row in failed:
            print(f"{row['id']}: expected {row['expected']}, observed {row['observed']}")
        for row in unchecked:
            print(f"{row['id']} unchecked. {row['detail']}")
        receipt["status"] = "FAIL" if failed else "UNCHECKED" if unchecked else "PASS"
        print(f"{len(rows) - len(failed) - len(unchecked)}/{len(rows)} runtime scenarios passed")
    except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
        receipt["error"] = str(error)
        print(f"Proof setup failed. {error}")
    args.output.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    return {"PASS": 0, "FAIL": 1, "UNCHECKED": 2, "SETUP_ERROR": 2}[receipt["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
