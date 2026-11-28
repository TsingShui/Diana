"""Run the Diana DecBench corpus, compare reports, and manage repair candidates."""

import argparse
from contextlib import closing
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile

import candidates
import cycle
from evidence import write_json
import judge
import reporting

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_STATE = ROOT / ".cache" / "decbench"
JVM_FIXTURE = ROOT / "quality" / "fixtures" / "jvm" / "source-recovery"


def absolute(value: str) -> Path:
    return Path(value).expanduser().resolve()


def positive(value: str) -> int:
    number = int(value)
    if number < 1:
        raise argparse.ArgumentTypeError("expected a positive integer")
    return number


def parse_args():
    parser = argparse.ArgumentParser(prog="diana-decbench", description=__doc__)
    parser.add_argument("--state", type=absolute, default=DEFAULT_STATE)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("suites", help="list authored suites and their methods")
    prepare = commands.add_parser("prepare", help="prepare the authored APK and JVM corpus")
    prepare.add_argument("--prepared", type=absolute)
    run = commands.add_parser("run", help="judge one Diana executable")
    run.add_argument("--binary", required=True, type=absolute)
    run.add_argument("--prepared", type=absolute)
    run.add_argument("--suite", action="append")
    run.add_argument("--inventory-only", action="store_true", help="check definitions only, without fetching publications")
    run.add_argument("--input", action="append", type=absolute, help="add a definition suite for APK, DEX, class or flat JAR")
    run.add_argument("--revision", help="source revision for this executable")
    run.add_argument("--backend", default="diana", help="name of the CLI-compatible backend")
    run.add_argument("--declared-only", action="store_true", help="run only authored suite manifests")
    run.add_argument("--at", action="append", help="select an original locator from one suite")
    run.add_argument("--resume", action="store_true", help="reuse completed per-input checkpoints")
    run.add_argument("--jobs", default=4, type=positive)
    run.add_argument("--timeout", default=45, type=positive)
    run.add_argument("--output", type=absolute)
    diff = commands.add_parser("compare", help="compare two completed evaluation reports")
    diff.add_argument("--baseline", required=True, type=absolute)
    diff.add_argument("--candidate", required=True, type=absolute)
    diff.add_argument("--output", type=absolute)
    diff.add_argument("--gate", action="store_true", help="fail on regressions or lost evidence")
    index = commands.add_parser("index", help="build a dashboard of ordered evaluation reports")
    index.add_argument("inputs", type=absolute, nargs="+")
    index.add_argument("--output", type=absolute, required=True)
    mining = commands.add_parser("mine", help="mine regressions, peer gaps and shared failures")
    mining.add_argument("inputs", type=absolute, nargs="+")
    mining.add_argument("--output", type=absolute)
    mining.add_argument("--metric", help="select findings for one evidence metric")
    view = commands.add_parser("report", help="render an evaluation or comparison as standalone HTML")
    view.add_argument("input", type=absolute)
    view.add_argument("--output", required=True, type=absolute)
    automate = commands.add_parser("cycle", help="run a repair worker and verify the proposed build")
    automate.add_argument("--baseline", type=absolute, required=True)
    automate.add_argument("--worker", type=absolute, required=True)
    automate.add_argument("--worktree", type=absolute, required=True)
    automate.add_argument("--group")
    automate.add_argument("--worker-timeout", default=1800, type=positive)
    automate.add_argument("--timeout", default=45, type=positive)
    automate.add_argument("--jobs", default=4, type=positive)
    listing = commands.add_parser("queue", help="list independently failing method candidates")
    listing.add_argument("--group")
    listing.add_argument("--metric", help="select cases with a failure in this metric")
    listing.add_argument("--status", choices=("pending", "claimed", "ignored", "cleared", "superseded"))
    details = commands.add_parser("show", help="print reproduction evidence for one case")
    details.add_argument("id")
    claim = commands.add_parser("claim", help="lease the next pending case")
    claim.add_argument("--group")
    claim.add_argument("--metric", help="lease a case with a failure in this metric")
    claim.add_argument("--lease-seconds", default=3600, type=positive)
    resolve = commands.add_parser("resolve", help="record triage or return a claim")
    resolve.add_argument("id")
    resolve.add_argument("--claim-token", required=True)
    resolve.add_argument("--status", choices=("pending", "ignored"), required=True)
    resolve.add_argument("--note", required=True)
    resolve.add_argument("--group", help="reviewed owning mechanism for this case")
    return parser.parse_args()


