"""Compare exact JVM publications against inherited preparation and initialization."""

from argparse import ArgumentParser
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import runpy
from subprocess import SubprocessError
import sys
from tempfile import mkdtemp


ROOT = Path(__file__).resolve().parent
SRC = ROOT / "src"
CAPTURE = runpy.run_path(str(next(
    path for parent in ROOT.parents
    if (path := parent / "quality/decbench/tool_capture.py").is_file()
)))["run"]
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData", "-Xmx128m"]
SUBJECTS = ("DefaultHierarchyOwner", "HierarchyAlias", "ReadStaticHierarchy")
HOSTS = ("ObservableRoot", "HierarchyEffects", "CheckHierarchy")
FIELDS = {
    "PREPARED_LONG": ("long", "J", None),
    "PREPARED_NULL": ("java.lang.String", "Ljava/lang/String;", None),
    "COMPILED_LONG": ("long", "J", "long 0l"),
    "COMPILED_TEXT": ("java.lang.String", "Ljava/lang/String;", "String hierarchy"),
}
READS = {
    "inheritedLong": ("long", "()J", [(0, "getstatic", "HierarchyAlias.PREPARED_LONG:J"), (3, "lreturn", None)]),
    "directNull": ("java.lang.String", "()Ljava/lang/String;",
                   [(0, "getstatic", "DefaultHierarchyOwner.PREPARED_NULL:Ljava/lang/String;"), (3, "areturn", None)]),
    "activeAlias": ("int", "()I", [(0, "getstatic", "HierarchyAlias.ALIAS_VALUE:I"), (3, "ireturn", None)]),
    "compiledLong": ("long", "()J", [(0, "lconst_0", None), (1, "lreturn", None)]),
    "compiledText": ("java.lang.String", "()Ljava/lang/String;",
                     [(0, "ldc", "hierarchy"), (2, "areturn", None)]),
}
SCENARIO_LOCATORS = {
    "compile-only": "j0:m4", "inherited-first": "j0:m1", "declaring-first": "j0:m2",
    "alias-first": "j0:m3", "root-fault-through-alias": "j0:m1", "alias-fault": "j0:m3",
}
KNOWN_FAILURES = (OSError, ValueError, KeyError, SubprocessError)
INSTRUCTION = re.compile(r"^\s+(\d+):\s+(\w+)(?:\s+.*)?$")
MEMBER_COMMENT = re.compile(r"// (?:Field|Method|String) (.+)$")


def call(argv, directory, timeout):
    result = CAPTURE(argv, cwd=directory, timeout=timeout, output_limit=64 * 1024,
                     diagnostic_limit=64 * 1024)
    if result.returncode:
        raw = result.stderr or result.stdout
        detail = raw.decode("utf-8", "replace").replace(str(directory), "<fixture>")[:1500]
        raise ValueError(detail or f"{Path(argv[0]).name} exited with status {result.returncode}")
    return result.stdout.decode("utf-8", errors="strict")


def compile_java(directory, source_files, timeout):
    call([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none", "-encoding", "UTF-8",
          "-cp", directory, "-d", directory, *source_files], directory, timeout)


def prepare(directory, timeout):
    directory.mkdir(parents=True, exist_ok=True)
    class_bytes = runpy.run_path(str(ROOT / "prepare.py"))["class_bytes"]
    (directory / "DefaultHierarchyOwner.class").write_bytes(class_bytes())
    compile_java(directory, [SRC / f"{name}.java" for name in (SUBJECTS[1:] + HOSTS)], timeout)


def declaration(text, header):
    lines = text.splitlines()
    indexes = [offset for offset, line in enumerate(lines) if line == header]
    if len(indexes) != 1:
        raise ValueError(f"missing or ambiguous original declaration {header.strip()}")
    start, = indexes
    end = next((index for index in range(start + 1, len(lines))
                if re.match(r"^(?:  \S|})", lines[index])), len(lines))
    return lines[start:end]


def instructions(lines):
    evidence = []
    for line in lines:
        decoded = INSTRUCTION.fullmatch(line)
        if not decoded:
            continue
        member = MEMBER_COMMENT.search(line)
        evidence.append((int(decoded[1]), decoded[2], member[1] if member else None))
    return evidence


