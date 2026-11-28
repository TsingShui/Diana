"""Compare corresponding public campaign units and their independent checks."""

from collections import Counter, defaultdict
import json
from pathlib import Path, PurePosixPath
import re

from dex_calls import LOCATOR
import reporting


CAMPAIGN_KIND = "diana-decbench-campaign"
COMPARISON_KIND = "diana-decbench-campaign-comparison"
MAX_DOCUMENT_BYTES = 128 * 1024 * 1024
MAX_COMPARISON_BYTES = 512 * 1024 * 1024
SHA256 = re.compile(r"[0-9a-f]{64}")
EXIT_CODES = {"PASS": 0, "FAIL": 1, "INCOMPLETE": 2, "SETUP_ERROR": 2, "EMPTY": 0}
EVIDENCE_MODES = ("profile", "execute_models", "inspect_structure")


def bundle_file(directory: Path, name: str) -> Path:
    relative = PurePosixPath(name)
    if (not relative.parts or relative.is_absolute() or ".." in relative.parts
            or "\\" in name or ":" in name or relative.as_posix() != name):
        raise ValueError("campaign file reference must be a normalized relative path")
    path = (directory / relative).resolve(strict=True)
    if not path.is_relative_to(directory):
        raise ValueError("campaign file reference escapes the bundle")
    return path


def read_document(path: Path, budget: list[int]) -> dict:
    if not path.is_file():
        raise ValueError("campaign evidence must be a regular file")
    limit = min(MAX_DOCUMENT_BYTES, budget[0])
    if path.stat().st_size > limit:
        raise ValueError("campaign evidence exceeds the comparison byte bound")
    with path.open("rb") as source:
        data = source.read(limit + 1)
    if len(data) > limit:
        raise ValueError("campaign evidence exceeds the comparison byte bound")
    budget[0] -= len(data)
    try:
        return json.loads(data)
    except RecursionError as error:
        raise ValueError("campaign evidence exceeds the JSON nesting bound") from error


def load_campaign(path: Path, budget: list[int]):
    path = path / "campaign.json" if path.is_dir() else path
    path = bundle_file(path.parent.resolve(strict=True), path.name)
    receipt = read_document(path, budget)
    reporting.require_report(receipt, CAMPAIGN_KIND)
    if (receipt.get("profile") not in ("reviewed", "sample", "full")
            or any(type(receipt.get(key)) is not bool for key in EVIDENCE_MODES[1:])):
        raise ValueError("invalid campaign evidence mode")
    shards = receipt.get("shards")
    if type(shards) is not int or not 1 <= shards <= 512:
        raise ValueError("invalid campaign shard count")
    project_shards = receipt.get("project_shards", {}) if receipt["profile"] == "full" else {}
    if not isinstance(project_shards, dict) or any(type(count) is not int or not shards <= count <= 512
                                                   for count in project_shards.values()):
        raise ValueError("invalid full campaign project shard count")
    units, setup = {}, []
    assigned = defaultdict(set)
    for unit in reporting.rows(receipt, "units"):
        corpus = reporting.string(unit, "corpus")
        stage = reporting.string(unit, "stage")
        status = reporting.string(unit, "status")
        if (status not in EXIT_CODES or type(unit.get("exit_code")) is not int
                or unit["exit_code"] != EXIT_CODES[status]):
            raise ValueError("invalid campaign unit status")
        if stage != "evaluation":
            if stage not in ("prepare", "selection", "export", "coverage") or status not in ("EMPTY", "SETUP_ERROR"):
                raise ValueError("invalid campaign preparation or selection unit")
            setup.append(unit)
            continue

        backend = reporting.string(unit, "backend")
        number = unit.get("shard")
        count = project_shards.get(corpus, shards)
        selected = unit.get("selected")
        if (type(unit.get("shards")) is not int or unit["shards"] != count
                or type(number) is not int or not 1 <= number <= count
                or not isinstance(selected, list) or not 1 <= len(selected) <= 512
                or any(not isinstance(at, str) or not LOCATOR.fullmatch(at) for at in selected)
                or len(set(selected)) != len(selected) or status == "EMPTY"):
            raise ValueError("invalid campaign evaluation selection")
        key = (corpus, backend, number, count)
        seen = assigned[corpus, backend]
        if key in units or seen.intersection(selected):
            raise ValueError(f"campaign repeats a unit or assigned method for {corpus}/{backend}")
        units[key] = unit
        seen.update(selected)
    if not units and not any(item["status"] == "SETUP_ERROR" for item in setup):
        raise ValueError("campaign does not contain evaluations or setup failures")
    return path, receipt, units, setup


