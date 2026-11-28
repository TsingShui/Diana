"""Check exact Diana class publications for hidden instance field identity."""

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
SUBJECTS = ("FieldChild", "FieldPeer")
RESULTS = {
    "dispatch": "137,205",
    "parent-before": "37,37,5",
    "parent-write": "49,49,5",
    "both-slots": "1107,11,7",
    "peer-read": "37,1",
    "peer-write": "66,5,11,7,2",
    "peer-null-read": "NullPointerException,1",
    "peer-null-write": "NullPointerException,1",
}
PARENT_FIELD = "FieldParent.slot:I"
CHILD_FIELD = "FieldChild.slot:I"
RECEIVER = "FieldPeer.observed:(LFieldChild;)LFieldChild;"
BINDINGS = {
    "FieldChild": {
        "readParent": ("()I", [("getfield", PARENT_FIELD)]),
        "writeParent": ("(I)V", [("putfield", PARENT_FIELD)]),
        "writeThenReadBoth": ("(I)I", [
            ("putfield", PARENT_FIELD), ("getfield", CHILD_FIELD), ("putfield", CHILD_FIELD),
            ("getfield", PARENT_FIELD), ("getfield", CHILD_FIELD),
        ]),
        "dispatch": ("()I", [("getfield", CHILD_FIELD)]),
        "callParent": ("()I", [("invokespecial", "FieldParent.dispatch:()I")]),
    },
    "FieldPeer": {
        "readPeer": ("(LFieldChild;)I", [("invokestatic", RECEIVER), ("getfield", PARENT_FIELD)]),
        "writePeer": ("(LFieldChild;I)V", [("invokestatic", RECEIVER), ("putfield", PARENT_FIELD)]),
    },
}


def invoke(argv):
    process = subprocess.run(argv, text=True, capture_output=True, timeout=20)
    if process.returncode:
        raise ValueError(f"{argv[0]} exited with {process.returncode}\n{process.stdout}{process.stderr}")
    return process.stdout


def compile_java(destination, sources, *dependencies):
    args = [*JAVAC, "-d", str(destination)]
    if dependencies:
        args.extend(["-cp", pathsep.join(map(str, dependencies))])
    invoke([*args, *map(str, sources)])


def runtime(classes, support):
    output = invoke([*JAVA, "-cp", pathsep.join(map(str, [classes, support])), "CheckFieldOwners"])
    (classes / "trace.tsv").write_text(output, encoding="utf-8")
    observed = {}
    for line in output.splitlines():
        name, result = line.split("\t")
        if name in observed:
            raise ValueError(f"duplicate observation {name}")
        observed[name] = result
    if observed.keys() != RESULTS.keys():
        raise ValueError(f"unexpected scenario keys {list(observed)}")
    return {name: dict(expected=expected, observed=observed[name], pass_=expected == observed[name])
            for name, expected in RESULTS.items()}


def method_bindings(listing, owner, name, descriptor):
    lines = listing.splitlines()
    header = re.compile(rf"^  \S.*\b{name}\(.*\);$")
    for index, line in enumerate(lines[:-1]):
        if header.fullmatch(line) and lines[index + 1].strip() == f"descriptor: {descriptor}":
            break
    else:
        raise ValueError(f"javap could not identify {owner}.{name}{descriptor}")
    sites = []
    for line in lines[index + 2:]:
        if re.match(r"^(?:  \S|})", line):
            break
        site = re.fullmatch(
            r"\s*\d+:\s+(getfield|putfield|invokespecial|invokestatic)\s+#\d+\s+// (?:Field|Method) (\S+)",
            line,
        )
        if site:
            opcode, reference = site.groups()
            if "." not in reference.partition(":")[0]:
                reference = f"{owner}.{reference}"
            sites.append((opcode, reference))
    return sites


def check_bindings(classes):
    result = {}
    for owner, methods in BINDINGS.items():
        listing = invoke([*JAVAP, str(classes / f"{owner}.class")])
        (classes / f"{owner}.javap").write_text(listing, encoding="utf-8")
        for name, (descriptor, expected) in methods.items():
            observed = method_bindings(listing, owner, name, descriptor)
            result[f"{owner}.{name}{descriptor}"] = dict(expected=expected, observed=observed,
                                                          pass_=expected == observed)
    return result


def original_classes(evidence):
    support, original = evidence / "support", evidence / "original"
    support.mkdir()
    original.mkdir()
    compile_java(support, [ROOT / "FieldParent.java"])
    compile_java(original, [ROOT / f"{name}.java" for name in SUBJECTS], support)
    compile_java(support, [ROOT / "CheckFieldOwners.java"], original, support)
    return original, support


def recovered_classes(binary, original, support, evidence):
    recovered = evidence / "recovered"
    recovered.mkdir()
    sources = []
    for name in SUBJECTS:
        output = invoke([str(binary), "get-class", str(original / f"{name}.class"), "j0", "--format", "json"])
        (recovered / f"{name}.json").write_text(output, encoding="utf-8")
        publication = json.loads(output)
        if publication["selection"]["class"] != f"L{name};" or publication["issues"]:
            raise ValueError(f"unexpected class publication for {name}")
        source = recovered / f"{name}.java"
        source.write_text(publication["result"]["text"], encoding="utf-8")
        sources.append(source)
    compile_java(recovered, sources, support)
    return recovered


def digest(path):
    hasher = sha256()
    with path.open("rb") as source:
        for part in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(part)
    return hasher.hexdigest()


def evidence_for(classes, support):
    return {"runtime": runtime(classes, support), "bindings": check_bindings(classes)}


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
        original, support = original_classes(evidence)
        checks = evidence_for(original, support)
        report["original"] = checks
        report["original_sha256"] = {name: digest(original / f"{name}.class") for name in SUBJECTS}
        if any(not row["pass_"] for kind in checks.values() for row in kind.values()):
            raise ValueError(f"original disagrees with the authored witness {checks}")
        if options.binary:
            binary = options.binary.resolve(strict=True)
            report.update(binary=str(binary), binary_sha256=digest(binary),
                          binary_version=invoke([str(binary), "--version"]).strip())
            recovered = recovered_classes(binary, original, support, evidence)
            checks = evidence_for(recovered, support)
            report["recovered"] = checks
        report["status"] = "PASS"
        for kind, observations in checks.items():
            passed = sum(row["pass_"] for row in observations.values())
            print(f"{passed}/{len(observations)} {kind} checks passed")
            for name, row in observations.items():
                if not row["pass_"]:
                    report["status"] = "FAIL"
                    print(f"{name} expected {row['expected']}, observed {row['observed']}")
    except (KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
        print(f"Proof setup failed. {error}", file=sys.stderr)
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"Receipt at {output}")
    return {"PASS": 0, "FAIL": 1, "SETUP_ERROR": 2}[report["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
