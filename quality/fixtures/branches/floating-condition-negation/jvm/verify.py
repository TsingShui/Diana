"""Check exact JVM publication of asymmetric branches over unordered values."""

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
JAVAP = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-s", "-c"]
SOURCE = "FloatingBranches.java"
CLASS = "FloatingBranches.class"
SHAPES = {
    "floatLow": ("(FF)V", "fcmpg", "ifge"),
    "floatHigh": ("(FF)V", "fcmpl", "ifle"),
    "doubleLow": ("(DD)V", "dcmpg", "ifge"),
    "doubleHigh": ("(DD)V", "dcmpl", "ifle"),
    "gated": ("(FFZ)V", "fcmpg", "ifge"),
    "separateReturns": ("(FF)V", "fcmpg", "ifge"),
    "integralLow": ("(II)V", None, "if_icmpge"),
}
CASE_COUNT = 47


def invoke(args):
    process = subprocess.run(args, capture_output=True, text=True, timeout=20)
    if process.returncode:
        raise ValueError(f"{args[0]} exited with {process.returncode}\n{process.stdout}{process.stderr}")
    return process.stdout


def runtime(classes, runner):
    path = pathsep.join(map(str, (classes, runner)))
    output = invoke([*JAVA, "-cp", path, "CheckFloatingBranches"])
    (classes / "trace.tsv").write_text(output, encoding="utf-8")
    checks = {}
    for row in output.splitlines():
        name, expected, observed = row.split("\t")
        if name in checks:
            raise ValueError(f"duplicate run {name}")
        checks[name] = dict(expected=int(expected), observed=int(observed))
    if len(checks) != CASE_COUNT:
        raise ValueError(f"expected {CASE_COUNT} scenarios, observed {len(checks)}")
    return checks


def instructions(lines, return_type, name, descriptor):
    header = re.compile(rf"^  public static {return_type} {name}\([^)]*\);$")
    for index, line in enumerate(lines[:-1]):
        if header.fullmatch(line) and lines[index + 1].strip() == f"descriptor: {descriptor}":
            break
    else:
        raise ValueError(f"javap did not identify {name}{descriptor}")
    code = []
    for line in lines[index + 2:]:
        if re.match(r"^(?:  \S|})", line):
            break
        if instruction := re.match(r"\s*(\d+):\s+(\w+)(?:\s+(.*))?$", line):
            code.append((int(instruction[1]), instruction[2], instruction[3] or ""))
    return code


def original_shapes(class_file):
    listing = invoke([*JAVAP, str(class_file)])
    class_file.with_suffix(".javap").write_text(listing, encoding="utf-8")
    lines = listing.splitlines()
    evidence = {}
    for name, (descriptor, cmp_op, branch_op) in SHAPES.items():
        code = instructions(lines, "void", name, descriptor)
        by_offset = {offset: (op, arg) for offset, op, arg in code}
        branch_pc = 3 if cmp_op else 2
        if cmp_op and by_offset.get(2) != (cmp_op, ""):
            raise ValueError(f"wrong comparison opcode in {name}{descriptor}")
        branch, target = by_offset[branch_pc]
        if branch != branch_op or by_offset.get(int(target)) != ("bipush", "9"):
            raise ValueError(f"wrong short-arm branch in {name}{descriptor}")
        returns = [offset for offset, op, _ in code if op == "return"]
        joins = [int(arg) for _, op, arg in code if op == "goto"]
        if name == "separateReturns":
            if len(returns) != 2 or joins:
                raise ValueError(f"expected two physical return sites in {name}{descriptor}")
        elif len(returns) != 1 or joins != returns:
            raise ValueError(f"expected two arms joined at one return in {name}{descriptor}")
        if name == "gated":
            if not any(branch_pc < offset < int(target) and op == "invokestatic"
                       and arg.endswith("// Method open:(Z)Z") for offset, op, arg in code):
                raise ValueError("missing gated fallthrough side effect")
        evidence[name + descriptor] = dict(cmp=cmp_op, branch=branch, short_arm=int(target), returns=returns)
    equality = instructions(lines, "int", "selfEqual", "(D)I")
    if len(equality) < 4 or equality[:3] != [(0, "dload_0", ""), (1, "dload_0", ""), (2, "dcmpl", "")]:
        raise ValueError("expected self comparison of the same double")
    offset, branch, target = equality[3]
    if (offset, branch) != (3, "ifne") or (int(target), "iconst_1", "") not in equality:
        raise ValueError("expected unordered self equality to branch to the value 1")
    evidence["selfEqual(D)I"] = dict(cmp="dcmpl", branch=branch, unordered_arm=int(target))
    return evidence


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
        invoke([*JAVAC, "-cp", str(original), "-d", str(runner), str(ROOT / "CheckFloatingBranches.java")])
        input_class = original / CLASS
        expected = runtime(original, runner)
        report.update(original=expected, original_sha256=digest(input_class), original_shapes=original_shapes(input_class))
        if any(row["expected"] != row["observed"] for row in expected.values()):
            raise ValueError("the original disagrees with the independently authored witness")
        checks = expected
        if options.binary:
            binary = options.binary.resolve(strict=True)
            report.update(binary=str(binary), binary_sha256=digest(binary),
                          binary_version=invoke([str(binary), "--version"]).strip())
            recovered = evidence / "recovered"
            recovered.mkdir()
            raw = invoke([str(binary), "get-class", str(input_class), "j0", "--format", "json"])
            (recovered / "publication.json").write_text(raw, encoding="utf-8")
            publication = json.loads(raw)
            if publication["selection"]["class"] != "LFloatingBranches;" or publication["issues"]:
                raise ValueError("missing complete FloatingBranches class publication")
            source = recovered / SOURCE
            source.write_text(publication["result"]["text"], encoding="utf-8")
            invoke([*JAVAC, "-d", str(recovered), str(source)])
            checks = runtime(recovered, runner)
            report["recovered"] = checks
            if checks.keys() != expected.keys():
                raise ValueError("recovered execution changed the scenario identities")
        failures = [name for name, row in checks.items() if row["observed"] != expected[name]["expected"]]
        report.update(status="FAIL" if failures else "PASS", failures=failures)
        print(f"{len(checks) - len(failures)}/{len(checks)} checks passed. Receipt at {output}")
        if failures:
            first = failures[0]
            print(f"{first} expected {expected[first]['expected']}, observed {checks[first]['observed']}")
    except (KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
        print(f"Proof setup failed. {error}. Receipt at {output}", file=sys.stderr)
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return {"PASS": 0, "FAIL": 1, "SETUP_ERROR": 2}[report["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
