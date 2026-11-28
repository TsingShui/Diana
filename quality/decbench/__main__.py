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


def shard_number(value: str) -> tuple[int, int]:
    fields = value.split("/")
    if len(fields) != 2 or not all(field.isascii() and field.isdecimal() for field in fields):
        raise argparse.ArgumentTypeError("expected a 1-based shard such as 2/4")
    number, count = map(int, fields)
    if not 1 <= number <= count:
        raise argparse.ArgumentTypeError("expected a shard number in its partition range")
    return number, count


def parse_args():
    parser = argparse.ArgumentParser(prog="diana-decbench", description=__doc__)
    parser.add_argument("--state", type=absolute, default=DEFAULT_STATE)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("suites", help="list authored suites and their methods")
    commands.add_parser("corpora", help="list public APK corpora available for opt-in evaluation")
    prepare = commands.add_parser("prepare", help="prepare fixtures or one opt-in public corpus")
    prepare.add_argument("--prepared", type=absolute)
    prepare.add_argument("--corpus", help="public corpus ID")
    prepare.add_argument("--download", action="store_true", help="allow fetching the selected public corpus")
    prepare.add_argument("--apk", type=absolute, help="import a local copy of the public APK")
    prepare.add_argument("--source", type=absolute, help="import a local copy of the pinned source archive")
    discover = commands.add_parser("discover", help="select source-aligned methods from a cached public corpus")
    discover.add_argument("--corpus", required=True)
    discover.add_argument("--prepared", type=absolute)
    discover.add_argument("--limit", type=positive, default=100)
    discover.add_argument("--shard", type=shard_number, default=(1, 1), help="select one deterministic partition, such as 2/4")
    discover.add_argument("--output", type=absolute)
    coverage = commands.add_parser("coverage", help="summarize original, selected, and actually checked public methods")
    coverage.add_argument("--corpus", required=True)
    coverage.add_argument("--prepared", type=absolute)
    coverage.add_argument("--cases", action="append", type=absolute, help="use a generated selection manifest")
    coverage.add_argument("--report", action="append", type=absolute, default=[], help="include receipts from one run")
    coverage.add_argument("--output", type=absolute)
    campaign = commands.add_parser("campaign", help="evaluate cached public corpora and collect one evidence bundle")
    campaign.add_argument("--corpus", action="append", help="select a public corpus ID, defaulting to all")
    campaign.add_argument("--prepared", type=absolute)
    campaign.add_argument("--binary", type=absolute, required=True)
    campaign.add_argument("--jadx-jar", type=absolute, help="evaluate the same methods with official JADX")
    campaign.add_argument("--revision", help="source revision for the Diana executable")
    campaign.add_argument("--profile", choices=("reviewed", "sample", "full"), default="reviewed",
                          help="full exports each APK once and evaluates all source-aligned selections")
    campaign.add_argument("--limit", type=positive, help="limit the selected methods per shard")
    campaign.add_argument("--shards", type=positive, default=1)
    campaign.add_argument("--execute-models", action="store_true")
    campaign.add_argument("--inspect-structure", action="store_true")
    campaign.add_argument("--resume", action="store_true")
    campaign.add_argument("--jobs", type=positive, default=4)
    campaign.add_argument("--timeout", type=positive, default=45)
    campaign.add_argument("--output", type=absolute, help="parent directory for the new campaign bundle")
    bench = commands.add_parser("bench", help="build a human-readable source diff from a saved public campaign")
    bench.add_argument("--campaign", type=absolute, help="campaign directory or campaign.json")
    bench.add_argument("--run", type=absolute, action="append", default=[], help="include a saved public run report")
    bench.add_argument("--prepared", type=absolute, help="override the cached corpus directory")
    bench.add_argument("--output", required=True, type=absolute, help="destination HTML file")
    run = commands.add_parser("run", help="judge one Diana executable")
    producer = run.add_mutually_exclusive_group(required=True)
    producer.add_argument("--binary", type=absolute)
    producer.add_argument("--jadx-jar", type=absolute, help="official JADX JAR for an opt-in public corpus")
    run.add_argument("--corpus", help="evaluate reviewed methods of this cached public corpus")
    run.add_argument("--cases", type=absolute, help="evaluate a generated public case manifest")
    run.add_argument("--execute-models", action="store_true", help="run authored Java models for selected public methods")
    run.add_argument("--inspect-structure", action="store_true", help="compare source and recovered CFGs for review")
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
    variation = commands.add_parser("compare-campaign", help="compare two public bundles by corpus, producer, and shard")
    variation.add_argument("--baseline", required=True, type=absolute, help="baseline bundle or campaign.json")
    variation.add_argument("--candidate", required=True, type=absolute, help="candidate bundle or campaign.json")
    variation.add_argument("--output", type=absolute)
    variation.add_argument("--gate", action="store_true", help="fail on regressions, lost, or incompatible evidence")
    index = commands.add_parser("index", help="build a dashboard of ordered evaluation reports")
    index.add_argument("inputs", type=absolute, nargs="+")
    index.add_argument("--output", type=absolute, required=True)
    mining = commands.add_parser("mine", help="mine regressions, peer gaps and shared failures")
    mining.add_argument("inputs", type=absolute, nargs="+")
    mining.add_argument("--output", type=absolute)
    mining.add_argument("--metric", help="select findings for one evidence metric")
    review = commands.add_parser("review", help="collect source-aligned differences for qualitative review")
    review.add_argument("inputs", type=absolute, nargs="+")
    review.add_argument("--target", default="diana")
    review.add_argument("--peer", default="jadx")
    review.add_argument("--output", type=absolute)
    triage = commands.add_parser("triage", help="replay one public candidate on a current executable")
    triage.add_argument("--baseline", required=True, type=absolute, help="saved Diana run report")
    triage.add_argument("--at", required=True, help="one exact original DEX method locator")
    triage.add_argument("--binary", required=True, type=absolute, help="explicit current Diana executable")
    triage.add_argument("--prepared", type=absolute)
    triage.add_argument("--cases", type=absolute, help="original custom cases if the selection was moved")
    triage.add_argument("--jadx-jar", type=absolute)
    triage.add_argument("--revision")
    triage.add_argument("--timeout", type=positive, default=60)
    triage.add_argument("--output", type=absolute)
    timing = commands.add_parser("timing", help="measure two Diana builds on the same cached public methods")
    timing.add_argument("--corpus", required=True)
    timing.add_argument("--prepared", type=absolute)
    timing.add_argument("--cases", type=absolute)
    timing.add_argument("--at", action="append")
    timing.add_argument("--baseline", type=absolute, required=True)
    timing.add_argument("--candidate", type=absolute, required=True)
    timing.add_argument("--repeats", type=positive, default=5)
    timing.add_argument("--timeout", type=positive, default=45)
    timing.add_argument("--time-budget", type=positive, default=300)
    timing.add_argument("--output", type=absolute)
    view = commands.add_parser("report", help="render an evaluation or comparison as standalone HTML")
    view.add_argument("input", type=absolute)
    view.add_argument("--output", required=True, type=absolute)
    automate = commands.add_parser("cycle", help="run a repair worker and verify the proposed build")
    automate.add_argument("--baseline", type=absolute, required=True)
    automate.add_argument("--guard-baseline", type=absolute, action="append", default=[],
                          help="require the proposed build to preserve an additional Diana evaluation")
    automate.add_argument("--prepared", type=absolute, help="override the prepared assets for public baselines")
    automate.add_argument("--cases", type=absolute, help="override the primary public case manifest")
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
        for fixture in ("string-switch-receiver", "interface-initializer-snapshot"):
            recipe = ROOT / "quality/fixtures/jvm" / fixture / "verify.py"
            subprocess.run([
                sys.executable, str(recipe), "--prepare", str(staging / fixture),
            ], check=True, timeout=120, stdout=subprocess.DEVNULL)
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
    if args.corpus:
        import corpus_cli

        return save_evaluation(args, corpus_cli.judge(args, prepared))
    if args.jadx_jar:
        raise ValueError("--jadx-jar requires --corpus")
    if args.execute_models or args.cases or args.inspect_structure:
        raise ValueError("--cases, --execute-models and --inspect-structure require --corpus")

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
    return save_evaluation(args, result)