def raw_facts(directory, timeout, receipts=None):
    dumps = {
        name: call([*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-v",
                    directory / f"{name}.class"], directory, timeout)
        for name in SUBJECTS
    }
    if receipts is not None:
        for name, dump in dumps.items():
            (receipts / f"{name}.javap").write_text(dump, encoding="utf-8")
    owner = dumps["DefaultHierarchyOwner"]
    alias = dumps["HierarchyAlias"]
    reader = dumps["ReadStaticHierarchy"]
    if ("public class DefaultHierarchyOwner extends ObservableRoot" not in owner.splitlines()
            or "public class HierarchyAlias extends DefaultHierarchyOwner" not in alias.splitlines()
            or "public final class ReadStaticHierarchy" not in reader.splitlines()):
        raise ValueError("classfile hierarchy differs from the authored control")
    fields = {}
    for name, (spelling, descriptor, _) in FIELDS.items():
        lines = declaration(owner, f"  public static final {spelling} {name};")
        if f"    descriptor: {descriptor}" not in lines:
            raise ValueError(f"wrong descriptor for {name}")
        constants = [line.removeprefix("    ConstantValue: ") for line in lines
                     if line.startswith("    ConstantValue: ")]
        if len(constants) > 1:
            raise ValueError(f"multiple ConstantValue attributes for {name}")
        fields[name] = constants[0] if constants else None
    reads = {}
    for name, (spelling, descriptor, _) in READS.items():
        lines = declaration(reader, f"  public static {spelling} {name}();")
        if f"    descriptor: {descriptor}" not in lines:
            raise ValueError(f"wrong descriptor for {name}")
        reads[name] = instructions(lines)
    alias_field = declaration(alias, "  public static final int ALIAS_VALUE;")
    if any(line.startswith("    ConstantValue:") for line in alias_field):
        raise ValueError("alias runtime field became a constant")
    if re.search(r"(?m)^  \S.* PREPARED_(LONG|NULL);$", alias):
        raise ValueError("the alias declares its own prepared field")
    return {
        "fields": fields, "reads": reads,
        "owner_clinit": "  static {};" in owner.splitlines(),
        "owner_putstatic": bool(re.search(r"(?m)^\s+\d+: putstatic\b", owner)),
        "alias_initializer": instructions(declaration(alias, "  static {};")),
        "resolved_inherited_declaration": "LDefaultHierarchyOwner;->PREPARED_LONG:J",
    }


def check_original(facts):
    expected = {
        "fields": {field: spec[2] for field, spec in FIELDS.items()},
        "reads": {read: spec[2] for read, spec in READS.items()},
        "owner_clinit": False, "owner_putstatic": False,
        "alias_initializer": [(0, "invokestatic", "HierarchyEffects.aliasValue:()I"),
                              (3, "putstatic", "ALIAS_VALUE:I"), (6, "return", None)],
        "resolved_inherited_declaration": "LDefaultHierarchyOwner;->PREPARED_LONG:J",
    }
    if facts != expected:
        raise ValueError(f"raw original classfiles disagree with the authored bytecode obligations: {facts}")


def scenario_values(directory, expected, timeout):
    output = {}
    for scenario in expected:
        result = call([*JAVA, "-cp", directory, "CheckHierarchy", scenario], directory, timeout)
        lines = result.splitlines()
        if len(lines) != 1:
            raise ValueError("observer did not emit one trace")
        output[scenario] = lines[0].split(",")
    return output


def render(binary, originals, recovered, timeout):
    publications = []
    for name in SUBJECTS:
        data = call([binary, "get-class", originals / f"{name}.class", "j0", "--format", "json"],
                    recovered, timeout)
        payload = json.loads(data)
        selection = payload["selection"]
        if (payload.get("schema_version") != 5 or payload.get("command") != "get-class"
                or payload.get("issues") != [] or selection.get("input_kind") != "jvm"
                or selection.get("class_at") != "j0" or selection.get("class") != f"L{name};"):
            raise ValueError(f"complete exact publication unavailable for {name}")
        original = payload["result"]["text"]
        if not isinstance(original, str) or not original.strip():
            raise ValueError(f"missing published source for {name}")
        (recovered / f"{name}.json").write_text(data, encoding="utf-8")
        source = recovered / f"{name}.java"
        source.write_text(original, encoding="utf-8")
        publications.append(source)
    compile_java(recovered, publications + [SRC / f"{name}.java" for name in HOSTS], timeout)


