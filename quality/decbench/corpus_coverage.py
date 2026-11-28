"""Describe the populations and measured subsets of a public corpus run."""

from __future__ import annotations

from collections import Counter, defaultdict
from hashlib import sha256
from pathlib import Path, PurePosixPath
import re
from tarfile import TarError, open as open_tar
from zipfile import BadZipFile, ZipFile
from zlib import error as CompressionError

from corpus_oracles import (
    BYTE_LIMIT, MAX_ARCHIVE_ENTRIES, SOURCE_ARCHIVE_LIMIT, SOURCE_SCAN_LIMIT,
    logical_path, validate_cases,
)
from dex_truth import APK_FILE_BYTES, APK_ROOT, archive_methods
from evidence import file_digest

LANGUAGES = {".java": "java", ".kt": "kotlin"}
CHECK_STATES = ("PASS", "FAIL", "UNCHECKED", "SETUP_ERROR")
STRUCTURE_STATES = ("MATCH", "DIFFERENT", "UNCHECKED", "SETUP_ERROR")
MEASURED = {"PASS", "FAIL"}
INFRASTRUCTURE = {"identity", "publication", "body_recovery"}
SOURCE_LAYOUT = re.compile(r"(?:(?P<module>.+)/)?src/(?P<set>[^/]+)/(?:java|kotlin)/.+")


def source_layout(path):
    matched = SOURCE_LAYOUT.fullmatch(path)
    return (matched["module"] or ".", matched["set"]) if matched else (None, None)


def original_inventory(path):
    if not path.is_file() or path.stat().st_size > APK_FILE_BYTES:
        raise ValueError("APK input is not a regular file within the inventory size bound")
    checksum = file_digest(path)
    with ZipFile(path) as archive:
        roots = [{"entry": index, "name": item.orig_filename}
                 for index, item in enumerate(archive.infolist()) if APK_ROOT.fullmatch(item.orig_filename)]
        methods = dict(archive_methods(archive))
    if file_digest(path) != checksum:
        raise ValueError("APK bytes changed during the independent inventory")
    return {
        "sha256": checksum, "complete": True, "root_dex_entries": roots, "method_definitions": len(methods),
        "classes_with_method_definitions": len({at.rpartition(":m")[0] for at in methods}),
    }, methods


def languages(counts):
    return {language: counts[language] for language in LANGUAGES.values()}


def source_inventory(path, root, pinned_paths):
    if not path.is_file() or path.stat().st_size > SOURCE_ARCHIVE_LIMIT:
        raise ValueError("source archive is not a regular file within the inventory size bound")
    checksum = file_digest(path)
    prefix = logical_path(root) + "/"
    seen = set()
    duplicates = set()
    source_paths = set()
    source_pins = {}
    expanded = 0
    found_root = False
    unsafe_paths = 0
    with open_tar(path, mode="r|gz") as archive:
        for number, entry in enumerate(archive, 1):
            if entry.size < 0:
                raise ValueError("negative source archive entry size")
            expanded += entry.size
            if expanded > SOURCE_SCAN_LIMIT or number > MAX_ARCHIVE_ENTRIES:
                raise ValueError("uncompressed source archive exceeds the inventory bound")
            if entry.name == root and entry.isdir():
                found_root = True
                continue
            if not entry.name.startswith(prefix):
                continue
            found_root = True
            try:
                relative = logical_path(entry.name[len(prefix):])
            except ValueError:
                unsafe_paths += 1
                continue
            if relative in seen:
                duplicates.add(relative)
                source_pins.pop(relative, None)
                continue
            seen.add(relative)
            if not entry.isfile():
                continue
            if PurePosixPath(relative).suffix in LANGUAGES:
                source_paths.add(relative)
            if relative in pinned_paths and entry.size <= BYTE_LIMIT:
                with archive.extractfile(entry) as stream:
                    content = stream.read(BYTE_LIMIT + 1)
                if len(content) != entry.size:
                    raise ValueError("selected source entry size disagrees with the archive header")
                try:
                    lines = len(content.decode("utf-8").splitlines())
                except UnicodeError:
                    continue
                source_pins[relative] = (sha256(content).hexdigest(), lines)
    if not found_root:
        raise ValueError("configured source root is absent from the archive")
    if file_digest(path) != checksum:
        raise ValueError("source archive bytes changed during the inventory")

    files = Counter()
    source_sets = defaultdict(Counter)
    modules = defaultdict(Counter)
    for relative in source_paths - duplicates:
        language = LANGUAGES[PurePosixPath(relative).suffix]
        module, source_set = source_layout(relative)
        files[language] += 1
        source_sets[source_set or "(outside src/<source-set>/java or kotlin)"][language] += 1
        if module is not None:
            modules[module][language] += 1
    return {
        "sha256": checksum, "root": root, "complete": True, "files_by_language": languages(files),
        "files_by_source_set": {name: languages(counts) for name, counts in sorted(source_sets.items())},
        "files_by_module": {name: languages(counts) for name, counts in sorted(modules.items())},
        "ambiguous_language_paths": sum(PurePosixPath(path).suffix in LANGUAGES for path in duplicates),
        "invalid_paths": unsafe_paths,
    }, source_pins


