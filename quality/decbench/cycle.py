"""Hand one reviewed failure to a configured repair worker and verify the result."""

import json
from pathlib import Path
import subprocess
import tempfile

import candidates
from evidence import write_json
import judge
import reporting


def find_case(report: dict, case_id: str) -> tuple[dict, dict] | None:
    for item in report["inputs"]:
        for row in item["methods"]:
            if candidates.identity(item, row) == case_id:
                return item, row
    return None


def gate(before: dict, after: dict, case_id: str) -> tuple[dict, bool]:
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
    harmful = ("newly_failing", "coverage_lost", "coverage_changed")
    accepted = (
        bool(needed) and needed <= recovered
        and candidate[1]["status"] == "PASS" and after["inventory_complete"]
        and not any(comparison["summary"][key] for key in harmful)
        and not any(after["checks_by_status"].get(key) for key in ("UNCHECKED", "SETUP_ERROR"))
    )
    return comparison, accepted


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


def execute(args, database, baseline: dict) -> tuple[Path | None, int]:
    reporting.require_report(baseline, reporting.RUN_KIND)
    if (
        baseline.get("backend") != "diana" or not baseline.get("inventory_complete")
        or any(baseline.get("checks_by_status", {}).get(key) for key in ("UNCHECKED", "SETUP_ERROR"))
    ):
        raise ValueError("cycle requires a complete Diana report")
    available = judge.load_suites(judge.QUALITY / "fixtures")
    suites = baseline_suites(baseline, available)
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
        proposed = judge.run(binary, Path(baseline["prepared"]), suites, reply.get("revision"),
                             args.jobs, args.timeout, "diana", None)
        path = directory / "report.json"
        write_json(path, proposed)
        comparison, accepted = gate(baseline, proposed, case["id"])
        write_json(directory / "comparison.json", comparison)
        proof.update(note=reply.get("note"), report=str(path), transitions=comparison["summary"])
        if accepted:
            candidates.enqueue(database, proposed, str(path.relative_to(args.state)))
            proof["status"] = "accepted"
            return directory, 0
        proof["status"] = "rejected"
        return directory, 1
    except (OSError, ValueError, subprocess.SubprocessError) as error:
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
