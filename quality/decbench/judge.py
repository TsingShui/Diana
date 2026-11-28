"""Evaluate one Diana executable against independently authored fixture contracts."""

from __future__ import annotations

from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import importlib
import json
from pathlib import Path
import runpy
import subprocess
import time

import dex_truth
import jvm_truth
from evidence import digest, file_digest, inventory, method, write_json

QUALITY = Path(__file__).resolve().parent.parent
STATUSES = ("FAIL", "SETUP_ERROR", "UNCHECKED", "PASS")
ADAPTERS = {
    "structure": ("flow_shape", "FlowShape.java"),
    "local_types": ("local_types", "LocalTypes.java"),
    "call_bindings": ("call_bindings", "CallBindings.java", "dex_calls.py",
                      "dex_truth.py", "local_types.py"),
    "effect_order": ("effect_order", "EffectOrder.java", "local_types.py", "dex_truth.py"),
}
METRICS = ("behavior", *ADAPTERS)


def definitions(suite: dict) -> bool:
    return bool(suite.get("dex_definitions") or suite.get("jvm_definitions"))


def read_suite(manifest: Path) -> dict:
    suite = json.loads(manifest.read_text(encoding="utf-8"))
    if suite.get("schema_version") != 1 or not isinstance(suite.get("id"), str):
        raise ValueError(f"{manifest}: expected a version-1 suite")
    if not isinstance(suite.get("input"), str):
        raise ValueError(f"{manifest}: expected an input path")
    entries = suite.get("methods", [])
    if not isinstance(entries, list) or (not entries and not definitions(suite)):
        raise ValueError(f"{manifest}: expected independently specified methods or a definition oracle")
    if any(not isinstance(entry, dict) or any(
        not isinstance(entry.get(field), str) or not entry[field] for field in ("at", "member")
    ) for entry in entries):
        raise ValueError(f"{manifest}: each method needs a locator and an original signature")
    for metric in ADAPTERS:
        requested = any(entry.get(metric) for entry in entries)
        config = suite.get(metric, {})
        if not isinstance(config, dict) or (requested and metric not in suite):
            raise ValueError(f"{manifest}: {metric} checks need a suite configuration object")
        if "reference" in config and (not isinstance(config["reference"], str) or not config["reference"]):
            raise ValueError(f"{manifest}: {metric} reference must be a relative input path")
    suite["methods"] = entries
    keys = [(entry["at"], entry["member"]) for entry in entries]
    if len(set(keys)) != len(keys) or len({at for at, _ in keys}) != len(keys):
        raise ValueError(f"{manifest}: duplicate method locator")
    suite["manifest"] = manifest
    return suite


def load_suites(fixture_root: Path) -> dict[str, dict]:
    suites = {}
    for manifest in sorted(fixture_root.glob("*/*/decbench*.json")):
        suite = read_suite(manifest)
        if suite["id"] in suites:
            raise ValueError(f"duplicate suite {suite['id']}")
        suites[suite["id"]] = suite
    for artifact in sorted(fixture_root.glob("*/*/**/classes.dex")):
        identifier = "definitions:" + str(artifact.parent.relative_to(fixture_root))
        suites[identifier] = {
            "id": identifier, "manifest": artifact, "input": artifact.name,
            "methods": [], "dex_definitions": True, "discovered": True,
        }
    if not suites:
        raise ValueError(f"no suite manifests in {fixture_root}")
    return suites


def external_suite(artifact: Path) -> dict:
    suffix = artifact.suffix.lower()
    if suffix not in (".apk", ".dex", ".class", ".jar"):
        raise ValueError(f"no independent definition oracle for {artifact.suffix!r}")
    dex = suffix in (".apk", ".dex")
    checksum = file_digest(artifact)
    return {
        "id": f"external:{artifact.name}:{checksum}", "manifest": artifact,
        "input": artifact.name, "input_sha256": checksum,
        "methods": [], "dex_definitions": dex, "jvm_definitions": not dex,
    }