def rows(document):
    value = document.get("methods") if isinstance(document, dict) else None
    return value if isinstance(value, list) else []


def inputs(report):
    value = report.get("inputs") if isinstance(report, dict) else None
    return [entry for entry in value if isinstance(entry, dict)] if isinstance(value, list) else []


def source_path(row):
    pin = row.get("source") if isinstance(row, dict) else None
    value = pin.get("path") if isinstance(pin, dict) else None
    return value if isinstance(value, str) else None


def anchor(row, methods):
    if not isinstance(row, dict) or not isinstance(row.get("at"), str):
        return None
    at, member = row["at"], row.get("member")
    if isinstance(member, str) and methods.get(at) == member:
        return at, member
    return None


def source_pin_matches(row, pins):
    path = source_path(row)
    if path not in pins:
        return False
    checksum, lines = pins[path]
    source = row["source"]
    first, last = source.get("start_line"), source.get("end_line")
    return (source.get("sha256") == checksum and type(first) is int and type(last) is int
            and 1 <= first <= last <= lines)


def discovery_scope(document):
    discovery = document.get("discovery")
    if not isinstance(discovery, dict):
        return None
    candidate_methods = discovery.get("candidate_methods")
    if type(candidate_methods) is not int or candidate_methods < 0:
        candidate_methods = None
    return {
        "kind": discovery.get("kind"), "candidate_methods": candidate_methods,
        "limit": discovery.get("limit"), "shard": discovery.get("shard"),
    }


def selection(cases, artifact_hash, methods, pins):
    selected = set()
    pinned = set()
    modules = defaultdict(set)
    language_methods = defaultdict(set)
    obligation_methods = defaultdict(set)
    manifests = []
    for number, case in enumerate(cases, 1):
        result = {"index": number}
        manifests.append(result)
        try:
            validate_cases(case, f"case manifest {number}", allow_empty=True)
            if artifact_hash is None or case["input_sha256"] != artifact_hash:
                raise ValueError("manifest APK fingerprint could not be matched to the independent inventory")
        except (TypeError, ValueError) as error:
            result["unavailable"] = str(error)
            continue

        original_matches = 0
        source_matches = 0
        obligation_counts = Counter()
        for row in rows(case):
            key = anchor(row, methods)
            if key is None:
                continue
            selected.add(key)
            original_matches += 1
            if not source_pin_matches(row, pins):
                continue
            source_matches += 1
            pinned.add(key)
            path = source_path(row)
            module, _ = source_layout(path)
            modules[module or "(outside src/<source-set>/java or kotlin)"].add(key)
            language_methods[LANGUAGES.get(PurePosixPath(path).suffix, "other")].add(key)
            kinds = {obligation["kind"] for obligation in row["obligations"]}
            obligation_counts.update(kinds)
            for kind in kinds:
                obligation_methods[kind].add(key)
        requested = len(rows(case))
        result.update({
            "source_revision": case["source_revision"], "discovery": discovery_scope(case),
            "requested_methods": requested, "matched_original_methods": original_matches,
            "matched_original_and_source_pin_methods": source_matches,
            "unmatched_original_methods": requested - original_matches,
            "unmatched_source_pins_for_matched_originals": original_matches - source_matches,
            "matched_source_pin_methods_by_obligation_kind": dict(sorted(obligation_counts.items())),
        })
    return {
        "manifests": manifests,
        "matched_original_methods": len(selected), "matched_original_and_source_pin_methods": len(pinned),
        "matched_source_pin_methods_by_module": {name: len(found) for name, found in sorted(modules.items())},
        "matched_source_pin_methods_by_language": {name: len(found) for name, found in sorted(language_methods.items())},
        "matched_source_pin_methods_by_obligation_kind": {
            kind: len(found) for kind, found in sorted(obligation_methods.items())
        },
    }, selected


def summarize_evidence(evidence, states, measured):
    return {
        "methods_with_receipts": len(evidence),
        "methods_with_any_measured_receipt": sum(bool(measured & outcomes) for outcomes in evidence.values()),
        "methods_with_only_measured_receipts": sum(outcomes <= measured for outcomes in evidence.values()),
        "methods_by_receipt_state": {state: sum(state in outcomes for outcomes in evidence.values())
                                     for state in states},
    }


