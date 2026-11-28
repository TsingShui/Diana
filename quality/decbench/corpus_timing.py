"""Measure paired native Diana CLI latency on one already verified corpus input."""

from __future__ import annotations

from hashlib import sha256
import math
from pathlib import Path
import platform
from statistics import median
import time

import evidence

KIND = "diana-decbench-timing"
ARMS = ("baseline", "candidate")
CLOCK = "subprocess wall time, including process startup and JSON transport"


def file_identity(path: Path) -> dict:
    resolved = path.resolve(strict=True)
    return {"path": str(resolved), "sha256": evidence.file_digest(resolved)}


def observation(capture: dict, started: float) -> dict:
    elapsed = capture.get("elapsed_seconds")
    return {
        "exit_code": capture.get("exit_code"),
        "elapsed_seconds": elapsed if elapsed is not None else time.perf_counter() - started,
        **{key: capture[key] for key in ("stderr", "error") if capture.get(key)},
    }


def unavailable(reason: str) -> dict:
    return {"exit_code": None, "elapsed_seconds": None, "error": reason}


def probed_inventory(binary: Path, artifact: Path, selected: dict[str, str], timeout: float) -> tuple[dict, set]:
    if timeout <= 0:
        return unavailable("benchmark time budget exhausted before inventory"), set()
    started = time.perf_counter()
    methods, capture = evidence.inventory(binary, artifact, timeout)
    row = observation(capture, started)
    row["methods"] = len(methods)
    if not capture.get("complete") or row["exit_code"] != 0:
        row.setdefault("error", "the full method inventory was incomplete")
        return row, set()
    found = dict(methods)
    missing = [
        {"at": at, "expected": member, "observed": found.get(at)}
        for at, member in selected.items() if found.get(at) != member
    ]
    row["inventory_sha256"] = evidence.digest(sorted(methods))
    if missing:
        row.update(error="the full method inventory differed at selected original identities", mismatches=missing)
    return row, methods


def probed_method(binary: Path, artifact: Path, at: str, member: str, timeout: float) -> dict:
    if timeout <= 0:
        return unavailable("benchmark time budget exhausted before method retrieval")
    argv = [str(binary), "get-method", str(artifact), at, "--format", "json"]
    started = time.perf_counter()
    capture = evidence.run_cli(argv, timeout)
    row = observation(capture, started)
    publication = capture.get("publication", {})
    result, selection = publication.get("result"), publication.get("selection")
    locator = selection.get("method_at") if isinstance(selection, dict) else None
    issues = publication.get("issues")
    if isinstance(issues, list) and issues:
        row["issues"] = issues
    if isinstance(locator, str):
        row["reported_locator"] = locator
    # Schema 5 can expose only the owning class in the locator suffix. The preceding
    # complete inventory established the exact name and descriptor at that coordinate.
    owner = member.partition("->")[0]
    text = result.get("text") if isinstance(result, dict) else None
    if (publication.get("schema_version") != 5 or publication.get("command") != "get-method"
            or row["exit_code"] != 0 or issues != [] or not isinstance(text, str) or not text.strip()
            or locator not in (f"{at}::{member}", f"{at}::{owner}->")):
        row.setdefault("error", "method retrieval did not confirm a complete publication for the original identity")
        return row
    encoded = text.encode("utf-8", "surrogatepass")
    row.update(publication_sha256=sha256(encoded).hexdigest(), publication_bytes=len(encoded))
    return row


def trial(index: int, binaries: dict, artifact: Path, selected: dict[str, str],
          timeout: float, deadline: float) -> dict:
    """Pair each workload immediately and reverse the first arm between rounds."""
    order = ARMS if index % 2 == 0 else ARMS[::-1]
    inventories = {}
    methods = {}
    for arm in order:
        remaining = min(timeout, deadline - time.perf_counter())
        inventories[arm], methods[arm] = probed_inventory(binaries[arm], artifact, selected, remaining)
    retrievals = []
    for at, member in selected.items():
        pair = {}
        for arm in order:
            remaining = min(timeout, deadline - time.perf_counter())
            if (at, member) in methods[arm]:
                pair[arm] = probed_method(binaries[arm], artifact, at, member, remaining)
            else:
                pair[arm] = unavailable("the complete inventory did not establish the selected identity")
        retrievals.append({"at": at, "member": member, **pair})
    return {"index": index, "order": list(order), "inventory": inventories, "methods": retrievals}


def comparable(pair: dict) -> bool:
    return all(
        "error" not in pair[arm] and pair[arm]["exit_code"] == 0
        and isinstance(pair[arm]["elapsed_seconds"], (int, float))
        and math.isfinite(pair[arm]["elapsed_seconds"]) and pair[arm]["elapsed_seconds"] >= 0
        for arm in ARMS
    )


