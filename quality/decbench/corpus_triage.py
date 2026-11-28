"""Freshly assess one saved public method against its original evidence pins."""

from collections import Counter
from pathlib import Path
import subprocess
from tarfile import TarError
from zipfile import BadZipFile

import corpus_assets
from corpus_comparison import MAX_DOCUMENT_BYTES, bundle_file, read_document
import corpus_eval
from dex_calls import LOCATOR
import reporting


KIND = "diana-decbench-corpus-triage"
SETUP_ERRORS = (OSError, ValueError, TypeError, KeyError, subprocess.SubprocessError, TarError, BadZipFile)
OUTCOMES = ("SETUP_ERROR", "EVIDENCE_CHANGED", "NEW_FAILURE", "REPRODUCED",
            "OBSERVED_FAILURE", "LOCAL_UNCERTAINTY", "RESOLVED", "PASS")


def selection(report, at, backend):
    methods, _ = reporting.index_methods(report)
    if report.get("backend") != backend or len(report["inputs"]) != 1:
        raise ValueError(f"expected a single public corpus report from {backend}")
    matching = [key for key in methods if key[3] == at]
    if len(matching) != 1:
        raise ValueError(f"the report must contain exactly one original method at {at}")
    key = matching[0]
    item = report["inputs"][0]
    corpus = item.get("corpus", {})
    if not isinstance(corpus, dict):
        raise ValueError("the report lacks exact public corpus provenance")
    identifier = reporting.string(corpus, "id")
    source_revision = reporting.string(corpus, "source_revision")
    source_archive = reporting.string(corpus, "source_archive_sha256")
    if (key[0] != f"public:{identifier}" or not corpus_assets.PROJECT_ID.fullmatch(identifier)
            or not corpus_assets.COMMIT.fullmatch(source_revision)
            or any(not corpus_assets.SHA256.fullmatch(pin) for pin in (*key[1:3], source_archive))):
        raise ValueError("the report lacks exact public APK, oracle and source pins")
    return key, item, methods[key][1]


def publication(report, item, method):
    return {**reporting.metadata(report), "evaluator_sha256": report.get("evaluator_sha256"),
            "inventory": item.get("inventory"), "status": method.get("status"),
            "observation": method.get("observation"), "source_structure": method.get("source_structure"),
            "provenance": method.get("provenance")}


def check_outcome(delta):
    previous, current = delta["baseline"], delta["candidate"]
    if current is None or (previous is not None and
            reporting.json_identity(previous["expected"]) != reporting.json_identity(current["expected"])):
        return "EVIDENCE_CHANGED"
    if current["status"] == "SETUP_ERROR":
        return "SETUP_ERROR"
    if current["status"] == "UNCHECKED":
        return "LOCAL_UNCERTAINTY"
    if previous is None or previous["status"] in ("UNCHECKED", "SETUP_ERROR"):
        return "OBSERVED_FAILURE" if current["status"] == "FAIL" else "COVERAGE_ADDED"
    if current["status"] == "FAIL":
        return "REPRODUCED" if previous["status"] == "FAIL" else "NEW_FAILURE"
    return "RESOLVED" if previous["status"] == "FAIL" else "PASS"


def current_context(report, item, method):
    inventory = item.get("inventory", {})
    structure = method.get("source_structure") or {}
    if not isinstance(inventory, dict) or not isinstance(structure, dict):
        raise ValueError("the fresh report has invalid capture metadata")
    if inventory.get("setup_error") or structure.get("outcome") == "SETUP_ERROR":
        return "SETUP_ERROR"
    if report.get("inventory_complete") is not True or not reporting.index_checks(method):
        return "LOCAL_UNCERTAINTY"
    return "PASS"


