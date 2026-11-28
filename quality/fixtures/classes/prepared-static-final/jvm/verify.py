"""Check exact JVM class publications for active reads of prepared static finals."""

from argparse import ArgumentParser
from hashlib import sha256
import json
from os import pathsep
from pathlib import Path
import re
import subprocess
import sys
from tempfile import mkdtemp

from prepare import class_bytes


ROOT = Path(__file__).resolve().parent
JAVA = ["java", "-XX:-UsePerfData"]
JAVAC = [*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none"]
JAVAP = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-v"]
NAMES = ("DefaultChild", "ReadDefaultChild")
CONSTANTS = {
    "PREPARED_ZERO": ("int", None),
    "PREPARED_NULL": ("java.lang.String", None),
    "PREPARED_BYTE": ("byte", None),
    "PREPARED_SHORT": ("short", None),
    "COMPILED_ZERO": ("int", "int 0"),
    "COMPILED_TEXT": ("java.lang.String", "String interned"),
}
ACTIVE_READS = {
    "preparedZero": ("int", ["DefaultChild.PREPARED_ZERO:I"]),
    "preparedNull": ("java.lang.String", ["DefaultChild.PREPARED_NULL:Ljava/lang/String;"]),
    "compiledZero": ("int", []),
    "compiledText": ("java.lang.String", []),
    "preparedByte": ("byte", ["DefaultChild.PREPARED_BYTE:B"]),
    "preparedShort": ("short", ["DefaultChild.PREPARED_SHORT:S"]),
}


def invoke(argv):
    result = subprocess.run(argv, capture_output=True, text=True, timeout=20)
    if result.returncode:
        raise ValueError(f"{argv[0]} exited with {result.returncode}\n{result.stdout}{result.stderr}")
    return result.stdout


def compile_java(destination, sources, *classpath):
    args = [*JAVAC, "-d", str(destination)]
    if classpath:
        args.extend(["-cp", pathsep.join(map(str, classpath))])
    invoke([*args, *map(str, sources)])


def runtime(classes, support, expected):
    classpath = pathsep.join(map(str, [classes, support]))
    observations = {}
    for name, events in expected.items():
        output = invoke([*JAVA, "-cp", classpath, "CheckDefaultChild", name])
        trace = output.rstrip("\n").split(",")
        observations[name] = {"expected": events, "observed": trace,
                              "status": "PASS" if trace == events else "FAIL"}
    return observations


def member(dump, declaration):
    lines = dump.splitlines()
    start = next((index for index, line in enumerate(lines) if re.fullmatch(declaration, line)), None)
    if start is None:
        raise ValueError(f"javap did not identify {declaration!r}")
    end = next(index for index in range(start + 1, len(lines)) if re.match(r"^(?:  \S|})", lines[index]))
    return "\n".join(lines[start:end])


def raw_facts(classes):
    def listing(name):
        text = invoke([*JAVAP, str(classes / f"{name}.class")])
        (classes / f"{name}.javap").write_text(text, encoding="utf-8")
        return text

    child = listing("DefaultChild")
    reader = listing("ReadDefaultChild")
    constants = {}
    for field, (java_type, _) in CONSTANTS.items():
        declaration = member(child, rf"  public static final {re.escape(java_type)} {field};")
        constant = re.search(r"(?m)^\s+ConstantValue: (.+)$", declaration)
        constants[field] = constant[1] if constant else None
    active = {}
    for name, (java_type, _) in ACTIVE_READS.items():
        method = member(reader, rf"  public static {re.escape(java_type)} {name}\(\);")
        active[name] = re.findall(r"\bgetstatic\s+#\d+\s+// Field (\S+)", method)
    return dict(constant_value=constants, active_reads=active,
                child_has_clinit=bool(re.search(r"(?m)^  static \{\};$", child)),
                child_has_putstatic=bool(re.search(r"(?m)^\s+\d+: putstatic\b", child)))


def digest(path):
    fingerprint = sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            fingerprint.update(chunk)
    return fingerprint.hexdigest()


def original_classes(evidence):
    support = evidence / "support"
    original = evidence / "original"
    support.mkdir()
    original.mkdir()
    compile_java(support, [ROOT / "ParentEffects.java", ROOT / "ObservableParent.java"])
    (original / "DefaultChild.class").write_bytes(class_bytes())
    compile_java(original, [ROOT / "ReadDefaultChild.java"], original, support)
    compile_java(support, [ROOT / "CheckDefaultChild.java"], original, support)
    return original, support


def recover(binary, original, support, evidence):
    recovered = evidence / "recovered"
    recovered.mkdir()
    sources = []
    for name in NAMES:
        publication = invoke([str(binary), "get-class", str(original / f"{name}.class"),
                              "j0", "--format", "json"])
        (recovered / f"{name}.json").write_text(publication, encoding="utf-8")
        payload = json.loads(publication)
        if payload["selection"]["class"] != f"L{name};" or payload["issues"]:
            raise ValueError(f"unexpected class publication for {name}")
        source = recovered / f"{name}.java"
        source.write_text(payload["result"]["text"], encoding="utf-8")
        sources.append(source)
    compile_java(recovered, sources, support)
    return recovered


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    opts = parser.parse_args()
    output = opts.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(mkdtemp(prefix=f"{output.stem}-", dir=output.parent))
    result = dict(evidence=str(evidence), status="SETUP_ERROR")
    try:
        expected = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
        original, support = original_classes(evidence)
        facts = raw_facts(original)
        observations = runtime(original, support, expected)
        result.update(original_facts=facts, original_runtime=observations,
                      original_sha256={name: digest(original / f"{name}.class") for name in NAMES})
        expected_constants = {name: value for name, (_, value) in CONSTANTS.items()}
        expected_reads = {name: reads for name, (_, reads) in ACTIVE_READS.items()}
        if facts != dict(constant_value=expected_constants, active_reads=expected_reads,
                         child_has_clinit=False, child_has_putstatic=False):
            raise ValueError(f"original JVM instructions violate the witness contract {facts}")
        if any(row["status"] != "PASS" for row in observations.values()):
            raise ValueError(f"original JVM behavior violates independent expectations {observations}")

        if opts.binary:
            binary = opts.binary.resolve(strict=True)
            result.update(binary=str(binary), binary_sha256=digest(binary),
                          binary_version=invoke([str(binary), "--version"]).strip())
            recovered = recover(binary, original, support, evidence)
            facts = raw_facts(recovered)
            observations = runtime(recovered, support, expected)
            result.update(recovered_facts=facts, recovered_runtime=observations)
        passed = sum(row["status"] == "PASS" for row in observations.values())
        shape_preserved = (facts["constant_value"] == expected_constants
                           and facts["active_reads"] == expected_reads)
        result["status"] = "PASS" if shape_preserved and passed == len(expected) else "FAIL"
        print(f"{passed}/{len(expected)} scenarios passed. Receipt at {output}")
        if not shape_preserved:
            print(f"Recompiled field preparation or active read changed {facts}")
        for name, row in observations.items():
            if row["status"] == "FAIL":
                print(f"{name} expected {row['expected']}, observed {row['observed']}")
    except (KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        result["error"] = str(error)
        print(f"Proof setup failed. {error}. Receipt at {output}", file=sys.stderr)
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return {"PASS": 0, "FAIL": 1, "SETUP_ERROR": 2}[result["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
