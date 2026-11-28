"""Recompile complete Diana output and check source behavior and monitor instruction count."""

from argparse import ArgumentParser
from collections import Counter
from hashlib import sha256
import json
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parent
SOURCE = ROOT / "MonitorContract.java"
RUNNER = ROOT / "CheckMonitorContract.java"
JAVA = ["java", "-XX:-UsePerfData"]
EXPECTED = {
    "different": ("(Ljava/lang/Object;)I", True, False, 1, 2),
    "differentStatic": ("(Ljava/lang/Object;)I", True, False, 1, 2),
    "sameReentry": ("()I", True, False, 1, 2),
    "exceptional": ("(Ljava/lang/Object;)V", True, False, 1, 1),
    "exceptionalStatic": ("(Ljava/lang/Object;)V", True, False, 1, 1),
    "implicit": ("()I", True, False, 0, 0),
    "implicitStatic": ("()I", True, False, 0, 0),
    "explicitOnly": ("(Ljava/lang/Object;)I", False, False, 1, 2),
    "nativeInstance": ("()I", True, True, 0, 0),
    "nativeStatic": ("()I", True, True, 0, 0),
    "nestedExplicit": ("(Ljava/lang/Object;)V", False, False, 2, 4),
}
ACC_NATIVE = 0x0100
ACC_SYNCHRONIZED = 0x0020


def invoke(argv):
    return subprocess.run(argv, capture_output=True, check=True, text=True, timeout=20)


def compile_source(source, directory):
    return invoke([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none",
                   "-d", str(directory), str(source), str(RUNNER)]).stderr


def runtime(directory):
    result = {}
    output = invoke([*JAVA, "-cp", str(directory), "CheckMonitorContract"]).stdout
    (directory / "trace.tsv").write_text(output, encoding="utf-8")
    for row in output.splitlines():
        name, status, expected, observed = row.split("\t")
        if name in result or status not in {"PASS", "FAIL"}:
            raise ValueError(f"unexpected case result {row!r}")
        result[name] = dict(status=status, expected=expected, observed=observed)
    if len(result) != 28:
        raise ValueError(f"expected all 28 behavioral or reflection cases, got {len(result)}")
    return result


def structure(directory):
    argv = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-v",
            str(directory / "MonitorContract.class")]
    text = invoke(argv).stdout
    (directory / "MonitorContract.javap").write_text(text, encoding="utf-8")
    lines = text.splitlines()
    result = {}
    for index, line in enumerate(lines):
        if not line.startswith("    descriptor:"):
            continue
        name = lines[index - 1].partition("(")[0].strip().split()[-1]
        if name not in EXPECTED or "(" not in lines[index - 1]:
            continue
        descriptor = line.partition(":")[2].strip()
        exp_descriptor, exp_sync, exp_native, exp_enters, exp_exits = EXPECTED[name]
        if name in result or descriptor != exp_descriptor:
            raise ValueError(f"unexpected or duplicate original descriptor {name}{descriptor}")
        flag_line = re.fullmatch(r"\s+flags: \((0x[0-9a-fA-F]+)\).*", lines[index + 1])
        if not flag_line:
            raise ValueError(f"method {name} has no classfile flags")
        flags = int(flag_line[1], 16)
        tail = []
        for instruction in lines[index + 2:]:
            if re.match(r"^(?:  \S|})", instruction):
                break
            tail.append(instruction)
        def count(opcode):
            return len([row for row in tail if re.fullmatch(rf"\s+\d+:\s+{opcode}\s*", row)])

        entered = count("monitorenter")
        exited = count("monitorexit")
        synchronized = bool(flags & ACC_SYNCHRONIZED)
        native = bool(flags & ACC_NATIVE)
        status = "PASS" if (synchronized, native, entered, exited) == (exp_sync, exp_native, exp_enters, exp_exits) else "FAIL"
        result[name] = dict(descriptor=descriptor, flags=hex(flags), acc_synchronized=synchronized,
                            acc_native=native, monitorenter=entered, monitorexit=exited,
                            expected=dict(acc_synchronized=exp_sync, acc_native=exp_native,
                                          monitorenter=exp_enters, monitorexit=exp_exits), status=status)
    if set(result) != EXPECTED.keys():
        raise ValueError(f"missing expected method descriptors: {EXPECTED.keys() - result.keys()}")
    return result


def compare(directory, source):
    diagnostics = compile_source(source, directory)
    return dict(runtime=runtime(directory), structure=structure(directory), compile_diagnostics=diagnostics)


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    output = args.output.resolve()
    evidence = output.with_suffix("")
    original_dir = evidence / "original"
    original_dir.mkdir(parents=True, exist_ok=True)
    original = compare(original_dir, SOURCE)
    if any(row["status"] != "PASS" for key in ("runtime", "structure") for row in original[key].values()):
        raise ValueError(f"original verification disagrees with the authored model: {original}")
    original_class = original_dir / "MonitorContract.class"
    result = dict(original=original, original_sha256=digest(original_class))

    if args.binary:
        binary = args.binary.resolve(strict=True)
        recovered_dir = evidence / "recovered"
        recovered_dir.mkdir(exist_ok=True)
        published = invoke([str(binary), "get-class", str(original_class), "j0", "--format", "json"]).stdout
        observation = json.loads(published)
        if observation["issues"]:
            raise ValueError(f"complete recovered class had issues: {observation['issues']}")
        (recovered_dir / "publication.json").write_text(published, encoding="utf-8")
        recovered_source = recovered_dir / SOURCE.name
        recovered_source.write_text(observation["result"]["text"], encoding="utf-8")
        captured = compare(recovered_dir, recovered_source)
        if set(captured["runtime"]) != set(original["runtime"]):
            raise ValueError("candidate executed a different set of expected scenarios")
        result.update(binary=str(binary), binary_sha256=digest(binary), recovered=captured)
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")

    checks = result.get("recovered", original)
    failed = False
    for key in ("runtime", "structure"):
        totals = Counter(row["status"] for row in checks[key].values())
        print(key, dict(totals))
        for name, row in checks[key].items():
            if row["status"] == "FAIL":
                observed = {fact: row[fact] for fact in row["expected"]} if key == "structure" else row["observed"]
                print(name, row["expected"], observed)
                failed = True
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