def triage(report_path: Path, at: str, binary: Path, *, cache: Path | None = None,
           case_path: Path | None = None, jadx_jar: Path | None = None,
           timeout: int = 60, revision: str | None = None) -> dict:
    """Check saved pins before invoking either producer, then compare the one selected method."""
    receipt = {"kind": KIND, "schema_version": 1, "baseline_report": str(report_path.absolute()),
               "at": at, "requested": {"binary": str(binary.absolute()), "timeout": timeout, "revision": revision},
               "checks": [], "summary": {}}
    if jadx_jar is not None:
        receipt["requested"]["jadx_jar"] = str(jadx_jar.absolute())
    try:
        if not LOCATOR.fullmatch(at):
            raise ValueError("triage requires one exact physical method locator")
        if type(timeout) is not int or timeout <= 0:
            raise ValueError("triage timeout must be a positive number of seconds")
        saved = read_document(report_path, [MAX_DOCUMENT_BYTES])
        key, original, before = selection(saved, at, "diana")
        corpus = original["corpus"]
        pins = {"input_sha256": key[1], "oracle_sha256": key[2], "member": key[4],
                "source_archive_sha256": corpus["source_archive_sha256"]}
        receipt.update(corpus=corpus["id"], member=key[4], source=before.get("source"), pins=pins,
                       baseline=publication(saved, original, before))
        profile = corpus.get("profile")
        flags = {name: corpus.get(name) for name in ("external_cases", "execute_models", "inspect_structure")}
        if (profile not in ("reviewed", "discovered", "modeled")
                or any(type(flag) is not bool for flag in flags.values())
                or flags["execute_models"] != (profile == "modeled")):
            raise ValueError("the saved report does not define an exact oracle configuration")
        project = corpus_assets.project_by_id(corpus["id"])
        if project.source_commit != corpus["source_revision"]:
            raise corpus_eval.EvidenceChanged("the catalog source revision differs from the saved run")
        saved_judge = saved.get("evaluator_sha256")
        exported = (original.get("inventory") or {}).get("capture") == "export"
        if saved_judge is not None and saved_judge != corpus_eval.evaluator_fingerprint(
                "diana", flags["inspect_structure"], export=exported):
            raise corpus_eval.EvidenceChanged("the current evaluator differs from the saved run")
        if case_path is None and flags["external_cases"]:
            try:
                case_path = bundle_file(report_path.parent.resolve(), "cases.json")
            except FileNotFoundError:
                raise ValueError("supply --cases with the exact original custom or discovered manifest") from None
        cache = cache if cache is not None else Path(reporting.string(saved, "prepared"))
        cache = cache.resolve()
        receipt["requested"]["prepared"] = str(cache)
        receipt["oracle"] = {"profile": profile, "source_revision": project.source_commit,
                             "cases": str((case_path or project.manifest.with_name(f"{project.id}.cases.json")).absolute()),
                             **flags}

        def recapture(producer, backend, current_revision=None):
            assessment = corpus_eval.run(
                project, cache, producer.resolve(), current_revision, backend, [at], 1, timeout,
                execute_models=flags["execute_models"], case_path=case_path,
                inspect_structure=flags["inspect_structure"], expected_evidence=pins,
            )
            current_key, current_item, method = selection(assessment, at, backend)
            current_corpus = current_item["corpus"]
            if (current_key != key or len(current_item["methods"]) != 1
                    or current_corpus["source_archive_sha256"] != pins["source_archive_sha256"]
                    or current_corpus["source_revision"] != project.source_commit
                    or current_corpus.get("profile") != profile
                    or any(current_corpus.get(name) is not flags[name]
                           for name in ("execute_models", "inspect_structure"))):
                raise corpus_eval.EvidenceChanged("the fresh report does not confirm the saved original evidence")
            return assessment, current_item, method

        fresh, observed, after = recapture(binary, "diana", revision)
        if not exported and not reporting.evaluator_compatible(saved, fresh):
            raise corpus_eval.EvidenceChanged("the fresh run did not use the saved evaluator")
        receipt.update(current=publication(fresh, observed, after), source=after.get("source"))
        baseline = {**saved, "inputs": [{**original, "methods": [before]}]}
        delta = reporting.compare_reports(baseline, fresh)["methods"][0]
        checks = [{**check, "outcome": check_outcome(check)} for check in delta["checks"]]
        outcomes = Counter(check["outcome"] for check in checks)
        context = current_context(fresh, observed, after)
        status = next((value for value in OUTCOMES if outcomes[value] or value == context), "PASS")
        receipt.update(status=status, checks=checks, summary=dict(sorted(outcomes.items())), capture_status=context,
                       publication_changed=delta["publication_changed"])
    except corpus_eval.EvidenceChanged as error:
        receipt.update(status="EVIDENCE_CHANGED", detail=str(error))
        return receipt
    except SETUP_ERRORS as error:
        receipt.update(status="SETUP_ERROR", detail=str(error))
        return receipt

    if jadx_jar is not None:
        try:
            peer, peer_input, peer_method = recapture(jadx_jar, "jadx")
            context = current_context(peer, peer_input, peer_method)
            peer_checks = peer_method["checks"]
            if any(check["status"] == "SETUP_ERROR" for check in peer_checks):
                context = "SETUP_ERROR"
            elif context == "PASS" and any(check["status"] == "UNCHECKED" for check in peer_checks):
                context = "LOCAL_UNCERTAINTY"
            receipt["peer"] = {**publication(peer, peer_input, peer_method), "capture_status": context,
                               "checks": peer_checks}
        except corpus_eval.EvidenceChanged as error:
            receipt["peer"] = {"capture_status": "EVIDENCE_CHANGED", "detail": str(error)}
        except SETUP_ERRORS as error:
            receipt["peer"] = {"capture_status": "SETUP_ERROR", "detail": str(error)}
    return receipt


def result_code(receipt: dict) -> int:
    incomplete = ("SETUP_ERROR", "EVIDENCE_CHANGED", "LOCAL_UNCERTAINTY")
    if receipt["status"] in incomplete or receipt.get("peer", {}).get("capture_status") in incomplete:
        return 2
    return int(receipt["status"] in ("REPRODUCED", "NEW_FAILURE", "OBSERVED_FAILURE"))


def render(receipt: dict) -> str:
    reporting.require_report(receipt, KIND)
    title = f"{receipt.get('member', '')} · {receipt['at']} · {receipt['status']}"
    summary = {key: receipt[key] for key in ("summary", "pins", "requested", "detail") if key in receipt}
    body = [f"<h2>{reporting.label(title)}</h2><pre>{reporting.format_data(summary)}</pre>",
            reporting.render_source(receipt.get("source"))]
    if receipt["checks"]:
        checks = [{**row, "transition": row["outcome"]} for row in receipt["checks"]]
        body.append(reporting.render_check_rows(checks, True))
    for name, key in (("Saved Diana", "baseline"), ("Current Diana", "current"), ("JADX", "peer")):
        if arm := receipt.get(key):
            capture = {field: value for field, value in arm.items() if field not in ("observation", "checks", "source_structure")}
            body.extend((f"<details><summary>{name}</summary><pre>{reporting.format_data(capture)}</pre>",
                         reporting.render_observation(arm.get("observation")),
                         reporting.render_source_structure(arm.get("source_structure"))))
            if arm.get("checks"):
                body.append(reporting.render_check_rows(arm["checks"], False))
            body.append("</details>")
    return reporting.html_page("Diana DecBench triage", "".join(body))
