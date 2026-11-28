"""Produce method-level JADX evidence for DecBench's physical DEX selectors."""

from __future__ import annotations

from collections import Counter, defaultdict
import json
from pathlib import Path
import re
import subprocess
from tempfile import TemporaryDirectory
from zipfile import BadZipFile, ZIP_DEFLATED, ZIP_STORED, ZipFile
from zlib import error as DeflateError

from dex_truth import APK_FILE_BYTES, APK_ROOT, APK_ROOT_BYTES, Dex, U32
from evidence import file_digest
from local_types import run

HELPER = Path(__file__).with_name("JadxMethods.java")
LOCATOR = re.compile(r"e(0|[1-9][0-9]*):l0:c(0|[1-9][0-9]*):m(0|[1-9][0-9]*)")


def unavailable(selection: dict, detail: str, *, setup_error: bool = False) -> dict:
    return {
        "at": selection["at"], "member": selection["member"], "identified": False,
        "status": "UNAVAILABLE", "complete": False, "has_body": False, "text": None, "source_span": None,
        "diagnostics": [detail],
        **({"setup_error": True} if setup_error else {}),
    }


def physical_requests(dex: Dex, input_name: str, methods: list[tuple[int, int, dict]], results: dict) -> list[dict]:
    requests = []
    classes = {}
    for class_index, method_index, selection in methods:
        try:
            offset = dex.classes.offset(class_index)
            if class_index not in classes:
                owner_id = U32.unpack_from(dex.data, offset)[0]
                owner = dex.descriptor(owner_id)
                start = U32.unpack_from(dex.data, offset + 24)[0]
                declarations = dex.class_methods(owner_id, start)[0] if start else []
                classes[class_index] = owner, dict(declarations), Counter(member for _, member in declarations)
            owner, declarations, counts = classes[class_index]
            if declarations.get(method_index) != selection["member"] or counts[selection["member"]] != 1:
                raise ValueError("selected physical DEX method does not have the requested unique original descriptor")
        except ValueError as error:
            results[selection["at"]] = unavailable(selection, str(error))
        else:
            requests.append({
                **selection,
                "physical": {"owner": owner, "input_name": input_name, "offset": offset},
            })
    return requests


def selected_requests(artifact: Path, selections: dict, results: dict) -> list[dict]:
    entries = defaultdict(list)
    for at, selection in selections.items():
        location = LOCATOR.fullmatch(at)
        if location is None:
            results[at] = unavailable(selection, "JADX coverage requires a physical DEX method locator with layout l0")
        else:
            entry, owner, method = map(int, location.groups())
            entries[entry].append((owner, method, selection))
    if not entries:
        return []
    extension = artifact.suffix.lower()
    size = artifact.stat().st_size
    if extension == ".dex" and size <= APK_ROOT_BYTES:
        for entry, methods in entries.items():
            if entry != 0:
                for _, _, selection in methods:
                    results[selection["at"]] = unavailable(selection, "standalone DEX uses physical entry e0")
        if 0 not in entries:
            return []
        with artifact.open("rb") as stream:
            dex = Dex(stream.read(APK_ROOT_BYTES + 1))
        return physical_requests(dex, str(artifact), entries[0], results)
    if extension != ".apk" or size > APK_FILE_BYTES:
        raise ValueError("JADX adapter covers bounded APK archives and standalone DEX files")
    requests = []
    with ZipFile(artifact) as archive:
        directory = archive.infolist()
        names = Counter(entry.orig_filename for entry in directory)
        for index, methods in entries.items():
            try:
                if index >= len(directory):
                    raise ValueError("physical APK entry is outside its ZIP directory")
                entry = directory[index]
                name = entry.orig_filename
                if (not APK_ROOT.fullmatch(name) or names[name] != 1 or entry.file_size > APK_ROOT_BYTES
                        or entry.flag_bits & 1 or entry.compress_type not in (ZIP_STORED, ZIP_DEFLATED)):
                    raise ValueError("physical APK entry is outside the supported unique root DEX entries")
                with archive.open(entry) as stream:
                    data = stream.read(APK_ROOT_BYTES + 1)
                if len(data) != entry.file_size:
                    raise ValueError("physical DEX size does not match the ZIP central directory")
                dex = Dex(data)
                requests.extend(physical_requests(dex, name, methods, results))
            except (OSError, ValueError, BadZipFile, DeflateError) as error:
                for _, _, selection in methods:
                    results[selection["at"]] = unavailable(selection, str(error))
    return requests


