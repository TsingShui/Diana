"""Run comparable public corpus evaluations from cached inputs."""

from collections import Counter
from math import ceil
from pathlib import Path
import sqlite3
import subprocess
from tarfile import TarError
from tempfile import mkdtemp
from zipfile import BadZipFile

import candidates
import corpus_assets
import corpus_bench
import corpus_coverage
import corpus_discover
import corpus_eval
import corpus_export
import corpus_review
from evidence import digest, write_json
import judge
import reporting


SETUP_ERRORS = (OSError, ValueError, KeyError, subprocess.SubprocessError, TarError, BadZipFile)


def selection(project, reviewed, discovery, limit, shard, state, directory, *, exhaustive=False):
    number, count = shard
    snapshot = directory / "cases.json"
    if discovery is None:
        methods = reviewed["methods"][number - 1::count][:limit]
        manifest = {**reviewed, "methods": methods}
        write_json(snapshot, manifest)
        return [case["at"] for case in methods], None, manifest

    manifest = (discovery.select(limit, shard, include=reviewed["methods"]) if exhaustive
                else discovery.select(limit or 100, shard))
    if exhaustive:
        manifest["discovery"]["exhaustive"] = True
    selected = [case["at"] for case in manifest["methods"]]
    write_json(snapshot, manifest)
    if not selected:
        return [], snapshot, manifest
    # Keep one stable manifest path across attempts so normal evaluation checkpoints can be reused.
    path = state / "campaign-cases" / project.id / f"{digest(manifest)}.json"
    write_json(path, manifest)
    return selected, path, manifest


def write_report(directory, unit, backend, report):
    path = unit / f"{backend}.json"
    page = path.with_suffix(".html")
    report["link"] = page.relative_to(directory).as_posix()
    write_json(path, report)
    page.write_text(reporting.render_report(report), encoding="utf-8")
    return {"report": path.relative_to(directory).as_posix(), "html": report["link"]}