def child_path(root: Path, relative: str) -> Path:
    root = root.resolve()
    path = (root / relative).resolve()
    if not path.is_relative_to(root):
        raise ValueError(f"{relative!r} escapes its fixture directory")
    return path


def status(checks: list[dict]) -> str:
    return min((check["status"] for check in checks), key=STATUSES.index)


def check(check_id: str, metric: str, outcome: str, expected, observed, detail=None) -> dict:
    result = {"id": check_id, "metric": metric, "status": outcome,
              "expected": expected, "observed": observed}
    if detail:
        result["detail"] = detail
    return result


def equality(check_id: str, metric: str, expected, observed) -> dict:
    return check(check_id, metric, "PASS" if expected == observed else "FAIL", expected, observed)


def match_issues(required: list[dict], observed: list[dict]) -> bool:
    unmatched = observed.copy()
    for expected_issue in required:
        for index, issue in enumerate(unmatched):
            if all(issue.get(key) == value for key, value in expected_issue.items()):
                del unmatched[index]
                break
        else:
            return False
    return not unmatched


def fact_checks(spec: dict, seen: set, coverage: dict, observation: dict | None) -> list[dict]:
    at, member = spec["at"], spec["member"]
    expected = {"at": at, "member": member}
    if (at, member) not in seen:
        found = next((name for locator, name in seen if locator == at), None)
        if not coverage["complete"]:
            outcome = "SETUP_ERROR" if "error" in coverage else "UNCHECKED"
            return [check("identity", "identity", outcome, expected, found,
                          coverage.get("error", "method inventory is incomplete"))]
        return [check("identity", "identity", "FAIL", expected, found)]
    checks = [equality("identity", "identity", expected, expected)]
    if spec.get("inventory_only"):
        return checks
    if observation is None or "error" in observation:
        detail = observation["error"] if observation else "selected publication did not run"
        return checks + [check("publication", "publication", "UNCHECKED", "method text", None, detail)]
    text = observation["text"]
    checks.append(equality("publication", "publication", True, bool(text.strip())))
    if "issues" in spec:
        issues = observation["issues"]
        outcome = "PASS" if match_issues(spec["issues"], issues) else "FAIL"
        checks.append(check("diagnostics", "diagnostics", outcome, spec["issues"], issues))
    for section, present in (("contains", True), ("omits", False)):
        for fragment in spec.get(section, []):
            checks.append(equality(f"{section}:{fragment}", "publication", present, fragment in text))
    return checks


def probe_results(suite: dict, binary: Path, artifact: Path, timeout: int) -> dict:
    results = {}
    for metric in METRICS:
        if not any(spec.get(metric) for spec in suite["methods"]):
            continue
        try:
            if metric == "behavior":
                path = child_path(suite["manifest"].parent, suite["probe"])
                observe = runpy.run_path(str(path))["probe"]
                observations = observe(binary, artifact, timeout)
            else:
                adapter = importlib.import_module(ADAPTERS[metric][0])
                observations = adapter.compare(binary, artifact, suite, timeout)
            results[metric] = (observations, None)
        except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
            results[metric] = ({}, str(error))
    return results


def append_probe_checks(checks: list[dict], spec: dict, results: dict) -> None:
    for metric in METRICS:
        if not spec.get(metric):
            continue
        observed, error = results.get(metric, ({}, None))
        rows = observed.get(spec["at"])
        if not rows:
            outcome = "SETUP_ERROR" if error is not None else "UNCHECKED"
            checks.append(check(
                f"{metric}:coverage", metric, outcome, "independent observations", None,
                error or "no observation for this selected original method",
            ))
            continue
        ids = set()
        for row in rows:
            if row.get("status") not in STATUSES or row["id"] in ids:
                raise ValueError(f"{spec['at']}: duplicate or invalid {metric} observation")
            ids.add(row["id"])
            checks.append({**row, "id": f"{metric}:{row['id']}", "metric": metric})


