"""Evaluate independently specified methods in an opt-in public APK corpus."""

from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
from tarfile import TarError
import time
from zipfile import BadZipFile

import corpus_assets
import corpus_models
import corpus_oracles
import corpus_types
import evidence
import judge
import legacy_diana


class EvidenceChanged(ValueError):
    """The requested original or oracle differs from the saved evaluation."""


def case_configuration(project, case_path):
    path = case_path or project.manifest.with_name(f"{project.id}.cases.json")
    config = corpus_oracles.load_cases(path)
    if config["source_revision"] != project.source_commit:
        raise ValueError(f"{project.id}: source revision differs between the project and cases")
    if project.apk.sha256 is not None and config["input_sha256"] != project.apk.sha256:
        raise ValueError(f"{project.id}: pinned APK differs between the project and cases")
    return path, config


def select_cases(project, verified, selected):
    target = set(selected or verified)
    if not target or not target <= verified.keys():
        missing = ", ".join(sorted(target - verified.keys()))
        raise ValueError(f"{project.id}: no such reviewed original method: {missing}")
    return [case for at, case in verified.items() if at in target]


def native_publications(binary, artifact, cases, jobs, timeout):
    inventory, capture = evidence.inventory(binary, artifact, timeout)
    collector = evidence.method
    if legacy_diana.unsupported_unlimited(capture):
        inventory, capture = legacy_diana.inventory(binary, artifact, timeout)
        collector = legacy_diana.method
    by_locator = {}
    conflicting = set()
    for at, member in inventory:
        if at in by_locator and by_locator[at] != member:
            conflicting.add(at)
        by_locator[at] = member
    for at in conflicting:
        del by_locator[at]
    present = [case for case in cases if case.get("original_verified", True)
               and by_locator.get(case["at"]) == case["member"]]

    def retrieve(case):
        return collector(binary, artifact, case["at"], timeout, member=case["member"])

    with ThreadPoolExecutor(max_workers=jobs) as workers:
        observations = {case["at"]: value for case, value in zip(present, workers.map(retrieve, present))}
    summary = {
        "complete": capture["complete"], "scope": "whole APK inventory", "observed": len(inventory),
        **({"error": capture["error"]} if "error" in capture else {}),
    }
    if capture.get("error") and not inventory:
        summary["setup_error"] = True
        summary["producer_diagnostics"] = capture.get("stderr", "")[:16_384]
    return observations, by_locator, summary


def peer_publications(binary, artifact, cases, timeout):
    import jadx

    requested = [case for case in cases if case.get("original_verified", True)]
    published = jadx.publish(binary, artifact, requested, timeout)
    observations, by_locator = {}, {}
    bodies = Counter()
    for case in requested:
        at, member = case["at"], case["member"]
        item = published["methods"].get(at, {})
        if (item.get("at"), item.get("member")) != (at, member):
            observations[at] = {"error": "the peer did not confirm the requested original identity"}
            continue
        available = item.get("status") == "AVAILABLE" and isinstance(item.get("text"), str)
        if available or item.get("identified") is True:
            by_locator[at] = member
        diagnostics = item.get("diagnostics", [])
        if available:
            observations[at] = {"text": item["text"], "issues": [], "diagnostics": diagnostics}
            if isinstance(item.get("has_body"), bool):
                observations[at]["has_body"] = item["has_body"]
            if item.get("complete") is False:
                observations[at]["error"] = "the peer only recovered a partial method"
                bodies["partial"] += 1
            else:
                bodies["complete"] += 1
        else:
            observations[at] = {"error": "the peer did not supply a recoverable method",
                                "diagnostics": diagnostics}
            bodies["unavailable"] += 1
    summary = {"complete": len(by_locator) == len(requested), "scope": "selected original methods",
               "observed": len(by_locator), "producer": published["provenance"], "publications": dict(bodies)}
    messages = published.get("diagnostics", [])
    if messages:
        summary["producer_diagnostics"] = messages
    worker = published.get("worker")
    reason = None
    if worker:
        summary["worker"] = worker
        if worker["status"] not in ("COMPLETE", "NOT_RUN"):
            reason = worker.get("error") or f"peer worker {worker['status'].lower()}"
    if not reason and not by_locator and messages:
        reason = messages[0]
    if reason:
        summary.update(setup_error=True, error=str(reason), complete=False)
        for stream in ("stdout", "stderr"):
            if published.get(stream):
                summary[f"producer_{stream}"] = published[stream][:16_384]
    return observations, by_locator, summary


