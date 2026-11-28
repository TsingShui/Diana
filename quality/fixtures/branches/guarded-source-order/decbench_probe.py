"""Execute selected guarded effects against independently authored helper behavior."""

import json
import os
from pathlib import Path
from runpy import run_path
import subprocess
from tempfile import TemporaryDirectory

from evidence import file_digest, method


FIXTURE = Path(__file__).resolve().parent
SOURCE = FIXTURE / "GuardedSourceOrder.java"
HARNESS = FIXTURE / "CheckGuardedSourceOrder.java"
MANIFEST = FIXTURE / "decbench.json"
METHODS = run_path(str(FIXTURE.parents[2] / "lib.py"))["methods"]


def run(arguments, work, timeout):
    environment = {
        name: value for name, value in os.environ.items()
        if name not in {"CLASSPATH", "_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "JDK_JAVAC_OPTIONS"}
    }
    completed = subprocess.run(arguments, cwd=work, env=environment, check=False, timeout=timeout,
                               capture_output=True, encoding="utf-8")
    if completed.returncode:
        raise ValueError(completed.stderr.strip() or f"command exited {completed.returncode}")
    return completed.stdout


def execute(work, text, name, scenarios, timeout):
    work.mkdir()
    source = work / SOURCE.name
    source.write_text(text, encoding="utf-8")
    run(["java", "-XX:-UsePerfData", "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none", "-encoding", "UTF-8",
         "-classpath", str(work), "-d", str(work), str(source), str(HARNESS)], work, timeout)
    arguments = [value for scenario in scenarios for value in scenario["arguments"]]
    output = run(["java", "-XX:-UsePerfData", "-cp", str(work), "fixture.CheckGuardedSourceOrder", name, *arguments],
                 work, timeout)
    observed = [line.split(",") for line in output.splitlines()]
    if len(observed) != len(scenarios):
        raise ValueError("execution returned an unexpected number of scenarios")
    return observed


def setup_rows(scenarios, status, detail):
    return [{"id": scenario["id"], "status": status, "expected": scenario["expected"],
             "observed": None, "detail": detail} for scenario in scenarios]


def probe(binary: Path, artifact: Path, timeout: int) -> dict[str, list[dict]]:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    by_name = json.loads((FIXTURE / "scenarios.json").read_text(encoding="utf-8"))
    reference = SOURCE.read_text(encoding="utf-8")
    declarations = METHODS(reference, SOURCE.stem)
    pinned = file_digest(artifact) == manifest["input_sha256"]
    results = {}
    for target in manifest["methods"]:
        at = target["at"]
        name = target["member"].split("->", 1)[1].split("(", 1)[0]
        scenarios = by_name[name]
        if not pinned:
            results[at] = setup_rows(scenarios, "SETUP_ERROR", "input differs from the authored DEX")
            continue
        expected = [scenario["expected"] for scenario in scenarios]
        with TemporaryDirectory(prefix="decbench-guarded-order-") as directory:
            work = Path(directory)
            try:
                control = execute(work / "reference", reference, name, scenarios, timeout)
                if control != expected:
                    raise ValueError("authored source disagrees with the independent scenario table")
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                results[at] = setup_rows(scenarios, "SETUP_ERROR", str(error))
                continue
            try:
                capture = method(binary, artifact, at, timeout)
                if "error" in capture or capture["issues"]:
                    raise ValueError(capture.get("error", "selected publication contains a recovery issue"))
                body = METHODS(capture["text"], SOURCE.stem).get(name)
                if body is None:
                    raise ValueError("selected method body is unavailable")
                text = reference.replace(declarations[name], body, 1)
                observed = execute(work / "observed", text, name, scenarios, timeout)
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                results[at] = setup_rows(scenarios, "UNCHECKED", str(error))
                continue
        results[at] = [
            {"id": scenario["id"], "status": "PASS" if actual == scenario["expected"] else "FAIL",
             "expected": scenario["expected"], "observed": actual}
            for scenario, actual in zip(scenarios, observed)
        ]
    return results