def report_population(entry, corpus, methods, pins, combined):
    identified = set()
    aligned = set()
    result = defaultdict(lambda: defaultdict(set))
    counts = defaultdict(Counter)
    kinds = defaultdict(lambda: defaultdict(set))
    kind_counts = defaultdict(Counter)
    structure = defaultdict(set)
    structure_counts = Counter()
    unmatched_originals = 0
    unsupported_receipts = 0
    for row in rows(entry):
        key = anchor(row, methods)
        if key is None:
            unmatched_originals += 1
            continue
        identified.add(key)
        if not source_pin_matches(row, pins):
            continue
        aligned.add(key)
        checks = row.get("checks")
        for check in checks if isinstance(checks, list) else ():
            metric = check.get("metric") if isinstance(check, dict) else None
            state = check.get("status") if isinstance(check, dict) else None
            if not isinstance(metric, str) or not metric or state not in CHECK_STATES:
                unsupported_receipts += 1
                continue
            result[metric][key].add(state)
            counts[metric][state] += 1
            combined["families"][metric][key].add(state)
            kind = check.get("kind")
            if isinstance(kind, str) and kind:
                kinds[kind][key].add(state)
                kind_counts[kind][state] += 1
                combined["obligation_kinds"][kind][key].add(state)
        receipt = row.get("source_structure")
        if receipt is None:
            continue
        outcome = receipt.get("outcome") if isinstance(receipt, dict) else None
        if outcome not in STRUCTURE_STATES:
            unsupported_receipts += 1
            continue
        structure[key].add(outcome)
        structure_counts[outcome] += 1
        combined["source_structure"][key].add(outcome)
    combined["originals"].update(identified)
    combined["aligned"].update(aligned)

    return {
        "corpus": corpus.get("id"), "source_revision": corpus.get("source_revision"),
        "profile": corpus.get("profile"), "oracle_sha256": entry.get("oracle_sha256"),
        "discovery": discovery_scope(corpus), "method_rows": len(rows(entry)),
        "matched_original_methods": len(identified), "matched_original_and_source_pin_methods": len(aligned),
        "unmatched_original_rows": unmatched_originals,
        "unmatched_source_pins_for_matched_originals": len(identified - aligned),
        "unsupported_receipts": unsupported_receipts,
        "families": {metric: {**summarize_evidence(evidence, CHECK_STATES, MEASURED),
                              "receipts_by_state": dict(sorted(counts[metric].items()))}
                     for metric, evidence in sorted(result.items())},
        "obligation_kinds": {kind: {**summarize_evidence(evidence, CHECK_STATES, MEASURED),
                                    "receipts_by_state": dict(sorted(kind_counts[kind].items()))}
                             for kind, evidence in sorted(kinds.items())},
        "source_structure": {**summarize_evidence(structure, STRUCTURE_STATES, {"MATCH", "DIFFERENT"}),
                             "receipts_by_state": dict(sorted(structure_counts.items()))},
    }


def provenance(report):
    binary = report.get("binary")
    values = {
        "backend": report.get("backend"), "revision": report.get("revision"),
        "binary_sha256": binary.get("sha256") if isinstance(binary, dict) else None,
        "evaluator_sha256": report.get("evaluator_sha256"),
    }
    return {key: value if isinstance(value, str) and value else None for key, value in values.items()}


