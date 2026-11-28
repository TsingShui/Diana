"""Compare original JVM null overloads with selected and full Diana publications."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
from tempfile import TemporaryDirectory, mkdtemp


FIXTURE = Path(__file__).resolve().parent
OWNER = "NullableOverload"
CLASS = f"{OWNER}.class"
SOURCE = f"{OWNER}.java"
JAVA = ["java", "-XX:-UsePerfData"]
JAVAC = [
    *JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main",
    "-source", "17", "-target", "17", "-Xlint:-options", "-proc:none", "-g",
]
JAVAP = [*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-c", "-s"]
CASES = {
    "j0:m1": ("object", "LNullableOverload;", "pick", "Ljava/lang/Object;", {"value": "object/1"}),
    "j0:m2": ("integer", "LNullableOverload;", "pick", "Ljava/lang/Integer;", {"value": "integer/2"}),
    "j0:m3": ("guarded", "LNullableOverload;Z", "choose", "ILjava/lang/Object;",
              {"false": "guarded-object/56", "true": "leading-fault/58"}),
    "j0:m4": ("bounded", "", "NullEffects.bounded", "Ljava/lang/Object;", {"value": "bounded/9"}),
}
ENV = {key: value for key, value in os.environ.items() if key not in (
    "CLASSPATH", "JDK_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS",
)}


def run(args, timeout):
    process = subprocess.run(args, capture_output=True, text=True, timeout=timeout, env=ENV)
    if process.returncode:
        raise ValueError(f"{args[0]} exited with {process.returncode}\n{process.stdout}{process.stderr}")
    return process.stdout


def classpath(*directories):
    return ["-cp", os.pathsep.join(map(str, directories))]


def compile_java(directory, source, timeout, *dependencies):
    directory.mkdir(parents=True, exist_ok=True)
    classpath_args = classpath(*dependencies) if dependencies else []
    run([*JAVAC, *classpath_args, "-d", str(directory), str(source)], timeout)


def compile_fixture(original, support, timeout):
    compile_java(support, FIXTURE / "NullEffects.java", timeout)
    compile_java(original, FIXTURE / SOURCE, timeout, support)
    return original / CLASS


def prepare(directory, timeout):
    with TemporaryDirectory(prefix="diana-null-overload-prepare-") as scratch:
        return compile_fixture(directory, Path(scratch), timeout)


def disassemble(class_file, output, timeout):
    text = run([*JAVAP, str(class_file)], timeout)
    output.write_text(text, encoding="utf-8")
    return {name: (descriptor, body) for name, descriptor, body in re.findall(
        r"^  public static java\.lang\.String (\w+)\([^\n]*\);\n"
        r"    descriptor: (\S+)\n(.*?)(?=^  \S|\Z)", text, re.MULTILINE | re.DOTALL,
    )}


def method_prototype(parameters):
    return f"({parameters})Ljava/lang/String;"


def guarded_handler_contains_calls(body):
    handler = re.search(r"Exception table:\s*from\s+to\s+target\s+type\s*"
                        r"(\d+)\s+(\d+)\s+(\d+)\s+Class java/lang/IllegalStateException", body)
    positions = [int(pc) for pc in re.findall(r"^\s*(\d+): invoke(?:static|virtual)\b", body, re.MULTILINE)]
    if handler is None or len(positions) != 3:
        return False
    start, end, target = map(int, handler.groups())
    return start <= positions[0] < positions[1] < end <= target <= positions[2]


def check_original(case, methods):
    name, parameters, consumer, formal, _ = case
    descriptor, body = methods[name]
    expected = f"{consumer}:{method_prototype(formal)}"
    calls = re.findall(r"invoke(?:static|virtual)\s+#\d+\s+// Method (\S+)", body)
    if descriptor != method_prototype(parameters) or "aconst_null" not in body:
        raise ValueError(f"{name}: original null path or prototype changed")
    if name != "guarded":
        if calls != [expected]:
            raise ValueError(f"{name}: unexpected original consumer {calls}")
        return
    if calls != ["NullEffects.leading:(Z)I", expected,
                 "NullEffects.recordLeading:(Ljava/lang/IllegalStateException;)Ljava/lang/String;"]:
        raise ValueError(f"{name}: unexpected original ordered calls {calls}")
    if not guarded_handler_contains_calls(body):
        raise ValueError("guarded method changed which calls reach the original handler")


def observe(classes, support, name, mode, timeout):
    args = [name] if mode == "value" else [name, mode]
    return run([*JAVA, *classpath(classes, support), "CheckNull", *args], timeout).strip()


def recover(binary, artifact, endpoint, selected, classes, support, timeout):
    locator = selected if selected else "j0"
    classes.mkdir()
    output = run([str(binary), endpoint, str(artifact), locator, "--format", "json"], timeout)
    (classes / "publication.json").write_text(output, encoding="utf-8")
    data = json.loads(output)
    rows = data["result"]["methods"]
    selected_cases = {selected: CASES[selected]} if selected else CASES
    expected = {(at, case[0], method_prototype(case[1])) for at, case in selected_cases.items()}
    reported = {(row["at"], row["name"], row["prototype"]) for row in rows}
    if (
        data["selection"]["class"] != f"L{OWNER};" or data["selection"]["class_at"] != "j0"
        or data["issues"] or not expected.issubset(reported)
        or (selected and (reported != expected or data["selection"]["method_at"] != selected))
    ):
        raise ValueError(f"{locator}: incomplete or incorrectly selected publication")
    source = classes / SOURCE
    text = data["result"]["text"]
    if selected:
        authored = (FIXTURE / SOURCE).read_text(encoding="utf-8")
        definitions = authored.split("    // BEGIN OVERLOAD DEFINITIONS\n", 1)[1].split(
            "    // END OVERLOAD DEFINITIONS", 1,
        )[0]
        body, closing = text.rsplit("}", 1)
        text = body + definitions + "}" + closing
    source.write_text(text, encoding="utf-8")
    compile_java(classes, source, timeout, support)
    return disassemble(classes / CLASS, classes / "recompiled.javap", timeout)


def row(identifier, expected, observed, detail=None):
    status = "UNCHECKED" if detail is not None else "PASS" if observed == expected else "FAIL"
    result = {"id": identifier, "expected": expected, "observed": observed, "status": status}
    if detail is not None:
        result["detail"] = str(detail)
    return result


def compare_endpoint(case, endpoint, classes, support, methods, error, timeout):
    name, parameters, consumer, formal, scenarios = case
    expected = f"{consumer}:{method_prototype(formal)}"
    descriptor_check = f"{endpoint}-binding"
    if error is not None:
        checks = [row(descriptor_check, expected, None, error)]
        if name == "guarded":
            checks.append(row(f"{endpoint}-handler-range", True, None, error))
        return checks + [
            row(f"{endpoint}-{mode}", value, None, error) for mode, value in scenarios.items()
        ]
    descriptor, body = methods.get(name, ("", ""))
    matching = re.findall(rf"invoke(?:static|virtual)\s+#\d+\s+// Method ({re.escape(consumer)}:\S+)", body)
    if descriptor != method_prototype(parameters) or len(matching) != 1:
        checks = [row(descriptor_check, expected, None, "no unique recompiled consumer")]
    else:
        checks = [row(descriptor_check, expected, matching[0])]
    if name == "guarded":
        checks.append(row(f"{endpoint}-handler-range", True, guarded_handler_contains_calls(body)))
    for mode, expected in scenarios.items():
        identifier = f"{endpoint}-{mode}"
        try:
            checks.append(row(identifier, expected, observe(classes, support, name, mode, timeout)))
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            checks.append(row(identifier, expected, None, error))
    return checks


def verify(binary, artifact, evidence, timeout):
    original = evidence / "original"
    support = evidence / "support"
    compiled = compile_fixture(original, support, timeout)
    if artifact is None:
        artifact = compiled
    else:
        authored = compiled.read_bytes()
        if artifact.stat().st_size != len(authored) or artifact.read_bytes() != authored:
            raise ValueError("prepared caller differs from the authored fixture")
    compile_java(support, FIXTURE / "CheckNull.java", timeout, original, support)
    original_methods = disassemble(compiled, evidence / "original.javap", timeout)
    checks = {}
    for at, case in CASES.items():
        check_original(case, original_methods)
        name, _, _, _, scenarios = case
        rows = []
        for mode, expected in scenarios.items():
            actual = observe(original, support, name, mode, timeout)
            if actual != expected:
                raise ValueError(f"{at}/{mode}: original {actual}, expected {expected}")
            if binary is None:
                rows.append(row(mode, expected, actual))
        checks[at] = rows
    if binary is None:
        return checks

    whole = evidence / "class"
    full_methods = full_error = None
    try:
        full_methods = recover(binary, artifact, "get-class", None, whole, support, timeout)
    except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
        full_error = error
    for at, case in CASES.items():
        checks[at].extend(compare_endpoint(case, "class", whole, support, full_methods, full_error, timeout))
        selected = evidence / at.replace(":", "-")
        selected_methods = selected_error = None
        try:
            selected_methods = recover(binary, artifact, "get-method", at, selected, support, timeout)
        except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
            selected_error = error
        checks[at].extend(compare_endpoint(
            case, "method", selected, support, selected_methods, selected_error, timeout,
        ))
    return checks


def probe(binary, artifact, timeout):
    with TemporaryDirectory(prefix="diana-null-overload-") as scratch:
        return verify(binary, artifact, Path(scratch), timeout)


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
            receipt["version"] = run([str(binary), "--version"], 20).strip()
        checks = verify(binary, None, evidence, 20)
        receipt["checks"] = checks
        outcomes = [check["status"] for rows in checks.values() for check in rows]
        receipt["status"] = next(status for status in ("FAIL", "UNCHECKED", "PASS") if status in outcomes)
        for at, rows in checks.items():
            for check in rows:
                if check["status"] != "PASS":
                    print(at, check["id"], check["expected"], check["observed"], check.get("detail", ""))
        print(f"{outcomes.count('PASS')}/{len(outcomes)} checks passed")
    except (KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
        receipt["detail"] = str(error)
        print(f"Proof setup failed. {error}")
    args.output.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    return {"PASS": 0, "FAIL": 1, "UNCHECKED": 2, "SETUP_ERROR": 2}[receipt["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