def evaluate(binary, originals, recovered, timeout):
    expected = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    if set(expected) != set(SCENARIO_LOCATORS):
        raise ValueError("authored scenario contract is incomplete")
    facts = raw_facts(originals, timeout, recovered.parent)
    check_original(facts)
    reference = scenario_values(originals, expected, timeout)
    if reference != expected:
        raise ValueError(f"original JVM execution contradicts independent expectations: {reference}")
    record = {"original_facts": facts, "original_runtime": reference}
    if binary is None:
        observed = reference
        why = None
    else:
        recovered.mkdir(parents=True, exist_ok=False)
        try:
            render(binary, originals, recovered, timeout)
            observed = scenario_values(recovered, expected, timeout)
            why = None
            try:
                evidence = raw_facts(recovered, timeout, recovered)
                record["recovered_facts"] = evidence
                fields_preserved = evidence["fields"] == facts["fields"]
                getstatics = dict(evidence["reads"])
                canonical = (0, "getstatic", "DefaultHierarchyOwner.PREPARED_LONG:J")
                if getstatics["inheritedLong"] == [canonical, (3, "lreturn", None)]:
                    getstatics["inheritedLong"] = facts["reads"]["inheritedLong"]
                accesses_preserved = getstatics == facts["reads"]
                record["recovered_classfile_contract"] = {
                    "status": "PASS" if fields_preserved and accesses_preserved else "FAIL",
                    "constant_values_preserved": fields_preserved,
                    "active_accesses_preserved": accesses_preserved,
                }
            except KNOWN_FAILURES as error:
                record["recovered_classfile_contract"] = {"status": "UNCHECKED", "detail": str(error)}
        except KNOWN_FAILURES as exc:
            observed, why = {}, str(exc)
    rows = []
    for scenario, wanted in expected.items():
        seen = observed.get(scenario)
        row = {"id": scenario, "at": SCENARIO_LOCATORS[scenario], "expected": wanted,
               "observed": seen, "original_runtime": reference[scenario]}
        if seen is None:
            row.update(status="UNCHECKED", detail=why)
        else:
            row["status"] = "PASS" if seen == wanted else "FAIL"
        rows.append(row)
    record["checks"] = rows
    record["counts"] = dict(Counter(row["status"] for row in rows))
    return record


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--classes", type=Path, help="reuse already prepared original classes")
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()
    output = arguments.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    evidence_dir = Path(mkdtemp(prefix=f"{output.stem}-", dir=output.parent))
    report = {"evidence": str(evidence_dir), "status": "SETUP_ERROR"}
    try:
        original = arguments.classes.resolve() if arguments.classes else evidence_dir / "original"
        if arguments.classes is None:
            prepare(original, 15)
        binary = arguments.binary.resolve(strict=True) if arguments.binary else None
        report.update(evaluate(binary, original, evidence_dir / "recovered", 15))
        report["original_sha256"] = {
            name: hashlib.sha256((original / f"{name}.class").read_bytes()).hexdigest()
            for name in SUBJECTS + HOSTS
        }
        if binary:
            with binary.open("rb") as stream:
                report["binary"] = str(binary)
                report["binary_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
            report["binary_version"] = call([binary, "--version"], evidence_dir, 15).strip()
        outcomes = set(report["counts"])
        structural = report.get("recovered_classfile_contract")
        if structural is not None:
            outcomes.add(structural["status"])
        report["status"] = next((status for status in ("UNCHECKED", "FAIL", "PASS")
                                 if status in outcomes), "SETUP_ERROR")
        for row in report["checks"]:
            print(row["id"], row["status"], row["observed"])
    except KNOWN_FAILURES as error:
        report["error"] = str(error)
        print(error, file=sys.stderr)
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(report["status"], report.get("counts", {}), output)
    return {"PASS": 0, "FAIL": 1, "SETUP_ERROR": 2, "UNCHECKED": 2}[report["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
