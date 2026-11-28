"""Hand one reviewed failure to a configured repair worker and verify the result."""

from functools import partial
import json
from pathlib import Path
import subprocess
import tempfile

import candidates
from evidence import file_digest, write_json
import judge
import reporting


EVALUATION_ERRORS = (OSError, ValueError, subprocess.SubprocessError)
HARMFUL_TRANSITIONS = ("newly_failing", "coverage_lost", "coverage_changed")


def find_case(report: dict, case_id: str) -> tuple[dict, dict] | None:
    for item in report["inputs"]:
        for row in item["methods"]:
            if candidates.identity(item, row) == case_id:
                return item, row
    return None


def gate(before: dict, after: dict, case_id: str, *, public: bool = False) -> tuple[dict, bool]:
    comparison = reporting.compare_reports(before, after)
    original = find_case(before, case_id)
    candidate = find_case(after, case_id)
    if original is None or candidate is None:
        return comparison, False
    item, row = original
    needed = {(check["metric"], check["id"]) for check in row["checks"]
              if check["status"] == "FAIL"}
    recovered = {
        (check["metric"], check["id"])
        for method in comparison["methods"]
        if (method["suite"], method["input_sha256"], method["oracle_sha256"],
            method["at"], method["member"]) ==
           (item["suite"], item["sha256"], item["oracle_sha256"], row["at"], row["member"])
        for check in method["checks"] if check["transition"] == "recovered"
    }
    accepted = (
        bool(needed) and needed <= recovered
        and candidate[1]["status"] == "PASS" and preserves_scope(after, comparison, public=public)
    )
    return comparison, accepted


def incomplete(report, *, public):
    unavailable = ("SETUP_ERROR",) if public else ("SETUP_ERROR", "UNCHECKED")
    return (not report.get("inventory_complete")
            or any(report.get("checks_by_status", {}).get(key) for key in unavailable)
            or any(item.get("corpus", {}).get("source_structure", {}).get("SETUP_ERROR")
                   for item in report["inputs"]))


def preserves_scope(report, comparison, *, public):
    return comparison["evaluator_compatible"] and not incomplete(report, public=public) and not any(
        comparison["summary"][key] for key in HARMFUL_TRANSITIONS
    )


def baseline_suites(baseline: dict, available: dict) -> list[dict]:
    result = []
    for item in baseline["inputs"]:
        name = item["suite"]
        if name.startswith("external:"):
            suite = judge.external_suite(Path(item["path"]))
            if suite["id"] != name:
                raise ValueError(f"baseline input changed: {item['path']}")
        elif name in available:
            suite = available[name]
        else:
            raise ValueError(f"baseline suite is no longer available: {name}")
        locators = {row["at"] for row in item["methods"]}
        if judge.definitions(suite):
            result.append({**suite, "inventory_only": item.get("inventory_only", False)})
        else:
            if not locators:
                raise ValueError(f"baseline suite has no measured methods: {name}")
            result.append({**suite, "methods": [
                spec for spec in suite["methods"] if spec["at"] in locators
            ]})
    return result


def validate_worktree(path: Path) -> None:
    where = subprocess.run(
        ["git", "-C", str(path), "rev-parse", "--show-toplevel"],
        check=True, text=True, capture_output=True, timeout=10,
    )
    branch = subprocess.run(
        ["git", "-C", str(path), "rev-parse", "--abbrev-ref", "HEAD"],
        check=True, text=True, capture_output=True, timeout=10,
    )
    if Path(where.stdout.strip()).resolve() != path or branch.stdout.strip() in ("main", "master"):
        raise ValueError("the worker requires a dedicated non-main worktree")


def worker_reply(executable: Path, worktree: Path, brief: dict, timeout: int) -> dict:
    process = subprocess.run(
        [str(executable)], cwd=worktree, input=json.dumps(brief),
        text=True, encoding="utf-8", capture_output=True, check=False, timeout=timeout,
    )
    if process.returncode:
        raise ValueError(f"repair worker exited {process.returncode}: {process.stderr.strip()}")
    result = json.loads(process.stdout)
    if not isinstance(result, dict) or not isinstance(result.get("binary"), str):
        raise ValueError("repair worker must return an object with a candidate binary path")
    return result


def plan(baseline, jobs, timeout, *, report_path=None, public_prepared=None, case_path=None):
    reporting.require_report(baseline, reporting.RUN_KIND)
    inputs = baseline["inputs"]
    public = len(inputs) == 1 and inputs[0]["suite"].startswith("public:")
    if baseline.get("backend") != "diana" or not inputs or incomplete(baseline, public=public):
        raise ValueError("cycle requires a complete Diana report")
    prepared = Path(baseline["prepared"])
    if public:
        import corpus_assets
        import corpus_eval

        project = corpus_assets.project_by_id(inputs[0]["suite"].split(":", 1)[1])
        selected = [row["at"] for row in inputs[0]["methods"]]
        corpus = inputs[0].get("corpus", {})
        if case_path is None and corpus.get("external_cases"):
            bundled = report_path.parent / "cases.json" if report_path is not None else None
            case_path = bundled if bundled and bundled.is_file() else Path(corpus["cases"])
        prepared = public_prepared or prepared
        assets = corpus_assets.prepare_assets(project, prepared)
        if (file_digest(assets.apk) != inputs[0]["sha256"]
                or file_digest(assets.source) != corpus.get("source_archive_sha256")):
            raise ValueError("the prepared public assets differ from the saved baseline")
        _, configuration = corpus_eval.case_configuration(project, case_path)
        identities = {method["at"]: method["member"] for method in configuration["methods"]}
        if any(identities.get(row["at"]) != row["member"] for row in inputs[0]["methods"]):
            raise ValueError("the case manifest differs from the saved original methods")
        inspect_structure = corpus.get("inspect_structure", False)
        if baseline.get("evaluator_sha256") != corpus_eval.evaluator_fingerprint("diana", inspect_structure):
            raise ValueError("the saved baseline was measured by a different evaluator")
        evaluate = partial(corpus_eval.run, project, prepared, backend="diana", selected=selected, jobs=jobs,
                           timeout=timeout, execute_models=corpus.get("execute_models", False),
                           case_path=case_path, inspect_structure=inspect_structure)
        return public, evaluate
    if case_path is not None:
        raise ValueError("--cases requires a public corpus baseline")
    if baseline.get("evaluator_sha256") != judge.evaluator_fingerprint():
        raise ValueError("the saved baseline was measured by a different evaluator")
    available = judge.load_suites(judge.QUALITY / "fixtures")
    suites = baseline_suites(baseline, available)
    evaluate = partial(judge.run, prepared=prepared, suites=suites, jobs=jobs, timeout=timeout,
                       backend="diana", checkpoint=None)
    return public, evaluate


