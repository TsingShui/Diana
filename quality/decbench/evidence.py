"""Capture ordinary Diana CLI publications with their original physical identities."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def digest(value) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def file_digest(path: Path) -> str:
    if not path.is_file():
        raise ValueError(f"expected a regular file: {path}")
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        dir=path.parent, mode="w", encoding="utf-8", delete=False
    ) as stream:
        temporary = Path(stream.name)
        json.dump(value, stream, ensure_ascii=True, sort_keys=True)
        stream.write("\n")
    try:
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def run_cli(argv: list[str], timeout: float) -> dict:
    environment = {key: value for key, value in os.environ.items() if not key.startswith("DEXDEC_")}
    started = time.perf_counter()
    try:
        result = subprocess.run(
            argv, capture_output=True, env=environment, timeout=timeout, check=False,
        )
    except subprocess.TimeoutExpired:
        return {"exit_code": None, "error": f"command timed out after {timeout:g} seconds"}
    except OSError as error:
        return {"exit_code": None, "error": str(error)}
    capture = {"exit_code": result.returncode, "stderr": result.stderr.decode("utf-8", "replace"),
               "elapsed_seconds": time.perf_counter() - started}
    try:
        publication = json.loads(result.stdout.decode("utf-8"))
        if not isinstance(publication, dict):
            raise ValueError("expected a JSON object")
    except ValueError as error:
        capture.update(error=f"invalid CLI publication: {error}", stdout=result.stdout.decode("utf-8", "replace"))
    else:
        capture["publication"] = publication
    return capture


def inventory(binary: Path, path: Path, timeout: float) -> tuple[set, dict]:
    argv = [str(binary), "methods", str(path), "--no-limit", "--format", "json"]
    evidence = run_cli(argv, timeout)
    evidence["argv"] = argv
    document = evidence.get("publication", {})
    items = document.get("items")
    complete = document.get("complete", True)
    methods = set()
    if (
        evidence["exit_code"] not in (0, 1)
        or document.get("schema_version") not in (1, 2)
        or document.get("command") != "methods"
        or not isinstance(items, list)
        or not isinstance(complete, bool)
        or (evidence["exit_code"] == 1 and complete)
    ):
        evidence.setdefault("error", "method inventory is unavailable or has an unexpected schema")
    else:
        for item in items:
            if not isinstance(item, dict) or not all(
                isinstance(item.get(key), str) and item[key] for key in ("at", "member")
            ):
                evidence["error"] = "method inventory contains an invalid member identity"
                break
            methods.add((item["at"], item["member"]))
        if len({at for at, _ in methods}) != len(items):
            evidence["error"] = "method inventory repeats an original locator"
    evidence["complete"] = complete and "error" not in evidence
    return methods, evidence


def method(binary: Path, path: Path, at: str, timeout: int, *, member: str | None = None) -> dict:
    argv = [str(binary), "get-method", str(path), at, "--format", "json"]
    capture = run_cli(argv, timeout)
    publication = capture.get("publication", {})
    result = publication.get("result")
    selection = publication.get("selection")
    issues = publication.get("issues")
    selected = selection.get("method_at") if isinstance(selection, dict) else None
    published_member = selected.partition("::")[2] if isinstance(selected, str) else ""
    valid_member = not member or published_member in ("", member, member.partition("->")[0] + "->")
    observation = {"argv": argv, "exit_code": capture["exit_code"]}
    for field in ("stderr", "elapsed_seconds"):
        if field in capture:
            observation[field] = capture[field]
    if (
        publication.get("schema_version") != 5
        or publication.get("command") != "get-method"
        or not isinstance(selected, str)
        or selected.split("::", 1)[0] != at
        or not valid_member
        or not isinstance(result, dict)
        or not isinstance(result.get("text"), str)
        or not isinstance(issues, list)
        or any(not isinstance(issue, dict) for issue in issues)
        or capture["exit_code"] != (1 if issues else 0)
    ):
        observation["error"] = capture.get("error", "the selected schema-5 publication was unavailable")
    else:
        observation.update(text=result["text"], issues=issues)
    return observation