def reference_files(suite: dict) -> list[Path]:
    root = suite["manifest"].parent
    names = dict.fromkeys(suite.get("oracle_files", []))
    for metric in ADAPTERS:
        if reference := suite.get(metric, {}).get("reference"):
            names[reference] = None
    return [child_path(root, name) for name in names]


def oracle_hash(suite: dict) -> str:
    manifest = suite["manifest"]
    profile = "identity" if suite.get("inventory_only") else "identity-and-publication"
    if suite.get("dex_definitions"):
        return digest([file_digest(Path(dex_truth.__file__)), profile])
    if suite.get("jvm_definitions"):
        return digest([file_digest(Path(jvm_truth.__file__)),
                       file_digest(QUALITY / "decbench/cfr.py"), profile])
    files = [manifest]
    if suite.get("probe"):
        files.append(child_path(manifest.parent, suite["probe"]))
    for metric, (module, *helpers) in ADAPTERS.items():
        if metric in suite:
            files += [QUALITY / "decbench" / name for name in (f"{module}.py", *helpers)]
    fingerprints = [[path.name, file_digest(path)] for path in files]
    for path in reference_files(suite):
        try:
            checksum = file_digest(path)
        except (OSError, ValueError):
            checksum = None
        fingerprints.append([path.name, checksum])
    return digest(fingerprints)


def run_suite(suite: dict, binary: Path, prepared: Path, timeout: int) -> dict:
    manifest = suite["manifest"]
    base = prepared if suite.get("prepared") else manifest.parent
    artifact = child_path(base, suite["input"])
    rows = []
    result = {
        "suite": suite["id"], "path": str(artifact), "sha256": None,
        "inventory_only": suite.get("inventory_only", False),
        "oracle_sha256": oracle_hash(suite), "methods": rows,
        "oracle_sources": [str(path) for path in reference_files(suite)],
    }
    seen, probes = set(), {}
    specs = suite["methods"]
    expected = set()
    try:
        checksum = file_digest(artifact)
        result["sha256"] = checksum
        if suite.get("input_sha256", checksum) != checksum:
            raise ValueError("prepared input differs from the authored fixture digest")
        if suite.get("dex_definitions"):
            get_definitions = dex_truth.apk_methods if artifact.suffix.lower() == ".apk" else dex_truth.methods
            expected = set(get_definitions(artifact))
        if suite.get("jvm_definitions"):
            expected = set(jvm_truth.methods(artifact, timeout))
        if definitions(suite):
            specs = [
                {"at": at, "member": member, "inventory_only": suite.get("inventory_only", False)}
                for at, member in sorted(expected)
            ]
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        coverage = {"complete": False, "error": str(error)}
    else:
        seen, coverage = inventory(binary, artifact, timeout)
        if not coverage.get("error"):
            probes = probe_results(suite, binary, artifact, timeout)
    result["inventory"] = coverage
    if definitions(suite) and coverage["complete"]:
        for at, member in sorted(seen - expected):
            issue = check("original-definition", "identity", "FAIL", None, {"at": at, "member": member})
            rows.append({"at": at, "member": member, "status": "FAIL", "checks": [issue],
                         "observation": None if suite.get("inventory_only") else
                         method(binary, artifact, at, timeout)})
    for spec in specs:
        at, member = spec["at"], spec["member"]
        observation = None
        if (at, member) in seen and not spec.get("inventory_only"):
            observation = method(binary, artifact, at, timeout)
        checks = fact_checks(spec, seen, coverage, observation)
        append_probe_checks(checks, spec, probes)
        rows.append({"at": at, "member": member, "status": status(checks),
                     "checks": checks, "observation": observation})
    if result["sha256"] is not None and file_digest(artifact) != result["sha256"]:
        raise ValueError(f"input changed during evaluation: {artifact}")
    return result