def recovery_body(case, observation):
    metric = "body_recovery"
    if "original_code_units" not in case:
        return judge.check(metric, metric, "UNCHECKED", True, None, case.get(
            "original_body_error", "the original DEX body was not established",
        ))
    if observation.get("has_body") is False:
        return judge.check(metric, metric, "FAIL", True, False, "the peer emitted no recovered body")
    if observation.get("error"):
        return judge.check(metric, metric, "UNCHECKED", True, None, observation["error"])
    if observation.get("has_body") is True:
        return judge.equality(metric, metric, True, True)
    text = observation.get("text")
    try:
        if not isinstance(text, str) or len(text.encode("utf-8")) > corpus_oracles.BYTE_LIMIT:
            raise corpus_oracles.Unavailable("the selected publication was unavailable")
        view = corpus_oracles.View(text)
        owner_scope = corpus_types.class_scope(view, case["member"].partition("->")[0])
        if view.declared_without_body(case["member"], owner_scope):
            return judge.check(metric, metric, "FAIL", True, False, "only the selected declaration was published")
        opening, closing = view.method_body(case["member"])
        if view.scopes[opening] != owner_scope:
            raise corpus_oracles.Unavailable("the displayed body could not be tied to the original declaring owner")
        if observation.get("issues") and closing == opening + 1:
            raise corpus_oracles.Unavailable("the publication reports recovery issues and has no visible statements")
    except (corpus_oracles.Unavailable, UnicodeError) as error:
        return judge.check(metric, metric, "UNCHECKED", True, None, str(error))
    return judge.equality(metric, metric, True, True)


def checked_method(case, observation, original_member, inventory):
    at, member = case["at"], case["member"]
    expected = {"at": at, "member": member}
    original_proven = case.get("original_verified", True)
    if not original_proven:
        identity = judge.check("identity", "identity", "SETUP_ERROR", expected, None,
                               case.get("original_error") or "original method identity could not be verified")
    elif original_member == member:
        identity = judge.equality("identity", "identity", expected, expected)
    else:
        actual = {"at": at, "member": original_member} if original_member else None
        if inventory.get("setup_error"):
            status = "SETUP_ERROR"
        else:
            status = "FAIL" if inventory["complete"] else "UNCHECKED"
        identity = judge.check("identity", "identity", status, expected, actual,
                               inventory.get("error") or (observation or {}).get("error"))
    checks = [identity]
    if original_proven and original_member == member and observation and "error" not in observation:
        checks.append(judge.equality("publication", "publication", True, bool(observation["text"].strip())))
    else:
        checks.append(judge.check("publication", "publication", "UNCHECKED", True, None,
                                  (observation or {}).get("error", "original method was unavailable")))
    oracle_observation = observation if original_proven and original_member == member else None
    checks.append(recovery_body(case, oracle_observation or {}))
    checks.extend(corpus_types.compare(case, oracle_observation or {}))
    observations = corpus_oracles.compare(case, oracle_observation or {"error": "original method was unavailable"})
    ids = set()
    for result in observations:
        if result["id"] in ids or result["status"] not in judge.STATUSES:
            raise ValueError(f"{at}: duplicate or invalid public corpus observation")
        ids.add(result["id"])
        if oracle_observation and oracle_observation.get("error") and result["status"] == "FAIL":
            result = {**result, "status": "UNCHECKED", "detail": oracle_observation["error"]}
        metric = {"char_arguments": "call_argument_types", "argument_values": "call_argument_values",
                  "field_access": "field_bindings"}.get(result.get("kind"), "corpus_quality")
        checks.append({"metric": metric, **result})
    row = {"at": at, "member": member, "status": judge.status(checks), "checks": checks,
           "observation": observation, "source": case["source"],
           "provenance": {key: case[key] for key in ("original_verified", "original_code_units", "original_body_error", "source_verified",
                                                     "original_error", "source_error") if key in case},
           **{key: case[key] for key in ("module", "source_forms") if key in case}}
    return row


