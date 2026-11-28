"""Read the schema-1 and schema-3 DEX publications of historical Diana builds."""

import json
import os
from pathlib import Path
import re
import subprocess
import time

import tool_capture


MAX_INVENTORY_BYTES = 64 * 1024 * 1024
MAX_METHOD_BYTES = 32 * 1024 * 1024
MAX_DIAGNOSTIC_BYTES = 256 * 1024
NUMBER = r"(0|[1-9][0-9]{0,9})"
LOCATOR = re.compile(rf"e{NUMBER}:l{NUMBER}:c{NUMBER}:m{NUMBER}")
OBJECT = r"L[^.;\[\]()\s]+;"
FIELD_TYPE = rf"\[*(?:[ZBSCIJFD]|{OBJECT})"
MEMBER = re.compile(rf"({OBJECT})->[^()\s]+\((?:{FIELD_TYPE})*\)(?:V|{FIELD_TYPE})")


def unsupported_unlimited(capture: dict) -> bool:
    """Recognize the historical parser's explicit rejection of the new flag."""
    stderr = capture.get("stderr")
    return (
        capture.get("exit_code") == 2 and "publication" not in capture
        and "--no-limit" in capture.get("argv", ())
        and isinstance(stderr, str)
        and stderr.splitlines()[:1] == ["error: unexpected argument '--no-limit' found"]
    )


def run_cli(argv: list[str], timeout: float, max_bytes: int) -> dict:
    started = time.perf_counter()
    environment = {key: value for key, value in os.environ.items() if not key.startswith("DEXDEC_")}
    try:
        result = tool_capture.run(argv, timeout=timeout, env=environment,
                                  output_limit=max_bytes, diagnostic_limit=MAX_DIAGNOSTIC_BYTES)
    except subprocess.TimeoutExpired:
        return {"exit_code": None, "error": f"historical Diana timed out after {timeout:g} seconds"}
    except (OSError, tool_capture.OutputLimit) as error:
        return {"exit_code": None, "error": str(error)}

    capture = {"exit_code": result.returncode, "stderr": result.stderr.decode("utf-8", "replace"),
               "elapsed_seconds": time.perf_counter() - started}
    try:
        publication = json.loads(result.stdout.decode("utf-8"))
        if not isinstance(publication, dict):
            raise ValueError("expected a JSON object")
    except (ValueError, RecursionError) as error:
        capture["error"] = f"invalid historical Diana publication: {error}"
    else:
        capture["publication"] = publication
    return capture


def inventory(binary: Path, path: Path, timeout: float) -> tuple[set[tuple[str, str]], dict]:
    argv = [str(binary), "methods", str(path), "--format", "json"]
    capture = run_cli(argv, timeout, MAX_INVENTORY_BYTES)
    capture["argv"] = argv
    capture["complete"] = False
    document = capture.get("publication", {})
    if (
        capture["exit_code"] != 0 or set(document) != {"schema_version", "command", "items"}
        or type(document.get("schema_version")) is not int or document["schema_version"] != 1
        or document.get("command") != "methods" or not isinstance(document.get("items"), list)
    ):
        capture.setdefault("error", "historical Diana did not deliver a complete schema-1 method inventory")
        return set(), capture

    methods = {}
    for item in document["items"]:
        if not isinstance(item, dict) or set(item) != {"at"} or not isinstance(item["at"], str):
            capture["error"] = "historical Diana inventory contains an invalid method row"
            return set(), capture
        at, _, member = item["at"].partition("::")
        if not LOCATOR.fullmatch(at) or not MEMBER.fullmatch(member):
            capture["error"] = "historical Diana inventory contains an invalid original method identity"
            return set(), capture
        if at in methods:
            capture["error"] = "historical Diana inventory repeats an original method locator"
            return set(), capture
        methods[at] = member
    capture["complete"] = True
    return set(methods.items()), capture


def selected_method(selection, path: Path, at: str, member: str) -> bool:
    if not isinstance(selection, dict) or selection.get("method_at") != f"{at}::{member}":
        return False
    entry, logical, definition, method = map(int, LOCATOR.fullmatch(at).groups())
    expected_entry = None if path.suffix.lower() == ".dex" else entry
    owner = MEMBER.fullmatch(member)[1]
    class_at = at.rsplit(":", 1)[0]
    cls = selection.get("class")
    return (
        "entry" in selection and selection["entry"] == expected_entry and selection.get("logical") == logical
        and isinstance(cls, dict) and cls.get("class_def") == definition and selection.get("method") == method
        and selection.get("class_at") == f"{class_at}::{owner}"
    )


def method(binary: Path, path: Path, at: str, timeout: float, *, member: str) -> dict:
    if not LOCATOR.fullmatch(at) or not MEMBER.fullmatch(member):
        return {"exit_code": None, "error": "historical Diana requires a physical DEX locator and full method identity"}
    argv = [str(binary), "get-method", str(path), f"{at}::{member}", "--format", "json"]
    capture = run_cli(argv, timeout, MAX_METHOD_BYTES)
    document = capture.get("publication", {})
    observation = {"argv": argv, "exit_code": capture["exit_code"]}
    for field in ("stderr", "elapsed_seconds"):
        if field in capture:
            observation[field] = capture[field]

    result = document.get("result")
    kind = result.get("kind") if isinstance(result, dict) else None
    product = result.get("product") if isinstance(result, dict) else None
    if (
        document.get("schema_version") != 3 or document.get("command") != "get-method"
        or not selected_method(document.get("selection"), path, at, member)
        or (kind, capture["exit_code"]) not in (("ready", 0), ("stopped", 1))
        or not isinstance(product, dict) or not isinstance(product.get("text"), str)
        or not isinstance(product.get("issues"), list) or any(not isinstance(item, dict) for item in product["issues"])
        or (kind == "stopped" and not isinstance(product.get("error"), str))
    ):
        observation["error"] = capture.get("error", "historical Diana did not deliver the selected schema-3 method")
        return observation
    observation.update(text=product["text"], issues=product["issues"])
    if kind == "stopped":
        observation["error"] = f"historical Diana returned an incomplete prefix: {product['error']}"
    return observation
