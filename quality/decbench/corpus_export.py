"""Collect source-aligned method publications from one ordinary Diana export."""

from collections import Counter, defaultdict
from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
from subprocess import TimeoutExpired
from tempfile import TemporaryDirectory

from corpus_discover import method_declarations, source_environment, source_match
from corpus_oracles import BYTE_LIMIT, CALL_OWNER, Unavailable, View, logical_path
import evidence
import tool_capture


CLASS_AT = re.compile(r"e(\d+):l(\d+):c(\d+)")


def class_inventory(binary, apk, timeout):
    capture = evidence.run_cli([str(binary), "classes", str(apk), "--no-limit", "--format", "json"], timeout)
    result = capture.get("publication", {})
    items = result.get("items")
    if (capture["exit_code"] != 0 or result.get("schema_version") not in (1, 2)
            or result.get("command") != "classes" or result.get("complete", True) is not True
            or result.get("truncated", False) is not False or not isinstance(items, list)):
        raise ValueError(capture.get("error", "complete Diana class inventory is unavailable"))
    classes = {}
    for row in items:
        if not isinstance(row, dict):
            raise ValueError("invalid original class inventory row")
        at, owner = row.get("at"), row.get("descriptor")
        if not isinstance(at, str) or not CLASS_AT.fullmatch(at) or not isinstance(owner, str) or at in classes:
            raise ValueError("invalid or repeated original class identity")
        classes[at] = owner
    return classes


def export_report(root, classes, partial):
    paths, issues, failed = {}, defaultdict(list), set()

    report = root / "export-report.jsonl"
    if not report.is_file():
        if partial:
            raise ValueError("partial Diana export has no issue report")
        return paths, {}, failed

    with report.open("rb") as stream:
        while line := stream.readline(BYTE_LIMIT + 1):
            if len(line) > BYTE_LIMIT:
                raise ValueError("Diana export report row exceeds the scan bound")
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError("invalid Diana export report row")
            kind = row.get("kind")
            coordinate = (row.get("entry"), row.get("logical"), row.get("class_def"))
            if kind == "source":
                failed.add(coordinate)
            elif kind in ("class-path", "methods"):
                entry, image, index = coordinate
                at = f"e{entry}:l{image}:c{index}"
                owner = classes.get(at)
                if owner is None:
                    raise ValueError(f"export report references an unknown class {at}")
                if kind == "class-path":
                    expected = f"source-definitions/e{entry}-l{image}-c{index}.java"
                    units = row.get("descriptor_units")
                    encoded = owner.encode("utf-16-le")
                    descriptor = [int.from_bytes(encoded[i:i + 2], "little") for i in range(0, len(encoded), 2)]
                    if row.get("path") != expected or units != descriptor or at in paths:
                        raise ValueError(f"export path differs from its original class identity {at}")
                    paths[at] = root / expected
                else:
                    failures = row.get("failures")
                    if not isinstance(failures, list) or any(not isinstance(item, dict) for item in failures):
                        raise ValueError(f"invalid export issue list for {at}")
                    issues[at].extend(failures)
    return paths, issues, failed


def class_paths(root, classes, paths, failed):
    conventional = defaultdict(list)
    for at, owner in classes.items():
        entry, image, index = map(int, CLASS_AT.fullmatch(at).groups())
        if (entry, None, None) in failed or (entry, image, None) in failed or (entry, image, index) in failed:
            paths.pop(at, None)
        elif at not in paths:
            conventional[owner].append(at)
    for owner, locations in conventional.items():
        if len(locations) == 1 and owner.startswith("L") and CALL_OWNER.fullmatch(owner):
            try:
                relative = logical_path(owner[1:-1] + ".java")
            except ValueError:
                continue
            paths[locations[0]] = root / "sources" / relative
    return paths


