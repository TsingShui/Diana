"""Check exact JVM source publication of authored virtual dispatch and String switches."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
from tempfile import TemporaryDirectory, mkdtemp


FIXTURE = Path(__file__).resolve().parent
SOURCE = "StringDispatch.java"
CLASS = "StringDispatch.class"
DESCRIPTOR = "LStringDispatch;"
ORACLE_ENV = {key: value for key, value in os.environ.items() if key not in (
    "CLASSPATH", "JDK_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS",
)}
JAVA = ["java", "-XX:-UsePerfData"]
JAVAC = [
    *JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main",
    "-source", "17", "-target", "17", "-Xlint:-options", "-proc:none", "-g",
]
JAVAP = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-c", "-s"]
METHODS = {
    "j0:m1": ("custom", "(LSwitchKey;)I", "SwitchKey", "Ljava/lang/Object;", 1, 6),
    "j0:m2": (
        "overloaded", "(LOverloadedSwitchKey;)I", "OverloadedSwitchKey", "Ljava/lang/String;", 2, 7,
    ),
    "j0:m3": ("genuine", "(Ljava/lang/String;)I", "java/lang/String", "Ljava/lang/Object;", 2, 4),
    "j0:m4": ("genuineCollision", "(Ljava/lang/String;)I", "java/lang/String", "Ljava/lang/Object;", 2, 4),
}
METHOD = re.compile(
    r"^  public static int (\w+)\([^\n]+\);\n    descriptor: (\S+)\n(.*?)(?=^  \S|\Z)",
    re.MULTILINE | re.DOTALL,
)
CALL = re.compile(r"^\s*\d+: invokevirtual\s+#\d+\s+// Method (\S+)$", re.MULTILINE)


def run(command, timeout):
    process = subprocess.run(
        command, text=True, capture_output=True, timeout=timeout, env=ORACLE_ENV,
    )
    if process.returncode:
        raise ValueError(
            f"{command[0]} exited with {process.returncode}\n"
            f"{process.stdout}{process.stderr}"
        )
    return process.stdout


def compile_java(output, sources, timeout, support=None):
    output.mkdir(parents=True, exist_ok=True)
    classpath = ["-classpath", str(support)] if support else []
    run([*JAVAC, *classpath, "-d", str(output), *map(str, sources)], timeout)


def prepare(output, timeout):
    compile_java(output, [FIXTURE / SOURCE, FIXTURE / "SwitchKey.java"], timeout)
    return output / CLASS


def verify_raw(class_file, directory, timeout):
    dump = run([*JAVAP, str(class_file)], timeout)
    (directory / "original.javap").write_text(dump, encoding="utf-8")
    blocks = {name: (descriptor, body) for name, descriptor, body in METHOD.findall(dump)}
    for name, descriptor, owner, equals_parameter, equals_count, _ in METHODS.values():
        seen_descriptor, body = blocks[name]
        calls = [f"{owner}.hashCode:()I", *[f"{owner}.equals:({equals_parameter})Z"] * equals_count]
        if (
            seen_descriptor != descriptor or CALL.findall(body) != calls
            or len(re.findall(r"\b(?:lookup|table)switch\b", body)) != 2
        ):
            raise ValueError(f"{name}: original instructions do not contain the switch protocol")
        if name in ("overloaded", "genuineCollision") and not re.search(r"\b2112:", body):
            raise ValueError(f"{name}: the original switch lost its collision bucket")


def evaluate(method, count, classes, support, timeout):
    classpath = os.pathsep.join(map(str, [classes, support]))
    output = run([*JAVA, "-cp", classpath, "CheckStringDispatch", method], timeout)
    rows = {}
    for line in output.splitlines():
        scenario, expected, observed = line.split("\t")
        if scenario in rows:
            raise ValueError(f"{method}: repeated result {scenario}")
        rows[scenario] = (expected, observed)
    if len(rows) != count:
        raise ValueError(f"{method}: expected {count} scenarios, saw {len(rows)}")
    return rows


def result(check, expected, observed, status=None, detail=None):
    row = {
        "id": check, "expected": expected, "observed": observed,
        "status": status or ("PASS" if observed == expected else "FAIL"),
    }
    if detail:
        row["detail"] = detail
    return row


def verify_publication(binary, artifact, locator, name, descriptor, directory, timeout):
    text = run([str(binary), "get-method", str(artifact), locator, "--format", "json"], timeout)
    (directory / "publication.json").write_text(text, encoding="utf-8")
    data = json.loads(text)
    source = data["result"]["text"]
    methods = data["result"]["methods"]
    if (
        data["selection"]["class"] != DESCRIPTOR
        or data["selection"]["method_at"] != locator or data["issues"]
        or len(methods) != 1
        or (methods[0]["at"], methods[0]["name"], methods[0]["prototype"]) != (locator, name, descriptor)
        or not source.strip()
    ):
        raise ValueError(f"{name}: incomplete or incorrectly selected publication")
    return source


def verify(binary, artifact, directory, timeout):
    original = directory / "original"
    support = directory / "support"
    original_class = prepare(original, timeout)
    if artifact is not None and artifact.read_bytes() != original_class.read_bytes():
        raise ValueError("prepared class differs from this fixture's fresh javac output")
    verify_raw(original_class, directory, timeout)
    compile_java(
        support, [FIXTURE / "SwitchKey.java", FIXTURE / "CheckStringDispatch.java"], timeout, original,
    )
    checks = {}
    for locator, (name, descriptor, owner, _, equals_count, count) in METHODS.items():
        expected = evaluate(name, count, original, support, timeout)
        if any(authored != observed for authored, observed in expected.values()):
            raise ValueError(f"{name}: original contradicts the authored results")
        if binary is None:
            checks[locator] = [result(scenario, *values) for scenario, values in expected.items()]
            continue
        recovered = directory / name
        recovered.mkdir()
        source_checks = {}
        if owner != "java/lang/String":
            source_checks["virtual-dispatch"] = {"hashCode": 1, "equals": equals_count}
        if name == "genuine":
            source_checks["ordinary-String-recovery"] = True
        rows = []
        try:
            source = verify_publication(binary, artifact or original_class, locator, name, descriptor,
                                        recovered, timeout)
            rendered = recovered / SOURCE
            rendered.write_text(source, encoding="utf-8")
            has_string_switch = re.search(r'\bcase\s+"[^"\n]+"\s*:', source) is not None
            if "virtual-dispatch" in source_checks:
                expected_calls = source_checks["virtual-dispatch"]
                calls = {method: len(re.findall(rf"\.{method}\s*\(", source))
                         for method in expected_calls}
                status = "PASS" if calls == expected_calls else "UNCHECKED"
                detail = None
                if has_string_switch and not any(calls.values()):
                    status = "FAIL"
                    detail = "String labels replaced both virtual calls on a custom class."
                elif status == "UNCHECKED":
                    detail = "The publication has a different call shape."
                rows.append(result(
                    "virtual-dispatch", expected_calls, calls, status, detail,
                ))
            if "ordinary-String-recovery" in source_checks:
                rows.append(result("ordinary-String-recovery", True, has_string_switch))
            compile_java(recovered, [rendered], timeout, support)
            observed = evaluate(name, count, recovered, support, timeout)
            if observed.keys() != expected.keys():
                raise ValueError(f"{name}: recovered execution has different scenario identities")
            rows.extend(result(scenario, values[0], observed[scenario][1]) for scenario, values in expected.items())
        except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
            if not rows:
                rows.extend(result(check, expectation, None, "UNCHECKED", str(error))
                            for check, expectation in source_checks.items())
            rows.extend(result(scenario, values[0], None, "UNCHECKED", str(error))
                        for scenario, values in expected.items())
        checks[locator] = rows
    return checks


def probe(binary, artifact, timeout):
    with TemporaryDirectory(prefix="diana-string-switch-") as directory:
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

    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(mkdtemp(prefix=f"{output.stem}-", dir=output.parent))
    report = {"evidence": str(evidence)}
    try:
        binary = args.binary.resolve(strict=True) if args.binary else None
        if binary:
            report["version"] = run([str(binary), "--version"], 20).strip()
        checks = verify(binary, None, evidence, 20)
        report["checks"] = checks
        rows = [row for method in checks.values() for row in method]
        failures = [row for row in rows if row["status"] == "FAIL"]
        unchecked = [row for row in rows if row["status"] == "UNCHECKED"]
        report["status"] = "FAIL" if failures else "UNCHECKED" if unchecked else "PASS"
        print(f"{len(rows) - len(failures) - len(unchecked)}/{len(rows)} checks passed")
        for row in failures:
            print(f"{row['id']} expected {row['expected']}, observed {row['observed']}")
        if unchecked:
            print(f"{len(unchecked)} checks unchecked. {unchecked[0]['detail']}")
    except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
        report.update(status="SETUP_ERROR", detail=str(error))
        print(f"Proof setup failed. {error}")
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"Receipt at {output}")
    return {"PASS": 0, "FAIL": 1, "UNCHECKED": 2, "SETUP_ERROR": 2}[report["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