def save_evaluation(args, result: dict) -> int:
    path = candidates.record_run(args.state, result)
    if args.output:
        write_json(args.output, result)
    print(args.output or path)
    return judge.result_code(result)


def cases_for_metric(cases: list[dict], metric: str | None) -> list[dict]:
    if metric is None:
        return cases
    return [case for case in cases if any(check["metric"] == metric for check in case["failures"])]


def main() -> int:
    args = parse_args()
    try:
        if args.command == "bench":
            import corpus_bench

            if args.campaign is None and not args.run:
                raise ValueError("bench requires --campaign or --run")
            document = corpus_bench.from_evidence(args.campaign, args.run, args.prepared)
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(document, encoding="utf-8")
            print(args.output)
            return 0
        if args.command == "suites":
            suites = judge.load_suites(judge.QUALITY / "fixtures")
            print(json.dumps([
                {**{key: suite[key] for key in ("id", "input", "methods")},
                 "default": suite.get("default", True)} for suite in suites.values()
            ], indent=2))
            return 0
        if args.command in ("corpora", "discover", "coverage", "campaign", "timing"):
            import corpus_cli

            if args.command == "corpora":
                return corpus_cli.list_projects()
            if args.command == "discover":
                return corpus_cli.discover(args)
            if args.command == "coverage":
                return corpus_cli.coverage(args)
            if args.command == "campaign":
                return corpus_cli.campaign(args)
            return corpus_cli.time_builds(args)
        if args.command in ("prepare", "run"):
            prepared = args.prepared or args.state / ("public-corpora" if args.corpus else "corpus")
            if args.command == "run":
                return evaluate(args, prepared)
            if args.corpus:
                import corpus_cli

                return corpus_cli.prepare(args, prepared)
            if args.download or args.apk or args.source:
                raise ValueError("--download, --apk and --source require --corpus")
            prepare_corpus(prepared)
            return 0
        if args.command == "compare":
            diff = reporting.compare_reports(read_json(args.baseline), read_json(args.candidate))
            if args.output:
                write_json(args.output, diff)
                print(args.output)
            else:
                print(json.dumps(diff, indent=2))
            return int(args.gate and (not diff["evaluator_compatible"] or any(
                diff["summary"][key] for key in ("newly_failing", "coverage_lost", "coverage_changed")
            )))
        if args.command == "compare-campaign":
            import corpus_comparison

            comparison = corpus_comparison.compare(args.baseline, args.candidate)
            if args.output:
                write_json(args.output, comparison)
                print(args.output)
            else:
                print(json.dumps(comparison, indent=2))
            return corpus_comparison.result_code(comparison) if args.gate else 0
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
        if args.command == "review":
            import corpus_review

            path = args.output or args.state / "public-review.json"
            write_json(path, corpus_review.review([read_json(path) for path in args.inputs], args.target, args.peer))
            print(path)
            return 0
        if args.command == "triage":
            import corpus_triage

            result = corpus_triage.triage(args.baseline, args.at, args.binary, cache=args.prepared,
                                          case_path=args.cases, jadx_jar=args.jadx_jar,
                                          revision=args.revision, timeout=args.timeout)
            if args.output:
                write_json(args.output, result)
                print(args.output)
            else:
                print(json.dumps(result, indent=2))
            return corpus_triage.result_code(result)
        with closing(candidates.connect(args.state)) as db:
            if args.command == "cycle":
                guards = [(path, read_json(path)) for path in dict.fromkeys(args.guard_baseline)]
                attempt, result = cycle.execute(args, db, read_json(args.baseline), guards)
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
                if result["suite"].startswith("public:"):
                    result["reproduce"].extend(("--corpus", result["suite"].split(":", 1)[1],
                                                "--at", result["at"]))
                    corpus = next(item["corpus"] for item in report["inputs"] if item["suite"] == result["suite"])
                    if corpus.get("execute_models"):
                        result["reproduce"].append("--execute-models")
                    if corpus.get("inspect_structure"):
                        result["reproduce"].append("--inspect-structure")
                    if corpus.get("external_cases"):
                        result["reproduce"].extend(("--cases", corpus["cases"]))
                elif result["suite"].startswith("external:"):
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