def guard_plans(baseline, guards, jobs, timeout, public_prepared=None):
    if not guards:
        return []
    baseline_hash = baseline.get("binary", {}).get("sha256")
    plans = []
    for path, guard in guards:
        if not baseline_hash or baseline_hash != guard.get("binary", {}).get("sha256"):
            raise ValueError(f"{path}: the guard and primary baseline must measure the same Diana executable")
        try:
            public, evaluate = plan(guard, jobs, timeout, report_path=path, public_prepared=public_prepared)
        except EVALUATION_ERRORS as error:
            raise ValueError(f"{path}: {error}") from error
        plans.append((path, guard, public, evaluate))
    return plans


def evaluate_guards(plans, directory, binary, revision, binary_hash):
    results = []
    for index, (source, baseline, public, evaluate) in enumerate(plans, 1):
        receipt = {"baseline": str(source)}
        scope = directory / f"guard-{index:02}"
        try:
            report = evaluate(binary=binary, revision=revision)
            if report["binary"]["sha256"] != binary_hash:
                raise ValueError("the candidate executable changed during evaluation")
            comparison = reporting.compare_reports(baseline, report)
            write_json(scope / "report.json", report)
            write_json(scope / "comparison.json", comparison)
        except EVALUATION_ERRORS as error:
            receipt.update(status="incomplete", error=str(error))
        else:
            status = "passed"
            if incomplete(report, public=public) or not comparison["evaluator_compatible"]:
                status = "incomplete"
            elif not preserves_scope(report, comparison, public=public):
                status = "rejected"
            receipt.update(status=status, report=str(scope / "report.json"),
                           comparison=str(scope / "comparison.json"), transitions=comparison["summary"])
        results.append(receipt)
    return results


def execute(args, database, baseline: dict, guards=()) -> tuple[Path | None, int]:
    public_prepared = getattr(args, "prepared", None)
    public, evaluate = plan(baseline, args.jobs, args.timeout, report_path=args.baseline,
                            public_prepared=public_prepared, case_path=getattr(args, "cases", None))
    plans = guard_plans(baseline, guards, args.jobs, args.timeout, public_prepared)
    validate_worktree(args.worktree)
    eligible = {
        candidates.identity(item, row)
        for item in baseline["inputs"] for row in item["methods"] if row["status"] == "FAIL"
    }
    case = candidates.claim(database, args.worker_timeout + 7200, args.group, eligible)
    if case is None:
        return None, 0
    attempts = args.state / "attempts"
    attempts.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(dir=attempts, prefix="attempt-"))
    proof = {"case": case["id"], "baseline": str(args.baseline), "status": "incomplete"}
    try:
        original = find_case(baseline, case["id"])
        if original is None or original[1]["status"] != "FAIL":
            raise ValueError("the leased failure does not match the selected baseline report")
        brief = candidates.show(args.state, case)
        brief.pop("claim_token", None)
        reply = worker_reply(args.worker, args.worktree, {
            "case": brief, "baseline": str(args.baseline), "worktree": str(args.worktree),
        }, args.worker_timeout)
        binary = (args.worktree / reply["binary"]).resolve()
        revision = reply.get("revision")
        binary_hash = file_digest(binary)
        proposed = evaluate(binary=binary, revision=revision)
        path = directory / "report.json"
        write_json(path, proposed)
        comparison, accepted = gate(baseline, proposed, case["id"], public=public)
        write_json(directory / "comparison.json", comparison)
        proof.update(note=reply.get("note"), report=str(path), transitions=comparison["summary"])
        if proposed["binary"]["sha256"] != binary_hash:
            raise ValueError("the candidate executable changed during evaluation")
        if plans:
            results = evaluate_guards(plans, directory, binary, revision, binary_hash)
            proof["guards"] = results
            if any(result["status"] == "incomplete" for result in results):
                return directory, 2
            accepted = accepted and all(result["status"] == "passed" for result in results)
        if file_digest(binary) != binary_hash:
            raise ValueError("the candidate executable changed during evaluation")
        if incomplete(proposed, public=public) or not comparison["evaluator_compatible"]:
            return directory, 2
        if accepted:
            candidates.enqueue(database, proposed, str(path.relative_to(args.state)))
            proof["status"] = "accepted"
            return directory, 0
        proof["status"] = "rejected"
        return directory, 1
    except EVALUATION_ERRORS as error:
        proof["error"] = str(error)
        return directory, 2
    finally:
        write_json(directory / "attempt.json", proof)
        if proof["status"] != "accepted":
            try:
                candidates.resolve(database, case["id"], case["claim_token"], "pending",
                                   f"Last attempt {proof['status']}; see {directory}", None)
            except ValueError:
                # A newer evaluation already superseded this lease.
                pass