def prepare_corpus(destination: Path) -> None:
    if destination.exists():
        raise ValueError(f"{destination} already exists; select a new prepared corpus directory")
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=destination.parent, prefix=".prepare-") as temp:
        staging = Path(temp) / "jvm"
        for variant in ("debug", "stripped"):
            subprocess.run([
                sys.executable, str(JVM_FIXTURE / "prepare.py"), str(staging / variant),
                "--variant", variant,
            ], check=True, timeout=120)
        apk_recipe = ROOT / "quality/fixtures/export/multidex-resources/prepare.py"
        subprocess.run([
            sys.executable, str(apk_recipe), str(staging / "multidex.apk"), "complete",
        ], check=True, timeout=120)
        if destination.exists():
            raise ValueError(f"another process already prepared {destination}")
        staging.rename(destination)
    print(destination)


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def evaluate(args, prepared: Path) -> int:
    suites = judge.load_suites(judge.QUALITY / "fixtures")
    external = [judge.external_suite(path) for path in sorted(set(args.input or []))]
    for item in external:
        suites[item["id"]] = item
    if args.suite or external:
        selected = sorted(set(args.suite or ()) | {item["id"] for item in external})
    else:
        selected = [name for name, suite in suites.items() if suite.get("default", True)]
    unknown = set(selected) - suites.keys()
    if unknown:
        raise ValueError(f"unknown suites: {', '.join(sorted(unknown))}")
    chosen = [suites[name] for name in selected if not args.declared_only or not suites[name].get("discovered")]
    if not chosen:
        raise ValueError("no selected suites")
    if args.inventory_only:
        if any(not judge.definitions(entry) for entry in chosen):
            raise ValueError("--inventory-only requires definition suites")
        chosen = [{**entry, "inventory_only": True} for entry in chosen]
    if args.at:
        if len(chosen) != 1 or judge.definitions(chosen[0]):
            raise ValueError("--at requires exactly one authored method suite")
        entries = [entry for entry in chosen[0]["methods"] if entry["at"] in args.at]
        if set(args.at) != {entry["at"] for entry in entries}:
            raise ValueError("one or more selected locators are not in the suite")
        chosen = [{**chosen[0], "methods": entries}]
    result = judge.run(
        args.binary, prepared, chosen, args.revision, args.jobs, args.timeout, args.backend,
        args.state / "checkpoints" if args.resume else None,
    )
    runs = args.state / "judgements"
    runs.mkdir(parents=True, exist_ok=True)
    path = Path(tempfile.mkdtemp(prefix="run-", dir=runs)) / "report.json"
    write_json(path, result)
    with closing(candidates.connect(args.state)) as db:
        if args.backend == "diana":
            candidates.enqueue(db, result, str(path.relative_to(args.state)))
    if args.output:
        write_json(args.output, result)
    print(args.output or path)
    counts = result["checks_by_status"]
    if not result["inventory_complete"] or counts.get("SETUP_ERROR") or counts.get("UNCHECKED"):
        return 2
    return 1 if counts.get("FAIL") else 0


def cases_for_metric(cases: list[dict], metric: str | None) -> list[dict]:
    if metric is None:
        return cases
    return [case for case in cases if any(check["metric"] == metric for check in case["failures"])]


def main() -> int:
    args = parse_args()
    try:
        if args.command == "suites":
            suites = judge.load_suites(judge.QUALITY / "fixtures")
            print(json.dumps([
                {**{key: suite[key] for key in ("id", "input", "methods")},
                 "default": suite.get("default", True)} for suite in suites.values()
            ], indent=2))
            return 0
        if args.command in ("prepare", "run"):
            prepared = args.prepared or args.state / "corpus"
            if args.command == "run":
                return evaluate(args, prepared)
            prepare_corpus(prepared)
            return 0
        if args.command == "compare":
            diff = reporting.compare_reports(read_json(args.baseline), read_json(args.candidate))
            if args.output:
                write_json(args.output, diff)
                print(args.output)
            else:
                print(json.dumps(diff, indent=2))
            return int(args.gate and any(diff["summary"][key] for key in ("newly_failing", "coverage_lost", "coverage_changed")))
        if args.command in ("index", "mine"):
            reports = [read_json(path) for path in args.inputs]
            if args.command == "mine":
                findings = reporting.mine_reports(reports)
                if args.metric:
                    findings = [finding for finding in findings if finding["metric"] == args.metric]
                if args.output:
                    write_json(args.output, findings)
                    print(args.output)
                else:
                    print(json.dumps(findings, indent=2))
                return 0
            output = reporting.render_index(reporting.build_index(reports))
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(output, encoding="utf-8")
            print(args.output)
            return 0
        if args.command == "report":
            document = reporting.render_report(read_json(args.input))
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(document, encoding="utf-8")
            print(args.output)
            return 0
        with closing(candidates.connect(args.state)) as db:
            if args.command == "cycle":
                attempt, result = cycle.execute(args, db, read_json(args.baseline))
                if attempt:
                    print(attempt)
                return result
            if args.command == "resolve":
                candidates.resolve(db, args.id, args.claim_token, args.status, args.note, args.group)
                return 0
            if args.command == "show":
                result = candidates.show(args.state, candidates.read_case(db, args.id))
                report = read_json(args.state / result["report"])
                result["reproduce"] = [
                    sys.executable, str(ROOT / "quality" / "decbench"),
                    "--state", str(args.state), "run", "--binary", result["binary"]["path"],
                    "--prepared", report["prepared"],
                ]
                if result.get("inventory_only"):
                    result["reproduce"].append("--inventory-only")
                if result["suite"].startswith("external:"):
                    result["reproduce"].extend(("--input", result["input"]))
                else:
                    result["reproduce"].extend(("--suite", result["suite"]))
                    if not result["suite"].startswith(("definitions:", "apk:", "jvm-inventory:")):
                        result["reproduce"].extend(("--at", result["at"]))
            elif args.command == "claim":
                allowed = None
                if args.metric:
                    allowed = {case["id"] for case in cases_for_metric(
                        candidates.cases(db, "pending", args.group), args.metric,
                    )}
                result = candidates.claim(db, args.lease_seconds, args.group, allowed)
            else:
                result = cases_for_metric(candidates.cases(db, args.status, args.group), args.metric)
            print(json.dumps(result, indent=2))
    except (OSError, ValueError, sqlite3.Error, subprocess.SubprocessError) as error:
        print(f"diana-decbench: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
