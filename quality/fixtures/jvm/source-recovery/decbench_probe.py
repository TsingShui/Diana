"""Judge authored ExceptionChains method scenarios in an isolated classpath."""

from __future__ import annotations

import json
import os
from pathlib import Path
import runpy
import subprocess
import tempfile

FIXTURE = Path(__file__).resolve().parent
CLASS = Path("fixture/jvm/ExceptionChains.class")
RECIPE = runpy.run_path(str(FIXTURE / "prepare.py"))
JAVAC = RECIPE["JAVAC"]
# Observations come from the independent CheckHandlers.java and the fixture README.
SCENARIOS = {
    "j0:m1": (
        "guarded",
        (
            ("quotient", "return", 7, 14),
            ("divide-zero", "return", -7, 124),
            ("bounds", "return", -8, 15),
            ("null", "throw", "java.lang.NullPointerException", 1),
        ),
    ),
    "j0:m2": (
        "guardedFinally",
        (
            ("quotient", "return", 7, 13),
            ("divide-zero", "return", -7, 123),
            ("bounds", "return", -8, 135),
            ("null", "throw", "java.lang.NullPointerException", 13),
        ),
    ),
}


def run(argv: list[str], cwd: Path, timeout: int) -> subprocess.CompletedProcess:
    environment = {
        key: value for key, value in RECIPE["java_environment"]().items()
        if not key.startswith("DEXDEC_")
    }
    return subprocess.run(
        argv, cwd=cwd, env=environment, timeout=timeout, check=True,
        capture_output=True, encoding="utf-8", errors="replace",
    )


def expectation(kind: str, outcome: int | str, trace: int) -> dict:
    return {"kind": kind, "outcome": outcome, "trace": trace}


def unavailable(status: str, detail: str) -> dict[str, list[dict]]:
    return {
        locator: [
            {
                "id": scenario, "status": status, "expected": expectation(kind, outcome, trace),
                "observed": None, "detail": detail,
            }
            for scenario, kind, outcome, trace in scenarios
        ]
        for locator, (_, scenarios) in SCENARIOS.items()
    }


def explain(error: Exception) -> str:
    if isinstance(error, subprocess.CalledProcessError):
        return error.stderr.strip() or f"command exited with status {error.returncode}"
    return str(error)


def probe(binary: Path, artifact: Path, timeout: int) -> dict[str, list[dict]]:
    binary = binary.resolve()
    artifact = artifact.resolve()
    with tempfile.TemporaryDirectory(prefix="diana-exception-judge-") as directory:
        scratch = Path(directory)
        reference = scratch / "reference"
        witness = scratch / "witness"
        rendered = scratch / "rendered"
        try:
            original = reference / CLASS
            for variant in RECIPE["VARIANTS"]:
                compile_command = RECIPE["compile_command"](
                    reference, variant, [FIXTURE / "ExceptionChains.java"],
                )
                run(compile_command, scratch, timeout)
                if (
                    artifact.stat().st_size == original.stat().st_size
                    and artifact.read_bytes() == original.read_bytes()
                ):
                    break
            else:
                return unavailable("SETUP_ERROR", "Expected an authored ExceptionChains.class variant")
            run([
                *JAVAC, "-classpath", str(reference), "-encoding", "UTF-8",
                "-d", str(witness), str(FIXTURE / "DecBenchHandlers.java"),
            ], scratch, timeout)
        except (OSError, subprocess.SubprocessError) as error:
            return unavailable("SETUP_ERROR", explain(error))

        # Run on the same trusted bytes in the scratch directory. The original
        # compilation is never present on the recovered class's runtime classpath.
        source = scratch / "ExceptionChains.java"
        try:
            response = run([str(binary), "get-class", str(original), "j0", "--format", "json"], scratch, timeout)
            publication = json.loads(response.stdout)
            selection = publication["selection"]
            text = publication["result"]["text"]
            if (
                publication["schema_version"] != 5
                or publication["command"] != "get-class"
                or selection["class"] != "Lfixture/jvm/ExceptionChains;"
                or selection["class_at"].split("::", 1)[0] != "j0"
                or not isinstance(text, str)
            ):
                raise ValueError("Unexpected Diana class publication")
            source.write_text(text, encoding="utf-8")
            run([*JAVAC, "-encoding", "UTF-8", "-d", str(rendered), str(source)], scratch, timeout)
        except (OSError, subprocess.SubprocessError, ValueError, TypeError, KeyError, AttributeError) as error:
            return unavailable("UNCHECKED", explain(error))

        results = {}
        argv = ["java", "-classpath", os.pathsep.join((str(rendered), str(witness))), "DecBenchHandlers"]
        for locator, (method, scenarios) in SCENARIOS.items():
            results[locator] = []
            for scenario, kind, outcome, trace in scenarios:
                expected = expectation(kind, outcome, trace)
                row = {"id": scenario, "expected": expected, "observed": None}
                try:
                    response = run(argv + [method, scenario], scratch, timeout)
                    seen_kind, seen_outcome, seen_trace = response.stdout.rstrip("\n").split("\t")
                    if seen_kind not in ("return", "throw"):
                        raise ValueError(f"Invalid witness outcome: {seen_kind!r}")
                    value = int(seen_outcome) if seen_kind == "return" else seen_outcome
                    observed = expectation(seen_kind, value, int(seen_trace))
                    row.update(status="PASS" if expected == observed else "FAIL", observed=observed)
                except (OSError, subprocess.SubprocessError, ValueError) as error:
                    row.update(status="UNCHECKED", detail=explain(error))
                results[locator].append(row)
        return results