def parse_publication(path: Path, requested: dict, publication: dict) -> bool:
    if not path.is_file():
        return False
    valid = True
    seen = set()
    class_locations = {at.rsplit(":m", 1)[0] for at in requested}
    with path.open(encoding="utf-8") as source:
        for line in source:
            try:
                row = json.loads(line)
                if not isinstance(row, dict):
                    raise ValueError("expected an object")
                at = row.get("at")
                class_at = row.get("class_at")
                version = row.get("version")
                if isinstance(at, str) and at in requested:
                    if at in seen or row.get("member") != requested[at]["member"]:
                        valid = False
                        publication["methods"][at] = unavailable(
                            requested[at], "JADX returned conflicting method identities", setup_error=True,
                        )
                    elif (row.get("status") not in ("AVAILABLE", "UNAVAILABLE")
                          or not isinstance(row.get("identified"), bool)
                          or not isinstance(row.get("complete"), bool)
                          or not isinstance(row.get("has_body"), bool)
                          or not isinstance(row.get("diagnostics"), list)
                          or not all(isinstance(value, str) for value in row["diagnostics"])
                          or row["status"] == "AVAILABLE" and (
                              not row["identified"] or not isinstance(row.get("text"), str) or not row["text"]
                          )):
                        valid = False
                        publication["methods"][at] = unavailable(
                            requested[at], "invalid JADX method publication", setup_error=True,
                        )
                    else:
                        publication["methods"][at] = row
                    seen.add(at)
                elif isinstance(class_at, str) and class_at in class_locations and isinstance(row.get("text"), str):
                    publication["classes"][class_at] = row["text"]
                elif isinstance(version, str) and publication["provenance"]["version"] is None:
                    publication["provenance"]["version"] = version
                else:
                    raise ValueError("unexpected identity or duplicate event")
            except ValueError as error:
                valid = False
                publication["diagnostics"].append(f"could not read a JADX publication event: {error}")
    return valid and seen == requested.keys() and bool(publication["provenance"]["version"])


def worker_status(publication: dict, status: str, exit_code: int | None = None,
                  error: str | None = None) -> None:
    publication["worker"] = {"status": status, "exit_code": exit_code, "error": error}
    if error:
        publication["diagnostics"].append(error)


def stream_text(value: bytes | str | None) -> str:
    return value.decode("utf-8", "replace") if isinstance(value, bytes) else value or ""


def publish(jadx_jar: Path, artifact: Path, methods: list[dict], timeout: float) -> dict:
    """Return one result per locator and the worker's independent execution status.

    Setup errors mark only methods without usable protocol responses. A worker
    failure can coexist with available output and locally unavailable methods.
    """
    selections = {method["at"]: {"at": method["at"], "member": method["member"]} for method in methods}
    if len(selections) != len(methods):
        raise ValueError("JADX method selection repeats a physical locator")
    publication = {
        "provenance": {"tool": "jadx", "version": None, "jar": str(jadx_jar.resolve()), "sha256": None,
                       "input": str(artifact.resolve()), "input_sha256": None},
        "methods": {}, "classes": {}, "diagnostics": [], "stdout": "", "stderr": "",
    }
    worker_status(publication, "NOT_RUN")
    results = publication["methods"]
    try:
        jadx_jar = jadx_jar.resolve(strict=True)
        artifact = artifact.resolve(strict=True)
        publication["provenance"]["sha256"] = file_digest(jadx_jar)
        requests = selected_requests(artifact, selections, results)
        if requests:
            publication["provenance"]["input_sha256"] = file_digest(artifact)
            with TemporaryDirectory(prefix="decbench-jadx-") as directory:
                scratch = Path(directory)
                request_file = scratch / "selected.json"
                output_file = scratch / "publication.jsonl"
                request_file.write_text(json.dumps(requests, ensure_ascii=True), encoding="utf-8")
                argv = ["java", "-Xmx2g", f"-Djava.io.tmpdir={scratch}", "-cp", str(jadx_jar), str(HELPER),
                        str(artifact), str(request_file), str(output_file)]
                try:
                    worker = run(argv, scratch, timeout)
                    publication.update(stdout=worker.stdout, stderr=worker.stderr)
                    if worker.returncode:
                        worker_status(publication, "FAILED", worker.returncode,
                                      f"JADX worker exited with status {worker.returncode}")
                    else:
                        worker_status(publication, "COMPLETE", worker.returncode)
                except subprocess.TimeoutExpired as error:
                    publication.update(stdout=stream_text(error.stdout), stderr=stream_text(error.stderr))
                    worker_status(publication, "TIMED_OUT", error=f"JADX worker timed out after {timeout:g} seconds")
                requested = {item["at"]: item for item in requests}
                try:
                    valid = parse_publication(output_file, requested, publication)
                except (OSError, ValueError) as error:
                    valid = False
                    publication["diagnostics"].append(f"cannot read the JADX worker protocol: {error}")
                if not valid and publication["worker"]["status"] == "COMPLETE":
                    worker_status(publication, "PROTOCOL_ERROR", worker.returncode,
                                  "JADX exited successfully without a complete method response")
    except (OSError, ValueError, BadZipFile, DeflateError) as error:
        worker_status(publication, "SETUP_ERROR", error=str(error))
    worker_error = publication["worker"]["error"]
    for at, selection in selections.items():
        reason = "JADX produced no response for this method"
        if worker_error:
            reason += ": " + worker_error
        results.setdefault(at, unavailable(selection, reason, setup_error=worker_error is not None))
    publication["methods"] = {at: results[at] for at in selections}
    return publication
