"""Execute one independently authored runner against JVM bytecode and exact Diana output."""

from argparse import ArgumentParser
from collections import Counter
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import mkdtemp


ROOT = Path(__file__).resolve().parent
RUNNER = ROOT / "CheckFloatCompare.java"
JAVA = ["java", "-XX:-UsePerfData"]
JAVAC = [*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none"]
JAVAP = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-s", "-c"]
CLASS_FILE = Path("fixture/FloatCompare.class")
CASE_COUNT = 75


def run(options, diagnostic):
    completed = subprocess.run(options, text=True, capture_output=True, timeout=30)
    diagnostic.with_suffix(".stdout").write_text(completed.stdout, encoding="utf-8")
    diagnostic.with_suffix(".stderr").write_text(completed.stderr, encoding="utf-8")
    if completed.returncode:
        raise ValueError(f"{options[0]} exited with {completed.returncode}: {completed.stderr.strip()}")
    return completed.stdout


def execution(classes, runner, evidence):
    classpath = os.pathsep.join([str(classes), str(runner)])
    output = run([*JAVA, "-cp", classpath, "CheckFloatCompare"], evidence / "execution")
    (evidence / "trace.tsv").write_text(output, encoding="utf-8")
    rows = {}
    for line in output.splitlines():
        name, state, expected, observed, trace = line.split("\t")
        if not name or name in rows or state not in {"PASS", "FAIL"}:
            raise ValueError(f"unexpected runner receipt row {line!r}")
        trace = int(trace)
        if (state == "PASS") != (expected == observed and trace == 12):
            raise ValueError(f"runner result disagrees with its reported observations {line!r}")
        rows[name] = dict(status=state, expected=expected, observed=observed, trace=trace)
    if len(rows) != CASE_COUNT:
        raise ValueError(f"expected {CASE_COUNT} distinct receipts, got {len(rows)}")
    return rows


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def build_original(evidence):
    runner = evidence / "runner"
    directory = evidence / "original"
    classes = directory / "classes"
    class_file = classes / CLASS_FILE
    classes.mkdir(parents=True)
    runner.mkdir()
    run([sys.executable, str(ROOT / "make_jvm.py"), str(class_file)], directory / "generate")
    run([*JAVAP, str(class_file)], directory / "javap")
    run([*JAVAC, "-cp", str(classes), "-d", str(runner), str(RUNNER)], evidence / "compile_runner")
    return class_file, runner, execution(classes, runner, directory)


def evaluate(binary, input_file, runner, evidence):
    directory = evidence / "recovered"
    classes = directory / "classes"
    classes.mkdir(parents=True)
    published = run([str(binary), "get-class", str(input_file), "j0", "--format", "json"],
                    directory / "publication")
    publication = json.loads(published)
    (directory / "publication.json").write_text(published, encoding="utf-8")
    selection = publication["selection"]
    if selection["class_at"] != "j0" or selection["class"] != "Lfixture/FloatCompare;":
        raise ValueError(f"unexpected class publication: {selection!r}")
    if publication["issues"]:
        raise ValueError(f"class publication has issues: {publication['issues']!r}")
    source = directory / "FloatCompare.java"
    source.write_text(publication["result"]["text"], encoding="utf-8")
    run([*JAVAC, "-d", str(classes), str(source)], directory / "compile_class")
    run([*JAVAP, str(classes / CLASS_FILE)], directory / "javap")
    return execution(classes, runner, directory)


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(mkdtemp(prefix=f"{output.stem}-", dir=output.parent))
    result = dict(evidence=str(evidence), runner_sha256=digest(RUNNER), status="SETUP_ERROR")
    try:
        result["java_version"] = run([*JAVA, "--version"], evidence / "java_version").strip()
        original_file, runner, original = build_original(evidence)
        result.update(original_sha256=digest(original_file), original=original)
        if any(row["status"] != "PASS" for row in original.values()):
            raise ValueError("the original JVM class disagrees with the independently authored runner")
        checks = original
        if args.binary:
            binary = args.binary.resolve(strict=True)
            result.update(binary=str(binary), binary_sha256=digest(binary))
            result["binary_version"] = run([str(binary), "--version"], evidence / "binary_version").strip()
            checks = evaluate(binary, original_file, runner, evidence)
            result["recovered"] = checks
            if {name: row["expected"] for name, row in checks.items()} != {
                name: row["expected"] for name, row in original.items()
            }:
                raise ValueError("the recovered class changed the scenario set or its expectations")
        counts = Counter(row["status"] for row in checks.values())
        result["summary"] = {key: counts[key] for key in ("PASS", "FAIL")}
        result["status"] = "FAIL" if counts["FAIL"] else "PASS"
        print(f"{counts['PASS']}/{CASE_COUNT} passed. Receipt at {output}")
        for name, row in checks.items():
            if row["status"] == "FAIL":
                print(f"{name}: expected {row['expected']} with trace 12; "
                      f"observed {row['observed']} with trace {row['trace']}")
                break
    except (KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        result["error"] = str(error)
        print(f"Proof setup failed. {error}. Receipt at {output}", file=sys.stderr)
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return {"PASS": 0, "FAIL": 1, "SETUP_ERROR": 2}[result["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
