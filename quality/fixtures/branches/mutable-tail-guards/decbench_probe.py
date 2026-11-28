"""Replay the authored mutable-tail-guard cases on the retrieved class."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
from tempfile import TemporaryDirectory


FIXTURE = Path(__file__).resolve().parent
DEX_SHA256 = "3fcec13c31d98cc5a56c8e3a51494eed71f1410ecd5303cf96e08f9c8e214b6e"
CLASS = "e0:l0:c1"
METHODS = {"route": f"{CLASS}:m2", "extended": f"{CLASS}:m1"}
JAVAC = ["java", "-m", "jdk.compiler/com.sun.tools.javac.Main", "-encoding", "UTF-8"]


def nodes(value: str) -> list[int]:
    return [int(node) for node in value.split(",")] if value else []


def fixture_scenarios() -> dict[str, dict]:
    scenarios = {}
    for line in (FIXTURE / "scenarios.tsv").read_text(encoding="utf-8").splitlines():
        if line.startswith("#"):
            continue
        method, seed, initial, terminal, call_path = line.split("\t")
        original = nodes(call_path)
        for fault in range(-1, int(terminal) + 1):
            path = original if fault not in original else original[: original.index(fault) + 1]
            trace = int(initial)
            for node in path:
                trace = ((trace * 31 + node + 2**31) % 2**32) - 2**31
            expected = {
                "outcome": {"kind": "fault", "value": fault} if fault in original else {
                    "kind": "return", "value": int(terminal)
                },
                "calls": path, "trace": trace, "argument_mismatches": [],
            }
            key = f"{method}/{seed}/{initial}/{fault}"
            scenarios[key] = {"locator": METHODS[method], "expected": expected}
    return scenarios


def run(arguments: list[str], work: Path, timeout: int) -> str:
    environment = {
        name: value for name, value in os.environ.items()
        if name not in {"CLASSPATH", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS"}
        and not name.startswith("DEXDEC_")
    }
    completed = subprocess.run(
        arguments, cwd=work, env=environment, check=False, timeout=timeout,
        capture_output=True, encoding="utf-8",
    )
    if completed.returncode:
        raise ValueError(f"{arguments[0]} exited {completed.returncode}: {completed.stderr.strip()}")
    return completed.stdout


def observations(text: str, scenarios: dict) -> dict:
    seen = {}
    for line in text.splitlines():
        method, seed, initial, fault, kind, value, path, trace, bad_args = line.split("\t")
        key = f"{method}/{seed}/{initial}/{fault}"
        if key in seen or key not in scenarios or kind not in {"return", "fault", "exception"}:
            raise ValueError(f"unrecognized observation {key}")
        seen[key] = {
            "outcome": {"kind": kind, "value": value if kind == "exception" else int(value)},
            "calls": nodes(path), "trace": int(trace),
            "argument_mismatches": bad_args.split(",") if bad_args else [],
        }
    if seen.keys() != scenarios.keys():
        raise ValueError(f"missing {len(scenarios.keys() - seen.keys())} scenario observations")
    return seen


def probe(binary: Path, artifact: Path, timeout: int) -> dict[str, list[dict]]:
    scenarios = fixture_scenarios()
    status = "SETUP_ERROR"
    observed = {}
    detail = None
    try:
        data = artifact.read_bytes()
        if hashlib.sha256(data).hexdigest() != DEX_SHA256:
            raise ValueError("input differs from the checked-in mutable-tail-guards fixture")
        with TemporaryDirectory() as directory:
            work = Path(directory)
            dex = work / "classes.dex"
            dex.write_bytes(data)
            run(JAVAC + ["-version"], work, timeout)
            status = "UNCHECKED"
            document = json.loads(run(
                [str(binary.resolve()), "get-class", str(dex), CLASS, "--format", "json"],
                work, timeout,
            ))
            if (
                document["schema_version"] != 5 or document["command"] != "get-class"
                or document["selection"]["class_at"].partition("::")[0] != CLASS
                or document["issues"]
            ):
                raise ValueError("publication does not establish the selected fixture class")
            source = work / "MutableGuardRoutes.java"
            source.write_text(document["result"]["text"], encoding="utf-8")
            run(JAVAC + ["-d", str(work), str(source), str(FIXTURE / "CheckMutableRoutes.java")],
                work, timeout)
            stdout = run([
                "java", "-cp", str(work), "fixture.CheckMutableRoutes", str(FIXTURE / "scenarios.tsv"),
            ], work, timeout)
            observed = observations(stdout, scenarios)
    except (OSError, subprocess.SubprocessError, ValueError, KeyError, TypeError, AttributeError) as error:
        detail = str(error)

    results = {locator: [] for locator in METHODS.values()}
    for case_id, scenario in scenarios.items():
        expected = scenario["expected"]
        actual = observed.get(case_id)
        verdict = status if actual is None else ("PASS" if actual == expected else "FAIL")
        row = {"id": case_id, "status": verdict, "expected": expected, "observed": actual}
        if detail is not None:
            row["detail"] = detail
        results[scenario["locator"]].append(row)
    return results
