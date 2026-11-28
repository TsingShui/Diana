"""Compare exact JVM source publication against independently specified signed zeros."""

from argparse import ArgumentParser
from hashlib import sha256
import json
from os import pathsep
from pathlib import Path
import re
import subprocess
import sys
from tempfile import mkdtemp


ROOT = Path(__file__).resolve().parent
JAVA = ["java", "-XX:-UsePerfData"]
JAVAC = [*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none", "-g"]
JAVAP = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-v"]
NAME = "SignedSelections"
SOURCE = f"{NAME}.java"
CLASS = f"{NAME}.class"
CASES = 25


def invoke(args):
    process = subprocess.run(args, text=True, capture_output=True, timeout=20)
    if process.returncode:
        raise ValueError(f"{args[0]} exited with {process.returncode}\n{process.stdout}{process.stderr}")
    return process.stdout


def evaluate(classes, runner):
    output = invoke([*JAVA, "-cp", pathsep.join(map(str, (classes, runner))), "CheckSignedSelections"])
    (classes / "trace.tsv").write_text(output, encoding="utf-8")
    results = {}
    for row in output.splitlines():
        name, expected, observed = row.split("\t")
        if name in results:
            raise ValueError(f"duplicate case {name}")
        results[name] = dict(expected=expected, observed=observed)
    if len(results) != CASES:
        raise ValueError(f"expected {CASES} scenarios, got {len(results)}")
    return results


def raw_evidence(class_file):
    dump = invoke([*JAVAP, str(class_file)])
    class_file.with_suffix(".javap").write_text(dump, encoding="utf-8")
    fields = {field: constant for field, constant in re.findall(
        r"public static final (?:float|double) (FIELD_\w+);\n[^\n]+\n[^\n]+\n\s*ConstantValue: (.+)", dump)}
    return fields


def digest(path):
    hasher = sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    options = parser.parse_args()
    output = options.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(mkdtemp(prefix=f"{output.stem}-", dir=output.parent))
    report = dict(evidence=str(evidence), status="SETUP_ERROR")
    try:
        original = evidence / "original"
        runner = evidence / "runner"
        original.mkdir()
        runner.mkdir()
        invoke([*JAVAC, "-d", str(original), str(ROOT / SOURCE)])
        invoke([*JAVAC, "-cp", str(original), "-d", str(runner), str(ROOT / "CheckSignedSelections.java")])
        input_class = original / CLASS
        expected = evaluate(original, runner)
        expected_fields = {
            "FIELD_NEG_FLOAT": "float -0.0f", "FIELD_POS_FLOAT": "float 0.0f",
            "FIELD_NEG_DOUBLE": "double -0.0d", "FIELD_POS_DOUBLE": "double 0.0d",
        }
        fields = raw_evidence(input_class)
        report.update(original=expected, original_fields=fields, original_sha256=digest(input_class))
        if fields != expected_fields or any(row["expected"] != row["observed"] for row in expected.values()):
            raise ValueError("original contradicts the independently authored witness")
        checks = expected
        if options.binary:
            binary = options.binary.resolve(strict=True)
            report.update(binary_sha256=digest(binary), binary_version=invoke([str(binary), "--version"]).strip())
            recovered = evidence / "recovered"
            recovered.mkdir()
            publication = invoke([str(binary), "get-class", str(input_class), "j0", "--format", "json"])
            (recovered / "publication.json").write_text(publication, encoding="utf-8")
            decoded = json.loads(publication)
            if decoded["selection"]["class"] != f"L{NAME};" or decoded["issues"]:
                raise ValueError(f"missing complete {NAME} publication")
            source = recovered / SOURCE
            source.write_text(decoded["result"]["text"], encoding="utf-8")
            invoke([*JAVAC, "-d", str(recovered), str(source)])
            fields = raw_evidence(recovered / CLASS)
            checks = evaluate(recovered, runner)
            report.update(recovered=checks, recovered_fields=fields)
            if checks.keys() != expected.keys():
                raise ValueError("recompiled publication changed the scenario identities")
        failures = [name for name, row in checks.items() if row["observed"] != expected[name]["expected"]]
        report.update(status="FAIL" if failures or fields != expected_fields else "PASS", failures=failures)
        print(f"{CASES - len(failures)}/{CASES} runtime checks passed. Receipt at {output}")
        if fields != expected_fields:
            print(f"Encoded field signs changed {fields}")
        for name in failures:
            print(f"{name} expected {expected[name]['expected']}, observed {checks[name]['observed']}")
    except (KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
        print(f"Proof setup failed. {error}. Receipt at {output}", file=sys.stderr)
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return {"PASS": 0, "FAIL": 1, "SETUP_ERROR": 2}[report["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