def summary(pairs: list[dict]) -> dict:
    series = {arm: [pair[arm]["elapsed_seconds"] for pair in pairs] for arm in ARMS}
    result = {arm: {"median_seconds": median(values), "total_seconds": math.fsum(values)}
              for arm, values in series.items()}
    baseline, candidate = (result[arm]["median_seconds"] for arm in ARMS)
    differences = [after - before for before, after in zip(series["baseline"], series["candidate"])]
    paired_median = median(differences)
    result["median_paired_delta_seconds"] = paired_median
    result["paired_deltas"] = {
        "median_absolute_deviation_seconds": median(abs(value - paired_median) for value in differences),
        "minimum_seconds": min(differences),
        "maximum_seconds": max(differences),
        "candidate_faster": sum(value < 0 for value in differences),
        "candidate_slower": sum(value > 0 for value in differences),
        "tied": sum(value == 0 for value in differences),
    }
    result["relative_change_between_medians_percent"] = 100 * (candidate / baseline - 1) if baseline > 0 else None
    return result


def comparisons(warmup: dict, measurements: list[dict], selected: dict[str, str],
                repeats: int, invalidated: bool) -> dict:
    enough_trials = not invalidated and len(measurements) == repeats
    inventories = [round_["inventory"] for round_ in measurements]
    all_inventories = [warmup["inventory"], *inventories]
    available_inventory = enough_trials and all(comparable(pair) for pair in all_inventories)
    hashes = [pair[arm].get("inventory_sha256") for pair in all_inventories for arm in ARMS]
    inventory_mismatch = enough_trials and None not in hashes and len(set(hashes)) != 1
    same_inventory = available_inventory and not inventory_mismatch
    method_rows = []
    for index, (at, member) in enumerate(selected.items()):
        pairs = [round_["methods"][index] for round_ in measurements]
        available = enough_trials and all(comparable(pair) for pair in [warmup["methods"][index], *pairs])
        method_row = {"at": at, "member": member, "comparison": summary(pairs) if available else None}
        if available:
            digests = {arm: {pair[arm]["publication_sha256"] for pair in pairs} for arm in ARMS}
            method_row["publication"] = {
                "distinct_outputs": {arm: len(values) for arm, values in digests.items()},
                "identical_all_trials": len(digests["baseline"] | digests["candidate"]) == 1,
            }
        method_rows.append(method_row)
    full = all(row["comparison"] is not None for row in method_rows)
    totals = [
        {arm: {"elapsed_seconds": math.fsum(row[arm]["elapsed_seconds"] for row in round_["methods"])}
         for arm in ARMS}
        for round_ in measurements
    ] if full else []
    return {
        "comparable": same_inventory and full,
        "inventory": summary(inventories) if same_inventory else None,
        "inventory_mismatch": inventory_mismatch,
        "methods": method_rows,
        "method_totals": summary(totals) if full else None,
    }


def measure(baseline: Path, candidate: Path, artifact: Path, methods: list[dict], repeats: int = 5,
            timeout: float = 45.0, *, source_archive: Path | None = None, time_budget: float = 300.0) -> dict:
    """Warm up both executables, then measure same-host workload pairs in alternating order.

    Timeout bounds each process and time_budget bounds the combined benchmark block.
    The verified source archive, when supplied, is recorded as provenance only.
    """
    if type(repeats) is not int or repeats < 1 or any(
        not math.isfinite(value) or value <= 0 for value in (timeout, time_budget)
    ):
        raise ValueError("timing requires a positive repeat count and positive finite time limits")
    selected = {}
    for method in methods:
        at, member = (method.get(field) for field in ("at", "member"))
        if (not isinstance(at, str) or not at or not isinstance(member, str) or "->" not in member
                or at in selected):
            raise ValueError("timing methods require nonempty, unique original locators and full member identities")
        selected[at] = member
    if not selected:
        raise ValueError("timing requires at least one verified original method")
    inputs = {"baseline": file_identity(baseline), "candidate": file_identity(candidate),
              "artifact": file_identity(artifact)}
    if source_archive is not None:
        inputs["source_archive"] = file_identity(source_archive)
    binaries = {arm: Path(inputs[arm]["path"]) for arm in ARMS}
    pinned_artifact = Path(inputs["artifact"]["path"])
    deadline = time.perf_counter() + time_budget
    warmup = trial(0, binaries, pinned_artifact, selected, timeout, deadline)
    measurements = []
    for index in range(1, repeats + 1):
        if time.perf_counter() >= deadline:
            break
        measurements.append(trial(index, binaries, pinned_artifact, selected, timeout, deadline))
    errors = []
    if len(measurements) != repeats:
        errors.append("benchmark time budget expired before every measured round began")
    changed = False
    for key, identity in inputs.items():
        try:
            current = evidence.file_digest(Path(identity["path"]))
        except (OSError, ValueError) as error:
            errors.append(f"cannot recheck {key}: {error}")
            changed = True
        else:
            if current != identity["sha256"]:
                errors.append(f"{key} changed during the benchmark")
                changed = True
    comparison = comparisons(warmup, measurements, selected, repeats, changed)
    return {
        "schema_version": 1, "kind": KIND, "comparable": comparison["comparable"],
        "provenance": {**inputs, "selection_sha256": evidence.digest(list(selected.items())),
                       "same_binary_bytes": inputs["baseline"]["sha256"] == inputs["candidate"]["sha256"],
                       "host": {"system": platform.system(), "machine": platform.machine()}, "clock": CLOCK},
        "repeats": repeats, "timeout_seconds": timeout, "time_budget_seconds": time_budget,
        "warmup": warmup, "measurements": measurements, "comparison": comparison, "errors": errors,
    }