def run(project_ids: list[str] | None, cache: Path, output: Path, state: Path, binary: Path, *,
        jadx_jar: Path | None = None, revision: str | None = None, profile: str = "reviewed",
        limit: int | None = None, shards: int = 1, execute_models: bool = False,
        inspect_structure: bool = False, resume: bool = False, jobs: int = 4, timeout: int = 45) -> tuple[Path, int]:
    """Evaluate each requested project/shard using one shared selection for both arms."""
    projects = corpus_assets.load_projects()
    selected_projects = sorted(set(project_ids or projects))
    unknown = set(selected_projects) - projects.keys()
    if unknown or not selected_projects:
        raise ValueError(f"unknown public corpora: {', '.join(sorted(unknown))}" if unknown else "no public corpora")
    if profile not in ("reviewed", "sample", "full"):
        raise ValueError("campaign profile must be reviewed, sample, or full")
    maximum = corpus_discover.MAX_METHODS
    if not 1 <= shards <= maximum or (limit is not None and not 1 <= limit <= maximum):
        raise ValueError(f"campaign shard count and per-shard limit must be between 1 and {maximum}")
    full = profile == "full"
    if full:
        limit = limit or maximum
        inspect_structure = True

    cache, state, output = cache.resolve(), state.resolve(), output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    directory = Path(mkdtemp(prefix="campaign-", dir=output))
    arms = {"diana": binary.resolve()}
    if jadx_jar is not None:
        arms["jadx"] = jadx_jar.resolve()
    reports, units = [], []
    checks = {backend: Counter() for backend in arms}
    producer_hashes = {}
    coverage_by_project = {}
    project_shards = {}

    def setup_error(scope, stage, error):
        units.append({**scope, "stage": stage, "status": "SETUP_ERROR", "exit_code": 2,
                      "detail": f"{type(error).__name__}: {error}"})

    for project_id in selected_projects:
        project = projects[project_id]
        project_scope = {"corpus": project_id}
        try:
            assets = corpus_assets.prepare_assets(project, cache)
            reviewed_path, reviewed = corpus_eval.case_configuration(project, None)
        except SETUP_ERRORS as error:
            setup_error(project_scope, "prepare", error)
            continue
        discovery = None
        if profile in ("sample", "full"):
            try:
                discovery = corpus_discover.prepare_discovery(
                    assets.apk, assets.source, project.source_root, project.source_commit,
                    project.provenance["source_repository_url"],
                    input_sha256=reviewed["input_sha256"], revision_entry=reviewed.get("revision_entry"),
                    exclude=[case["at"] for case in reviewed["methods"]],
                )
            except SETUP_ERRORS as error:
                setup_error(project_scope, "selection", error)
                continue
        count = shards
        exported = None
        if full:
            originals = [*reviewed["methods"], *discovery.candidates]
            count = max(shards, ceil(len(originals) / limit))
            if count > maximum:
                setup_error(project_scope, "selection", ValueError(
                    f"full scan needs {count} shards; increase the per-shard limit",
                ))
                continue
            try:
                exported = corpus_export.publish(arms["diana"], assets.apk, originals, timeout, directory)
            except SETUP_ERRORS as error:
                setup_error(project_scope, "export", error)
                continue
        project_shards[project_id] = count
        project_selected = False
        planned, evaluated = [], []
        for number in range(1, count + 1):
            shard_scope = {**project_scope, "shard": number, "shards": count}
            unit = directory / project_id / f"shard-{number:04d}"
            try:
                selected, case_path, manifest = selection(
                    project, reviewed, discovery, limit, (number, count), state, unit, exhaustive=full,
                )
                planned.append(manifest)
                write_json(unit / "selection.json", {"methods": selected, "cases": "cases.json",
                                                       "evaluation_cases": str(case_path or reviewed_path)})
            except SETUP_ERRORS as error:
                setup_error(shard_scope, "selection", error)
                continue
            if not selected:
                units.append({**shard_scope, "stage": "selection", "status": "EMPTY", "exit_code": 0})
                continue
            project_selected = True
            for backend, producer in arms.items():
                scope = {**shard_scope, "backend": backend, "selected": selected}
                try:
                    report = corpus_eval.run(
                        project, cache, producer, revision if backend == "diana" else None, backend,
                        selected, jobs, timeout, execute_models=execute_models, inspect_structure=inspect_structure,
                        case_path=case_path, checkpoint=state / "checkpoints" if resume else None,
                        **({"exported": exported} if full and backend == "diana" else {}),
                    )
                    producer_hash = report["binary"]["sha256"]
                    if producer_hashes.setdefault(backend, producer_hash) != producer_hash:
                        raise ValueError(f"the {backend} producer changed during the campaign")
                    files = write_report(directory, unit, backend, report)
                except SETUP_ERRORS as error:
                    setup_error(scope, "evaluation", error)
                    continue
                reports.append(report)
                evaluated.append(report)
                checks[backend].update(report["checks_by_status"])
                exit_code = judge.result_code(report)
                status = ("PASS", "FAIL", "INCOMPLETE")[exit_code]
                record = {**scope, **files, "stage": "evaluation", "status": status, "exit_code": exit_code,
                          "checks": report["checks_by_status"], "inventory_complete": report["inventory_complete"]}
                try:
                    durable = {key: value for key, value in report.items() if key != "link"}
                    record["recorded_run"] = str(candidates.record_run(state, durable))
                except (OSError, sqlite3.Error) as error:
                    record.update(exit_code=2, status="INCOMPLETE", recording_error=str(error))
                units.append(record)
        if not project_selected and not any(unit["corpus"] == project_id and unit["exit_code"] == 2 for unit in units):
            setup_error(project_scope, "selection", ValueError("no source-aligned methods were selected"))
        coverage = corpus_coverage.measure(assets.apk, assets.source, project.source_root,
                                           cases=planned, reports=evaluated)
        coverage["corpus"] = project_id
        coverage_path = directory / project_id / "coverage.json"
        coverage_page = coverage_path.with_suffix(".html")
        write_json(coverage_path, coverage)
        coverage_page.write_text(corpus_coverage.render(coverage), encoding="utf-8")
        coverage_by_project[project_id] = {
            "report": coverage_path.relative_to(directory).as_posix(),
            "html": coverage_page.relative_to(directory).as_posix(),
            "complete": corpus_coverage.result_code(coverage) == 0,
        }
        if not coverage_by_project[project_id]["complete"]:
            setup_error(project_scope, "coverage", ValueError(
                f"independent population coverage was incomplete. See {coverage_by_project[project_id]['report']}",
            ))

    index = reporting.build_index(reports)
    write_json(directory / "index.json", index)
    (directory / "index.html").write_text(reporting.render_index(index), encoding="utf-8")
    write_json(directory / "opportunities.json", index["mining"])
    artifacts = {"index": "index.json", "index_html": "index.html", "opportunities": "opportunities.json"}
    if full or "jadx" in arms:
        review = corpus_review.review(reports, peer_backend="jadx" if "jadx" in arms else None)
        write_json(directory / "review.json", review)
        (directory / "review.html").write_text(corpus_review.render(review), encoding="utf-8")
        artifacts.update(review="review.json", review_html="review.html")
    if full:
        (directory / "bench.html").write_text(corpus_bench.render(reports, projects, cache), encoding="utf-8")
        artifacts["bench_html"] = "bench.html"

    receipt = directory / "campaign.json"
    exit_code = max(unit["exit_code"] for unit in units)
    write_json(receipt, {"kind": "diana-decbench-campaign", "schema_version": 1,
                        "projects": selected_projects, "profile": profile, "shards": shards,
                        **({"project_shards": project_shards} if full else {}),
                        "limit": limit if limit is not None or profile == "reviewed" else 100,
                        "producers": {backend: str(path) for backend, path in arms.items()}, "revision": revision,
                        "producer_sha256": producer_hashes, "prepared": str(cache),
                        "execute_models": execute_models, "inspect_structure": inspect_structure,
                        "jobs": jobs, "timeout": timeout, "checks_by_backend": checks,
                        "units": units, "artifacts": artifacts, "coverage": coverage_by_project, "exit_code": exit_code})
    return receipt, exit_code