def run_or_resume(suite: dict, binary: Path, binary_hash: str, prepared: Path,
                  timeout: int, checkpoint: Path | None, evaluator_hash: str) -> dict:
    if checkpoint is None:
        return run_suite(suite, binary, prepared, timeout)
    base = prepared if suite.get("prepared") else suite["manifest"].parent
    artifact = child_path(base, suite["input"])
    if not artifact.is_file():
        return run_suite(suite, binary, prepared, timeout)
    path = checkpoint / (digest([
        str(artifact), file_digest(artifact), str(binary), binary_hash,
        suite["id"], oracle_hash(suite), suite["methods"], evaluator_hash,
        suite.get("inventory_only", False),
    ]) + ".json")
    if path.is_file():
        return {**json.loads(path.read_text(encoding="utf-8")), "resumed": True}
    result = run_suite(suite, binary, prepared, timeout)
    if result["inventory"]["complete"] and all(
        check["status"] in ("PASS", "FAIL") for row in result["methods"] for check in row["checks"]
    ):
        write_json(path, result)
    return result


def run_header(binary: Path, binary_hash: str, prepared: Path, revision: str | None,
               backend: str, evaluator_hash: str) -> dict:
    return {
        "schema_version": 1, "kind": "diana-decbench-run",
        "created_at": datetime.now(timezone.utc).isoformat(), "revision": revision,
        "binary": {"path": str(binary), "sha256": binary_hash}, "backend": backend,
        "prepared": str(prepared), "evaluator_sha256": evaluator_hash,
    }


def summarize(report: dict, started: float) -> dict:
    report["methods_by_status"] = dict(Counter(
        row["status"] for item in report["inputs"] for row in item["methods"]
    ))
    report["checks_by_status"] = dict(Counter(
        check["status"] for item in report["inputs"] for row in item["methods"]
        for check in row["checks"]
    ))
    metrics = defaultdict(lambda: {"checks": Counter(), "method_rows": Counter()})
    measured = {"PASS", "FAIL"}
    for item in report["inputs"]:
        for row in item["methods"]:
            observed = defaultdict(set)
            for result in row["checks"]:
                metric, outcome = result["metric"], result["status"]
                metrics[metric]["checks"][outcome] += 1
                observed[metric].add(outcome)
            for metric, outcomes in observed.items():
                if outcomes <= measured:
                    scope = "fully_measured"
                elif outcomes & measured:
                    scope = "partially_measured"
                else:
                    scope = "unmeasured"
                metrics[metric]["method_rows"][scope] += 1
    report["metrics"] = dict(sorted(metrics.items()))
    report["inventory_complete"] = all(item["inventory"]["complete"] for item in report["inputs"])
    report["elapsed_seconds"] = time.perf_counter() - started
    return report


def result_code(report: dict) -> int:
    counts = report["checks_by_status"]
    inspection_failed = any(item.get("corpus", {}).get("source_structure", {}).get("SETUP_ERROR")
                            for item in report["inputs"])
    if (not report["inventory_complete"] or counts.get("SETUP_ERROR") or counts.get("UNCHECKED")
            or inspection_failed):
        return 2
    return 1 if counts.get("FAIL") else 0


def evaluator_fingerprint() -> str:
    here = Path(__file__)
    return digest([file_digest(here), file_digest(here.with_name("evidence.py"))])


def run(binary: Path, prepared: Path, suites: list[dict], revision: str | None,
        jobs: int, timeout: int, backend: str, checkpoint: Path | None) -> dict:
    started = time.perf_counter()
    binary_hash = file_digest(binary)
    evaluator_hash = evaluator_fingerprint()
    report = run_header(binary, binary_hash, prepared, revision, backend, evaluator_hash)
    with ThreadPoolExecutor(max_workers=jobs) as workers:
        report["inputs"] = list(workers.map(
            lambda suite: run_or_resume(
                suite, binary, binary_hash, prepared, timeout, checkpoint, evaluator_hash
            ), suites
        ))
    if file_digest(binary) != binary_hash:
        raise ValueError("Diana executable changed during evaluation")
    return summarize(report, started)
