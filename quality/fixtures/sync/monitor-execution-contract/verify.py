"""Run the exact recovered class with an independent JVM observer and disassembly."""

import argparse
import json
from pathlib import Path
import re
import runpy
from subprocess import SubprocessError
from tempfile import TemporaryDirectory

from call_bindings import JAVAP_INSTRUCTION, JAVAP_METHOD
from corpus_models import diagnostics, invoke


ROOT = Path(__file__).resolve().parent
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData", "-Xmx128m"]
AT = "e0:l0:c0"
OWNER = "Lfixture/MonitorContract;"
EXPECTED_ERRORS = (OSError, ValueError, KeyError, SubprocessError)
FLAGS = re.compile(r"    flags: \((0x[\da-fA-F]+)\)(?: .*)?")


def call(args, directory, timeout):
    result = invoke(args, directory, timeout)
    if result.returncode:
        raise ValueError(diagnostics(result.stderr, directory)
                         or f"{Path(args[0]).name} exited with status {result.returncode}")
    return result.stdout.decode("utf-8", errors="strict")


def source(binary, artifact, directory, timeout):
    if binary is None:
        return (ROOT / "MonitorContract.java").read_text(encoding="utf-8")
    payload = call([binary, "get-class", artifact, AT, "--format", "json"], directory, timeout)
    publication = json.loads(payload)
    if (not isinstance(publication, dict) or publication.get("schema_version") != 5
            or publication.get("command") != "get-class"
            or not isinstance(publication.get("selection"), dict)
            or publication["selection"].get("class_at") != f"{AT}::{OWNER}"
            or publication.get("issues") != []):
        raise ValueError("complete original MonitorContract publication could not be established")
    result = publication.get("result")
    text = result.get("text") if isinstance(result, dict) else None
    if not isinstance(text, str) or not text.strip():
        raise ValueError("the original MonitorContract class text is unavailable")
    return text


def classfile(compiled, directory, timeout):
    outcome = call([*JAVA, "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-c", "-s", "-v",
                    compiled / "fixture/MonitorContract.class"], directory, timeout)
    if "public final class fixture.MonitorContract" not in outcome.splitlines():
        raise ValueError("Javap class identity is absent or unexpected")
    methods = {}
    declaration = None
    previous = ""
    in_code = False
    for line in outcome.splitlines():
        signature = JAVAP_METHOD.fullmatch(line)
        candidate_flags = FLAGS.fullmatch(line)
        if signature is not None and "(" in previous:
            name = previous.partition("(")[0].rsplit(" ", 1)[-1]
            descriptor = signature[1]
            member = f"{OWNER}->{name}{descriptor}"
            if member in methods:
                raise ValueError("duplicate Javap method descriptor")
            declaration = {"monitorenter_pcs": [], "monitorexit_pcs": []}
            methods[member] = declaration
            in_code = False
        elif declaration is not None and candidate_flags is not None and declaration.get("flags") is None:
            declaration["flags"] = int(candidate_flags[1], 16)
        elif declaration is not None and line == "    Code:":
            in_code = True
        elif in_code:
            insn = JAVAP_INSTRUCTION.fullmatch(line)
            if insn and insn[2] in ("monitorenter", "monitorexit"):
                declaration[f"{insn[2]}_pcs"].append(int(insn[1]))
            if not line.startswith("      "):
                in_code = False
        previous = line
    return methods


def execute(binary, artifact, directory, scenarios, timeout):
    directory.mkdir()
    publication = source(binary, artifact, directory, timeout)
    edited = directory / "MonitorContract.java"
    edited.write_text(publication, encoding="utf-8")
    compiled = directory / "classes"
    call([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none", "-encoding",
          "UTF-8", "-d", compiled, edited, ROOT / "MonitorHooks.java", ROOT / "CheckMonitor.java"],
         directory, timeout)
    evidence = classfile(compiled, directory, timeout)
    observations = {}
    for scenario in scenarios:
        try:
            text = call([*JAVA, "-cp", compiled, "fixture.CheckMonitor", scenario], directory, timeout)
            lines = text.splitlines()
            if len(lines) != 1:
                raise ValueError("observer did not publish exactly one event trace")
            observations[scenario] = {"observed": lines[0].split(",")}
        except EXPECTED_ERRORS as error:
            observations[scenario] = {"observed": None, "detail": str(error)}
    return {"classfile": evidence, "observations": observations, "publication": publication}


def shape(classfiles, member):
    record = classfiles.get(member)
    if record is None or "flags" not in record:
        return None
    return {"acc_synchronized": bool(record["flags"] & 0x20),
            "acc_native": bool(record["flags"] & 0x100),
            "monitorenter": len(record["monitorenter_pcs"]),
            "monitorexit": len(record["monitorexit_pcs"])}


def check(identifier, wanted, control, observed, cand_error=None):
    row = {"id": identifier, "expected": wanted, "observed": observed, "source_control": control}
    if wanted != control:
        row.update(status="SETUP_ERROR", detail="authored Java control disagrees with the original DEX")
    elif observed is None:
        row.update(status="UNCHECKED", detail=cand_error or "compatible observation unavailable")
    else:
        row["status"] = "PASS" if wanted == observed else "FAIL"
    return row


def evaluate(binary, artifact, timeout):
    binary = Path(binary).resolve() if binary is not None else None
    artifact = Path(artifact).resolve()
    raw = runpy.run_path(str(ROOT / "raw_proof.py"))["verify"](artifact)
    scenarios = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    candidate_error = None
    with TemporaryDirectory(prefix="monitor-contract-dex-") as temporary:
        directory = Path(temporary)
        original = execute(None, artifact, directory / "original", scenarios, timeout)
        recovered = original
        if binary is not None:
            try:
                recovered = execute(binary, artifact, directory / "recovered", scenarios, timeout)
            except EXPECTED_ERRORS as error:
                candidate_error = str(error)
                recovered = {"classfile": {}, "observations": {}}
    results = {}
    for evidence in raw.values():
        member = evidence["member"]
        original_shape = shape(original["classfile"], member)
        observed_shape = shape(recovered["classfile"], member)
        original_lock = {key: value for key, value in evidence.items() if key != "checks"}
        results[evidence["at"]] = [{
            **check("lock-classfile", evidence["expected_classfile"], original_shape,
                    observed_shape, candidate_error),
            "original_dex": original_lock,
            "recovered_classfile": recovered["classfile"].get(member),
        }]
        for scenario, witness in evidence["checks"].items():
            control = original["observations"].get(scenario, {}).get("observed")
            receipt = recovered["observations"].get(scenario, {})
            results[evidence["at"]].append({
                **check(scenario, witness["expected"], control,
                        receipt.get("observed"), receipt.get("detail", candidate_error)),
                "original_dex": witness,
            })
    return {"checks": results, "recovered_publication": recovered.get("publication"),
            "reference_classfile": original["classfile"], "recovered_classfile": recovered["classfile"]}


def probe(binary, artifact, timeout):
    return evaluate(binary, artifact, timeout)["checks"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    receipt = evaluate(args.binary, ROOT / "classes.dex", 15)
    args.output.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    checks = [row for rows in receipt["checks"].values() for row in rows]
    for row in checks:
        print(row["id"], row["status"], row["observed"])
    outcomes = {row["status"] for row in checks}
    return 2 if outcomes & {"SETUP_ERROR", "UNCHECKED"} else int("FAIL" in outcomes)


if __name__ == "__main__":
    raise SystemExit(main())