def evaluator_fingerprint(backend, inspect_structure):
    here = Path(__file__).parent
    files = [Path(__file__), Path(evidence.__file__), Path(judge.__file__), here / "tool_capture.py"]
    if backend == "jadx":
        import jadx

        files.extend((Path(jadx.__file__), jadx.HELPER))
    else:
        files.append(Path(legacy_diana.__file__))
    if inspect_structure:
        files.extend(here / name for name in ("corpus_structure.py", "flow_shape.py", "FlowShape.java"))
    return evidence.digest([evidence.file_digest(path) for path in files])


def run(project, cache: Path, binary: Path, revision: str | None, backend: str,
        selected: list[str] | None, jobs: int, timeout: int, *, execute_models: bool = False,
        case_path: Path | None = None, checkpoint: Path | None = None, inspect_structure: bool = False,
        expected_evidence: dict | None = None) -> dict:
    started = time.perf_counter()
    assets = corpus_assets.prepare_assets(project, cache)
    external_cases = case_path is not None
    case_path, config = case_configuration(project, case_path)
    binary_hash = evidence.file_digest(binary)
    input_hash = evidence.file_digest(assets.apk)
    source_hash = evidence.file_digest(assets.source)
    verified = corpus_oracles.verify_cases(config, assets.apk, assets.source, project.source_root)
    cases = select_cases(project, verified, selected)
    here = Path(__file__).parent
    oracle_files = [case_path, *(here / name for name in (
        "corpus_oracles.py", "corpus_types.py", "corpus_discover.py", "dex_calls.py", "dex_truth.py",
    ))]
    if any(obligation["kind"] == "field_access"
           for case in verified.values() for obligation in case["obligations"]):
        oracle_files.append(here / "corpus_fields.py")
    models = {}
    model_errors = {}
    if execute_models:
        oracle_files.append(Path(corpus_models.__file__))
        for at, case in verified.items():
            if case.get("model"):
                try:
                    model = corpus_models.read_model(case["model"])
                except (OSError, ValueError, KeyError, TypeError) as error:
                    model_errors[at] = str(error)
                else:
                    models[at] = model
                    oracle_files.extend((model.manifest, model.template_path))
                    if model.raw_probe:
                        oracle_files.extend((model.raw_probe, here / "dex_witness.py"))
    oracle_hash = evidence.digest({
        "files": [evidence.file_digest(path) for path in oracle_files],
        "source_revision": project.source_commit,
        "source_methods": [[at, case["source"].get("sha256")] for at, case in sorted(verified.items())],
        "unavailable_models": sorted(model_errors),
    })
    if expected_evidence is not None:
        if len(cases) != 1:
            raise ValueError("fresh triage requires exactly one original method")
        case, = cases
        actual = {"input_sha256": input_hash, "source_archive_sha256": source_hash,
                  "oracle_sha256": oracle_hash, "member": case["member"]}
        changed = [field for field, value in actual.items() if expected_evidence.get(field) != value]
        if changed:
            raise EvidenceChanged("saved case evidence differs in " + ", ".join(changed))
        error = case.get("original_error") or case.get("source_error")
        if error:
            raise ValueError("could not verify the selected original evidence: " + error)
    evaluator_hash = evaluator_fingerprint(backend, inspect_structure)
    report = judge.run_header(binary, binary_hash, cache, revision, backend, evaluator_hash)
    checkpoint_path = None
    if checkpoint is not None:
        checkpoint_path = checkpoint / (evidence.digest([
            project.id, str(assets.apk), input_hash, str(assets.source), source_hash, str(case_path), oracle_hash,
            str(binary), binary_hash, backend, evaluator_hash, [case["at"] for case in cases],
            inspect_structure,
        ]) + ".json")
        if checkpoint_path.is_file():
            cached = json.loads(checkpoint_path.read_text(encoding="utf-8"))
            report["inputs"] = [{**cached, "resumed": True}]
            return judge.summarize(report, started)
    if input_hash != config["input_sha256"]:
        observations, by_locator = {}, {}
        inventory = {"complete": False, "observed": 0, "setup_error": True,
                     "error": "APK bytes differ from the pinned assessment input"}
    elif backend == "diana":
        observations, by_locator, inventory = native_publications(binary, assets.apk, cases, jobs, timeout)
    else:
        observations, by_locator, inventory = peer_publications(binary, assets.apk, cases, timeout)
    methods = [checked_method(case, observations.get(case["at"]), by_locator.get(case["at"]),
                              inventory) for case in cases]
    if models or model_errors:
        append_modeled_checks(methods, cases, models, model_errors, assets, project.source_root, timeout)
    structure_by_outcome = Counter()
    if inspect_structure and not inventory.get("setup_error"):
        import corpus_structure

        confirmed = [case for case in cases if case.get("original_verified", True)]
        inspected = corpus_structure.inspect(confirmed, assets.source, project.source_root, observations, timeout,
                                             backend=backend) if confirmed else {}
        for row in methods:
            receipt = inspected.get(row["at"]) or corpus_structure.receipt(
                None, None, "SETUP_ERROR", "the original method could not be verified",
            )
            row["source_structure"] = receipt
            structure_by_outcome[receipt["outcome"]] += 1
    report["inputs"] = [{
        "suite": f"public:{project.id}", "path": str(assets.apk), "sha256": input_hash,
        "oracle_sha256": oracle_hash, "inventory_only": False, "inventory": inventory, "methods": methods,
        "oracle_sources": [str(case_path), str(assets.source)],
        "corpus": {"id": project.id, "source_revision": project.source_commit,
                   "source_archive_sha256": source_hash, "catalog": str(project.manifest),
                   "cases": str(case_path), "external_cases": external_cases, "execute_models": execute_models,
                   "inspect_structure": inspect_structure,
                   "profile": "modeled" if execute_models else "discovered" if "discovery" in config else "reviewed",
                   **({"source_structure": dict(structure_by_outcome)} if inspect_structure else {}),
                   **({"discovery": config["discovery"]} if "discovery" in config else {})},
    }]
    if any(evidence.file_digest(path) != expected for path, expected in (
        (binary, binary_hash), (assets.apk, input_hash), (assets.source, source_hash),
    )):
        raise ValueError(f"{project.id}: the executable or corpus input changed during evaluation")
    judge.summarize(report, started)
    if checkpoint_path and report["inventory_complete"] and not any(
        report["checks_by_status"].get(status) for status in ("UNCHECKED", "SETUP_ERROR")
    ) and not structure_by_outcome.get("SETUP_ERROR"):
        evidence.write_json(checkpoint_path, report["inputs"][0])
    return report