def collect(root, path, owner, methods, owners, issues):
    if path is None:
        raise Unavailable("the original class has no unambiguous export file")
    if not path.resolve().is_relative_to(root):
        raise Unavailable("the export file resolves outside the export directory")
    if not path.is_file():
        raise Unavailable("the original class has no exported source file")
    with path.open("rb") as stream:
        data = stream.read(BYTE_LIMIT + 1)
    if len(data) > BYTE_LIMIT:
        raise Unavailable("the export class exceeds the scan bound")
    text = data.decode("utf-8")
    view = View(text, locate_tokens=True)
    environment = source_environment(view, owner)
    declarations = defaultdict(list)
    for declaration in method_declarations(view, environment[-1], include_declarations=True):
        declarations[declaration.name].append(declaration)
    failures = defaultdict(list)
    for issue in issues:
        failures[(issue.get("class"), issue.get("name"), issue.get("prototype"))].append(issue)

    observations = {}
    for at, member in methods:
        name, _, proto = member.split("->", 1)[1].partition("(")
        matched = source_match(declarations[name], member, environment, owners)
        notes = failures[owner, name, "(" + proto]
        if matched is None:
            observations[at] = {"error": "the export has no unique declaration with this full signature",
                                "issues": notes}
            continue
        start = view.offsets[matched.first_token][0]
        end = view.offsets[matched.bounds[1]][1]
        observation = {"text": text[start:end], "issues": notes, "fragment": True}
        if view.words[matched.bounds[0]] == ";":
            observation["has_body"] = False
        observations[at] = observation
    return observations


@dataclass(frozen=True)
class Publications:
    observations: dict
    inventory: dict
    members: dict
    binary_sha256: str
    input_sha256: str

    def select(self, cases):
        return ({case["at"]: self.observations[case["at"]] for case in cases if case["at"] in self.observations},
                {case["at"]: self.members[case["at"]] for case in cases if case["at"] in self.members},
                self.inventory)


def publish(binary: Path, apk: Path, cases, timeout, scratch: Path) -> Publications:
    binary_hash, input_hash = evidence.file_digest(binary), evidence.file_digest(apk)
    classes = class_inventory(binary, apk, timeout)
    originals, capture = evidence.inventory(binary, apk, timeout)
    if not capture["complete"]:
        raise ValueError(capture.get("error", "complete Diana method inventory is unavailable"))
    members = dict(originals)
    keys = {(case["at"].rsplit(":m", 1)[0], case["member"]) for case in cases}
    counts = Counter()
    for at, member in originals:
        key = (at.rsplit(":m", 1)[0], member)
        if key in keys:
            counts[key] += 1
    requested = defaultdict(list)
    observations = {}
    for case in cases:
        at, member = case["at"], case["member"]
        if members.get(at) == member:
            class_at = at.rsplit(":m", 1)[0]
            if counts[class_at, member] == 1:
                requested[class_at].append((at, member))
            else:
                observations[at] = {"error": "the original class repeats this full method signature"}

    with TemporaryDirectory(prefix="export-", dir=scratch) as temporary:
        root = Path(temporary) / "publication"
        argv = [str(binary), "export", str(apk), "-o", str(root)]
        env = {key: value for key, value in os.environ.items() if not key.startswith("DEXDEC_")}
        try:
            run = tool_capture.run(argv, timeout=timeout, output_limit=BYTE_LIMIT,
                                   diagnostic_limit=BYTE_LIMIT, env=env)
        except TimeoutExpired as error:
            raise ValueError(f"Diana export timed out after {timeout:g} seconds") from error
        if run.returncode not in (0, 1) or not (root / "sources").is_dir():
            raise ValueError("Diana export did not publish a source tree: " +
                             run.stderr.decode("utf-8", "replace")[:16_384])
        paths, issues, failed = export_report(root, classes, run.returncode == 1)
        paths = class_paths(root, classes, paths, failed)
        owners = set(classes.values())
        for class_at, methods in requested.items():
            try:
                observations.update(collect(root, paths.get(class_at), classes[class_at], methods,
                                            owners, issues.get(class_at, [])))
            except (OSError, ValueError, KeyError) as error:
                observations.update((at, {"error": str(error)}) for at, _ in methods)
    summary = {"complete": True, "scope": "whole APK inventory", "observed": len(members),
               "capture": "export", "export_exit_code": run.returncode,
               "export_diagnostics": run.stderr.decode("utf-8", "replace")[:16_384]}
    if evidence.file_digest(binary) != binary_hash or evidence.file_digest(apk) != input_hash:
        raise ValueError("the executable or APK changed during Diana export")
    return Publications(observations, summary, members, binary_hash, input_hash)
