"""Evaluate complete paired class publications against authored and raw controls."""

import argparse
from hashlib import file_digest
import json
from pathlib import Path
import runpy
from subprocess import SubprocessError
from tempfile import TemporaryDirectory

from corpus_models import diagnostics, invoke


FIXTURE = Path(__file__).resolve().parent
CLASSES = {"RuntimeFace": "e0:l0:c2", "FaceConsumer": "e0:l0:c0"}
SCENARIO_METHODS = {"first": "e0:l0:c0:m0", "second": "e0:l0:c0:m1"}
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData", "-Xmx128m"]
EXECUTION_ERRORS = (OSError, ValueError, KeyError, SubprocessError)


def call(arguments, directory, timeout):
    result = invoke(arguments, directory, timeout)
    if result.returncode:
        detail = diagnostics(result.stderr, directory)
        raise ValueError(detail or f"{Path(arguments[0]).name} exited with status {result.returncode}")
    return result.stdout.decode("utf-8", errors="strict")


def classes(binary, artifact, directory, timeout):
    outputs = {}
    for name, at in CLASSES.items():
        if binary is None:
            outputs[name] = (FIXTURE / f"{name}.java").read_text(encoding="utf-8")
            continue
        payload = call([
            str(binary), "get-class", str(artifact), at, "--format", "json",
        ], directory, timeout)
        publication = json.loads(payload)
        identity = f"{at}::Lfixture/{name};"
        if (not isinstance(publication, dict) or publication.get("schema_version") != 5
                or publication.get("command") != "get-class"
                or not isinstance(publication.get("selection"), dict)
                or publication["selection"].get("class_at") != identity
                or publication.get("issues") != []):
            raise ValueError(f"unavailable complete class publication for {identity}")
        result = publication.get("result")
        text = result.get("text") if isinstance(result, dict) else None
        if not isinstance(text, str) or not text.strip():
            raise ValueError(f"missing class text for {identity}")
        outputs[name] = text
    return outputs


def execute(publications, scenarios, scratch, timeout):
    sources = []
    for name, body in publications.items():
        source = scratch / f"{name}.java"
        source.write_text(body, encoding="utf-8")
        sources.append(source)
    hooks = [FIXTURE / f"{name}.java" for name in ("CheckInterface", "InterfaceEffects")]
    output = scratch / "compiled"
    call([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none",
          "-encoding", "UTF-8", "-d", str(output), *map(str, [*sources, *hooks])],
         scratch, timeout)
    observations = {}
    for scenario in scenarios:
        try:
            text = call([*JAVA, "-cp", str(output), "fixture.CheckInterface", scenario],
                        scratch, timeout)
            lines = text.splitlines()
            if len(lines) != 1:
                raise ValueError("observer did not publish exactly one event trace")
            observations[scenario] = {"observed": lines[0].split(",") if lines[0] else []}
        except EXECUTION_ERRORS as error:
            observations[scenario] = {"observed": None, "detail": str(error)}
    return observations


def evaluate(binary, artifact, timeout):
    artifact = Path(artifact).resolve()
    binary = Path(binary).resolve() if binary is not None else None
    expected = json.loads((FIXTURE / "scenarios.json").read_text(encoding="utf-8"))
    if set(expected) != set(SCENARIO_METHODS):
        raise ValueError("authored scenarios and method assignments differ")

    raw = None
    original = {}
    recovered = {}
    publications = {}
    setup_error = None
    candidate_error = None
    with TemporaryDirectory(prefix="interface-runtime-dex-") as temporary:
        scratch = Path(temporary)
        try:
            raw = runpy.run_path(str(FIXTURE / "raw_proof.py"))["verify"](artifact)
            raw_rows = {row["id"]: row for row in raw["checks"]}
            if set(raw_rows) != set(expected) or len(raw_rows) != len(raw["checks"]):
                raise ValueError("original DEX replay did not cover the authored scenarios")
            original_dir = scratch / "original"
            original_dir.mkdir()
            authored = classes(None, artifact, original_dir, timeout)
            original = execute(authored, expected, original_dir, timeout)
        except EXECUTION_ERRORS as error:
            setup_error = str(error)
            raw_rows = {}
        if setup_error is None:
            if binary is None:
                publications, recovered = authored, original
            elif any(raw_rows[scenario]["status"] == "PASS" and
                     original[scenario]["observed"] == wanted
                     for scenario, wanted in expected.items()):
                try:
                    candidate_dir = scratch / "recovery"
                    candidate_dir.mkdir()
                    publications = classes(binary, artifact, candidate_dir, timeout)
                    recovered = execute(publications, expected, candidate_dir, timeout)
                except EXECUTION_ERRORS as error:
                    candidate_error = str(error)
    rows = []
    for scenario, wanted in expected.items():
        control = original.get(scenario, {})
        raw_row = raw_rows.get(scenario)
        comparison = recovered.get(scenario, {})
        observed = comparison.get("observed")
        row = {
            "id": scenario,
            "class_scope": dict(CLASSES),
            "expected": wanted,
            "observed": observed,
            "source_control": control.get("observed"),
            "original_dex": raw_row,
        }
        if setup_error is not None or not raw_row or raw_row["status"] != "PASS":
            row.update(status="SETUP_ERROR",
                       detail=setup_error or "original DEX replay disagrees with the authored expectation")
        elif control.get("observed") != wanted:
            row.update(status="SETUP_ERROR",
                       detail=control.get("detail") or "authored Java control disagrees with the expectation")
        elif observed is None:
            row.update(status="UNCHECKED",
                       detail=comparison.get("detail") or candidate_error or "no recovered event trace")
        else:
            row["status"] = "PASS" if observed == wanted else "FAIL"
        rows.append(row)
    return {"classes": publications, "original_dex": raw, "checks": rows}


def probe(binary, artifact, timeout):
    observations = {}
    for row in evaluate(binary, artifact, timeout)["checks"]:
        observations.setdefault(SCENARIO_METHODS[row["id"]], []).append(row)
    return observations


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--artifact", type=Path, default=FIXTURE / "classes.dex")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = evaluate(args.binary, args.artifact, 15)
    if args.binary is not None:
        with args.binary.open("rb") as file:
            report["binary_sha256"] = file_digest(file, "sha256").hexdigest()
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    counts = {row["status"] for row in report["checks"]}
    for row in report["checks"]:
        print(row["id"], row["status"], row["observed"])
    return 2 if counts & {"SETUP_ERROR", "UNCHECKED"} else int("FAIL" in counts)


if __name__ == "__main__":
    raise SystemExit(main())
