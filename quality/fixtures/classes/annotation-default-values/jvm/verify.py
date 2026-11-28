"""Check JVM annotation defaults, raw xrefs, and local malformed-attribute recovery."""

from argparse import ArgumentParser
from hashlib import sha256
import json
from os import pathsep
from pathlib import Path
import subprocess
import sys
from tempfile import mkdtemp

from classfile import class_literal_offset, default_attributes, malformed_defaults


ROOT = Path(__file__).resolve().parent
JAVA = ["java", "-XX:-UsePerfData"]
JAVAC = [*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none"]
JAVAP = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-v"]
SUBJECT = "Defaults"
EXPECTED = {
    "number": "37", "enabled": "true", "byteValue": "-7", "shortValue": "300",
    "letter": "10", "wide": "9876543210", "floatValue": "80000000", "doubleValue": "1.25",
    "title": 'line\\n"quoted"', "token": "[[I", "choice": "LAST", "order": "[2, 5, 2]",
    "empty": "[]", "types": "[java.lang.String, void, [J, java.time.Instant]", "nested": "inside:3",
    "children": "[first:3, second:9]", "required": "absent", "annotation-use": "explicit:37:[first:3, second:9]",
}
TARGET_TYPE = "Ljava/time/Instant;"


def invoke(args, exits=(0,)):
    process = subprocess.run(args, capture_output=True, text=True, timeout=20)
    if process.returncode not in exits:
        raise ValueError(f"{args[0]} exited with {process.returncode}\n{process.stdout}{process.stderr}")
    return process


def compile_java(target, sources, *classpath):
    command = [*JAVAC, "-d", str(target)]
    if classpath:
        command.extend(["-cp", pathsep.join(map(str, classpath))])
    invoke([*command, *map(str, sources)])


def observe(classes, support):
    observed = invoke([*JAVA, "-cp", pathsep.join(map(str, (classes, support))), "CheckDefaults"]).stdout
    (classes / "trace.tsv").write_text(observed, encoding="utf-8")
    rows = {}
    for line in observed.splitlines():
        name, value = line.split("\t")
        if name in rows:
            raise ValueError(f"duplicate observation for {name}")
        rows[name] = value
    if rows.keys() != EXPECTED.keys():
        raise ValueError(f"unexpected observed keys {list(rows)}")
    return rows


def original_classes(evidence):
    support, original = evidence / "support", evidence / "original"
    support.mkdir()
    original.mkdir()
    compile_java(support, [ROOT / "Choice.java", ROOT / "Label.java"])
    compile_java(original, [ROOT / f"{SUBJECT}.java"], support)
    compile_java(support, [ROOT / "CheckDefaults.java"], original, support)
    listing = invoke([*JAVAP, str(original / f"{SUBJECT}.class")]).stdout
    (original / f"{SUBJECT}.javap").write_text(listing, encoding="utf-8")
    return original, support


def evaluate_publication(binary, class_file, support, directory, expected):
    directory.mkdir()
    raw = invoke([str(binary), "get-class", str(class_file), "j0", "--format", "json"], exits=(0, 1))
    (directory / "publication.json").write_text(raw.stdout, encoding="utf-8")
    publication = json.loads(raw.stdout)
    if publication["selection"]["class"] != f"L{SUBJECT};":
        raise ValueError("unexpected JVM class publication")
    source = directory / f"{SUBJECT}.java"
    source.write_text(publication["result"]["text"], encoding="utf-8")
    compile_java(directory, [source], support)
    observed = observe(directory, support)
    mismatches = {name: dict(expected=value, observed=observed[name])
                  for name, value in expected.items() if observed[name] != value}
    return dict(exit=raw.returncode, issues=publication["issues"], runtime=observed, mismatches=mismatches)


def read_xref(binary, class_file, target, output):
    raw = invoke([str(binary), "xref", str(class_file), "type", target, "--exact", "--format", "json"]).stdout
    output.write_text(raw, encoding="utf-8")
    return json.loads(raw)


def verify_xref(binary, class_file, defaults, literal_at, directory):
    result = read_xref(binary, class_file, TARGET_TYPE, directory / "xref.json")
    ordinal = defaults["types"].ordinal
    use = f"annotation-default-class-literal at class-file byte {literal_at} depth 1"
    return (result["complete"] and not result["issues"]
            and [(row["context"], row["match"], row["use"]) for row in result["items"]]
            == [(f"j0:m{ordinal}", TARGET_TYPE, use)])


def verify_malformed_xref(binary, defaults, directory):
    result = read_xref(binary, directory / "nested-name.class", "LLabel;", directory / "nested-xref.json")
    entry = defaults["nested"]
    context = f"j0:m{entry.ordinal}"
    use = f"annotation-default-type at class-file byte {entry.value_at + 1} depth 1"
    return (not result["complete"]
            and any(issue["context"] == context and issue["kind"] == "Malformed" for issue in result["issues"])
            and any(row["context"] == context and row["match"] == "LLabel;" and row["use"] == use
                    for row in result["items"]))


def check_malformed(binary, defaults, variants, support, evidence):
    outcomes = {}
    for name, (element, reason, mutated) in variants.items():
        input_file = evidence / f"{name}.class"
        input_file.write_bytes(mutated)
        expected = {**EXPECTED, element: "absent"}
        if element == "number":
            expected["annotation-use"] = "missing:number"
        observed = evaluate_publication(binary, input_file, support, evidence / name, expected)
        context = f"j0:m{defaults[element].ordinal}"
        localized = (observed["exit"] == 1
                     and all(issue["context"] == context for issue in observed["issues"])
                     and any(issue["kind"] == "Malformed" and reason in issue["reason"]
                             for issue in observed["issues"]))
        outcomes[name] = {**observed, "expected_context": context, "localized_issue": localized,
                          "passed": localized and not observed["mismatches"]}
    return outcomes


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
        original, support = original_classes(evidence)
        observed = observe(original, support)
        class_file = original / f"{SUBJECT}.class"
        data = class_file.read_bytes()
        defaults, pool = default_attributes(data)
        if observed != EXPECTED or set(defaults) != EXPECTED.keys() - {"required", "annotation-use"}:
            raise ValueError("the original disagrees with the authored annotation defaults")
        literal_at = class_literal_offset(data, defaults["types"], pool, TARGET_TYPE.encode("ascii"))
        members = {name: dict(at=f"j0:m{entry.ordinal}", descriptor=entry.descriptor.decode("ascii"),
                              value_at=entry.value_at) for name, entry in defaults.items()}
        report.update(original=observed, original_sha256=digest(class_file), default_members=members,
                      xref_literal_at=literal_at)
        passed = True
        if options.binary:
            binary = options.binary.resolve(strict=True)
            report.update(binary_version=invoke([str(binary), "--version"]).stdout.strip(), binary_sha256=digest(binary))
            ordinary = evaluate_publication(binary, class_file, support, evidence / "ordinary", EXPECTED)
            xref = verify_xref(binary, class_file, defaults, literal_at, evidence)
            malformed = check_malformed(binary, defaults, malformed_defaults(data, defaults), support, evidence)
            malformed_xref = verify_malformed_xref(binary, defaults, evidence)
            report.update(recovered=ordinary, xref=xref, malformed=malformed, malformed_xref=malformed_xref)
            passed = (ordinary["exit"] == 0 and not ordinary["issues"] and not ordinary["mismatches"]
                      and xref and malformed_xref and all(case["passed"] for case in malformed.values()))
            print(f"{len(EXPECTED) - len(ordinary['mismatches'])}/{len(EXPECTED)} ordinary observations passed")
            print(f"{sum(case['passed'] for case in malformed.values())}/{len(malformed)} malformed cases isolated")
            if not xref:
                print("Original annotation-default xref offset changed")
            if not malformed_xref:
                print("Checked prefix disappeared from the malformed default xref")
        else:
            print(f"{len(EXPECTED)}/{len(EXPECTED)} original observations passed")
        report["status"] = "PASS" if passed else "FAIL"
    except (KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
        print(f"Proof setup failed. {error}", file=sys.stderr)
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"Receipt at {output}")
    return {"PASS": 0, "FAIL": 1, "SETUP_ERROR": 2}[report["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