def unit_report(path, receipt, unit, budget):
    if unit["status"] == "SETUP_ERROR":
        raise ValueError(unit.get("detail", "campaign unit setup failed"))
    relative = reporting.string(unit, "report")
    report = read_document(bundle_file(path.parent, relative), budget)
    methods, scopes = reporting.index_methods(report)
    if len(scopes) != 1 or report.get("backend") != unit["backend"]:
        raise ValueError("producer report differs from its campaign unit")
    suite, apk, oracle = next(iter(scopes))
    if (suite != f"public:{unit['corpus']}" or not SHA256.fullmatch(apk) or not SHA256.fullmatch(oracle)
            or len(methods) != len(unit["selected"]) or {key[3] for key in methods} != set(unit["selected"])):
        raise ValueError("producer report differs from its campaign selection or pins")
    pin = receipt.get("producer_sha256", {}).get(unit["backend"])
    if pin is not None and report.get("binary", {}).get("sha256") != pin:
        raise ValueError("producer report differs from its campaign executable pin")
    inventory = report["inputs"][0].get("inventory", {})
    if report.get("inventory_complete") is not True or inventory.get("complete") is not True:
        raise ValueError(inventory.get("error") or "producer inventory did not complete")
    outcomes = {check["status"] for _, _, checks in methods.values() for check in checks.values()}
    inspection_failed = report["inputs"][0].get("corpus", {}).get("source_structure", {}).get("SETUP_ERROR")
    if "SETUP_ERROR" in outcomes or inspection_failed or unit.get("recording_error"):
        status = "incomplete"
    elif unit["exit_code"] == 2 or "UNCHECKED" in outcomes:
        status = "partial"
    else:
        status = "compared"
    return report, set(methods), status


def compare(before: Path, after: Path) -> dict:
    budget = [MAX_COMPARISON_BYTES]
    old_path, old, old_units, old_setup = load_campaign(before, budget)
    new_path, new, new_units, new_setup = load_campaign(after, budget)
    summary = Counter(dict.fromkeys(reporting.TRANSITIONS, 0))
    scope = Counter(dict.fromkeys(("compared", "partial", "coverage_changed", "incomplete", "unpaired"), 0))
    units = []
    preparation = [{**item, "side": side} for side, items in (("baseline", old_setup), ("candidate", new_setup))
                   for item in items]
    scope["incomplete"] += sum(item["status"] == "SETUP_ERROR" for item in preparation)
    same_modes = all(old.get(key) == new.get(key) for key in EVIDENCE_MODES)
    for key in sorted(old_units.keys() | new_units.keys()):
        corpus, backend, number, count = key
        left, right = old_units.get(key), new_units.get(key)
        unit = {"corpus": corpus, "backend": backend, "shard": number, "shards": count,
                "baseline": left, "candidate": right}
        if left is None or right is None:
            unit.update(status="unpaired", detail="the other campaign has no matching unit")
        elif not same_modes or set(left["selected"]) != set(right["selected"]):
            unit.update(status="coverage_changed", detail="campaign evidence mode or selected locators differ")
        else:
            try:
                a, original, status_a = unit_report(old_path, old, left, budget)
                b, replacement, status_b = unit_report(new_path, new, right, budget)
            except (OSError, ValueError, TypeError, AttributeError) as error:
                unit.update(status="incomplete", detail=str(error))
            else:
                if original != replacement:
                    old_input, new_input = a["inputs"][0], b["inputs"][0]
                    if old_input["sha256"] != new_input["sha256"]:
                        detail = "original APK digests differ"
                    elif old_input["oracle_sha256"] != new_input["oracle_sha256"]:
                        detail = "independent oracle digests differ"
                    else:
                        detail = "original member identities differ"
                    unit.update(status="coverage_changed", detail=detail)
                elif a.get("evaluator_sha256") != b.get("evaluator_sha256"):
                    unit.update(status="coverage_changed", detail="evaluator fingerprints differ")
                else:
                    delta = reporting.compare_reports(a, b)
                    status = next(value for value in ("incomplete", "partial", "compared")
                                  if value in (status_a, status_b))
                    unit.update(status=status, comparison=delta)
                    summary.update(delta["summary"])
        scope[unit["status"]] += 1
        units.append(unit)

    def campaign_metadata(path, receipt):
        return {"receipt": str(path), **{key: receipt.get(key) for key in (*EVIDENCE_MODES, "revision", "jobs")}}

    return {"kind": COMPARISON_KIND, "schema_version": 1, "baseline": campaign_metadata(old_path, old),
            "candidate": campaign_metadata(new_path, new), "summary": dict(summary), "scope": dict(scope),
            "preparation": preparation, "units": units}


def result_code(comparison: dict) -> int:
    if any(comparison["scope"][key] for key in ("coverage_changed", "incomplete", "unpaired")):
        return 2
    return int(any(comparison["summary"][key] for key in ("newly_failing", "coverage_lost", "coverage_changed")))


def render(document: dict) -> str:
    reporting.require_report(document, COMPARISON_KIND)
    label = reporting.label
    rows = []
    for number, unit in enumerate(document["units"]):
        title = label(f"{unit['corpus']} · {unit['backend']} · {unit['shard']}/{unit['shards']} · {unit['status']}")
        delta = unit.get("comparison")
        if delta:
            details = f"<pre>{reporting.format_data(delta['summary'])}</pre>"
            details += "".join(reporting.render_method(row, True, f"unit-{number}-method-{index}")
                               for index, row in enumerate(delta["methods"])
                               if row["publication_changed"] or any(check["transition"] != "unchanged" for check in row["checks"]))
        else:
            details = f"<p>{label(unit.get('detail', ''))}</p>"
        rows.append(f"<details><summary>{title}</summary>{details}</details>")
    summary = reporting.format_data({key: document[key] for key in ("baseline", "candidate", "summary", "scope")})
    preparation = reporting.format_data(document["preparation"])
    body = f"<pre>{summary}</pre><h2>Preparation and selection</h2><pre>{preparation}</pre><h2>Evaluation units</h2>"
    return reporting.html_page("Diana DecBench campaign comparison", body + "".join(rows))