def report_coverage(reports, artifact_hash, source_hash, methods, pins):
    snapshots = []
    groups = {}
    for number, report in enumerate(reports, 1):
        entry_summaries = []
        snapshot = {"index": number, "inputs": entry_summaries}
        snapshots.append(snapshot)
        if (not isinstance(report, dict) or report.get("kind") != "diana-decbench-run"
                or report.get("schema_version") != 1):
            snapshot["unavailable"] = "expected a version-1 DecBench run report"
            continue
        origin = provenance(report)
        snapshot.update(origin)
        group_key = (*origin.values(), None if all(origin[key] for key in (
            "backend", "binary_sha256", "evaluator_sha256",
        )) else number)
        report_inputs = inputs(report)
        if not report_inputs:
            snapshot["unavailable"] = "run report has no usable input records"
        for entry in report_inputs:
            corpus = entry.get("corpus")
            if not isinstance(corpus, dict):
                entry_summaries.append({"suite": entry.get("suite"), "skipped": "non-public corpus input"})
                continue
            if (artifact_hash is None or source_hash is None or entry.get("sha256") != artifact_hash
                    or corpus.get("source_archive_sha256") != source_hash):
                entry_summaries.append({"suite": entry.get("suite"), "unavailable":
                                        "report APK and source archive fingerprints could not both be matched"})
                continue
            if entry.get("inventory_only"):
                entry_summaries.append({"suite": entry.get("suite"), "skipped": "inventory-only report input"})
                continue
            group = groups.setdefault(group_key, {
                "provenance": origin, "reports": set(), "originals": set(), "aligned": set(),
                "families": defaultdict(lambda: defaultdict(set)),
                "obligation_kinds": defaultdict(lambda: defaultdict(set)), "source_structure": defaultdict(set),
            })
            group["reports"].add(number)
            entry_summaries.append(report_population(entry, corpus, methods, pins, group))
    combined = []
    for group in groups.values():
        substantive = set().union(*(evidence.keys() for metric, evidence in group["families"].items()
                                    if metric not in INFRASTRUCTURE))
        measured = {key for metric, evidence in group["families"].items() if metric not in INFRASTRUCTURE
                    for key, states in evidence.items() if states & MEASURED}
        combined.append({
            **group["provenance"], "reports": sorted(group["reports"]),
            "matched_original_methods": len(group["originals"]),
            "matched_original_and_source_pin_methods": len(group["aligned"]),
            "methods_with_quality_check_receipts": len(substantive),
            "methods_with_measured_quality_checks": len(measured),
            "original_methods_without_measured_quality_checks": len(methods) - len(measured),
            "families": {metric: summarize_evidence(evidence, CHECK_STATES, MEASURED)
                         for metric, evidence in sorted(group["families"].items())},
            "obligation_kinds": {kind: summarize_evidence(evidence, CHECK_STATES, MEASURED)
                                 for kind, evidence in sorted(group["obligation_kinds"].items())},
            "source_structure": summarize_evidence(group["source_structure"], STRUCTURE_STATES,
                                                   {"MATCH", "DIFFERENT"}),
        })
    return {"snapshots": snapshots, "unique_methods_by_tool_and_evaluator": combined}


def measure(artifact: Path, source: Path, source_root: str, *,
            cases: list[dict] | None = None, reports: list[dict] | None = None) -> dict:
    """Measure coverage without running a decompiler or creating a new quality verdict."""
    cases = cases or []
    reports = reports or []
    documents = [*cases, *(entry for report in reports for entry in inputs(report))]
    pinned_paths = {path for document in documents for row in rows(document) if (path := source_path(row))}
    try:
        original, methods = original_inventory(artifact)
    except (OSError, ValueError, BadZipFile, EOFError, NotImplementedError, CompressionError) as error:
        original, methods = {"complete": False, "unavailable": str(error)}, {}
    try:
        sources, pins = source_inventory(source, source_root, pinned_paths)
    except (OSError, ValueError, TarError, EOFError, CompressionError) as error:
        sources, pins = {"complete": False, "unavailable": str(error)}, {}
    selected, matches = selection(cases, original.get("sha256"), methods, pins)
    return {
        "schema_version": 1, "kind": "diana-decbench-coverage",
        "original": original, "source": sources,
        "selection": {**selected, "original_methods_without_a_matched_case":
                      len(methods) - len(matches) if original["complete"] and cases else None},
        "evidence": report_coverage(reports, original.get("sha256"), sources.get("sha256"), methods, pins),
        "scope": {
            "original": "Method definitions in supported root DEX ZIP entries, including methods without code.",
            "source": "Unique regular Java and Kotlin paths in the specified archive root, including other source sets.",
            "selection": "Discovery reports candidate denominators per manifest before final obligation verification. "
                         "Cases are matched to exact original locators, file hashes and source line bounds. "
                         "The archive file inventory alone cannot establish eligible methods.",
            "evidence": "Successful and failed decisions count as measured for their check family. "
                        "Structure receipts measure shape comparisons separately from quality checks. "
                        "Identity, publication and body recovery are also counted separately from quality checks.",
            "skipped": ["Whole-program equivalence", "Behavior without an executed model", "Methods without supplied receipts"],
        },
    }


def result_code(document):
    unavailable = (not document["original"]["complete"] or not document["source"]["complete"]
                   or any("unavailable" in item for item in document["selection"]["manifests"])
                   or any("unavailable" in snapshot or any("unavailable" in item for item in snapshot["inputs"])
                          for snapshot in document["evidence"]["snapshots"]))
    return 2 if unavailable else 0


def render(document):
    import reporting

    reporting.require_report(document, "diana-decbench-coverage")
    data = reporting.format_data
    headings = (
        ("Original DEX definitions", document["original"]),
        ("Source archive files", document["source"]),
        ("Selected methods", document["selection"]),
        ("Measured methods by producer", document["evidence"]["unique_methods_by_tool_and_evaluator"]),
        ("Scope", document["scope"]),
    )
    sections = "".join(f"<h2>{title}</h2><pre>{data(value)}</pre>" for title, value in headings)
    for snapshot in document["evidence"]["snapshots"]:
        sections += f"<details><summary>Run {snapshot['index']}</summary><pre>{data(snapshot)}</pre></details>"
    return reporting.html_page("Diana DecBench coverage", sections)