def append_modeled_checks(methods, cases, models, model_errors, assets, source_root, timeout):
    def has_verified_source(case):
        if not case.get("original_verified", True):
            return False
        if "source_verified" in case:
            return case["source_verified"]
        return not any(item.get("status") == "SETUP_ERROR" for item in case["verified"])

    paths = {case["source"]["path"] for case in cases if case["at"] in models and has_verified_source(case)}
    files = {}
    archive_error = None
    if paths:
        try:
            files = corpus_oracles.read_sources(assets.source, source_root, paths)
        except (OSError, ValueError, TarError, BadZipFile, RuntimeError) as error:
            archive_error = str(error)
    for row, case in zip(methods, cases):
        model = models.get(row["at"])
        if model:
            source = case["source"]
            data = files.get(source["path"])
            raw = None
            detail = case.get("source_error") or case.get("original_error") or archive_error
            if data and has_verified_source(case):
                try:
                    lines = data.decode("utf-8").splitlines()
                    raw = "\n".join(lines[source["start_line"] - 1:source["end_line"]])
                except UnicodeError as error:
                    detail = str(error)
            checks = corpus_models.compare(model, raw, row["observation"], row["member"], timeout, source_detail=detail)
            controls = {}
            if case.get("original_verified", True):
                controls = corpus_models.raw_controls(model, assets.apk, at=row["at"], member=row["member"])
            row["checks"].extend(corpus_models.attach_raw_controls(checks, controls))
        elif row["at"] in model_errors:
            row["checks"].append(judge.check("model-setup", "modeled_behavior", "SETUP_ERROR",
                                             "valid authored execution model", None, model_errors[row["at"]]))
        row["status"] = judge.status(row["checks"])
