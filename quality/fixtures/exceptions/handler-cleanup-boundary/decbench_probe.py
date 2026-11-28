"""Bounded DecBench execution adapter for the authored DEX fixture."""

import hashlib
import json
import os
from pathlib import Path
import runpy
import subprocess
from tempfile import TemporaryDirectory


# Execute only the independently authored fixture's exact prepared bytes.
FIXTURE_SHA256 = "1ee4f0787fccf4a40c666dcdeb9c7475f128edc4685a03c287d005e7864e6069"
LOCATOR = "e0:l0:c0:m6"


def probe(binary: Path, artifact: Path, timeout: int) -> dict[str, list[dict]]:
    harness = runpy.run_path(str(Path(__file__).with_name("check.py")))
    outcomes = harness["outcomes"]
    try:
        data = artifact.read_bytes()
        if hashlib.sha256(data).hexdigest() != FIXTURE_SHA256:
            return {LOCATOR: outcomes("SETUP_ERROR", "input digest differs from the authored fixture")}
        with TemporaryDirectory() as scratch:
            fixed_input = Path(scratch, "classes.dex")
            fixed_input.write_bytes(data)
            result = subprocess.run(
                [
                    str(binary.resolve()), "get-method", str(fixed_input), LOCATOR,
                    "--format", "json",
                ],
                capture_output=True, check=False, timeout=timeout, encoding="utf-8",
                errors="replace",
                env={name: value for name, value in os.environ.items() if not name.startswith("DEXDEC_")},
            )
    except OSError as error:
        return {LOCATOR: outcomes("SETUP_ERROR", str(error))}
    except subprocess.TimeoutExpired as error:
        return {LOCATOR: outcomes("UNCHECKED", str(error))}
    try:
        publication = json.loads(result.stdout)
        if (
            result.returncode not in (0, 1)
            or publication["schema_version"] != 5
            or publication["command"] != "get-method"
            or publication["selection"]["method_at"].split("::", 1)[0] != LOCATOR
        ):
            raise ValueError("missing selected schema-5 method publication")
        source = publication["result"]["text"]
        if not isinstance(source, str):
            raise ValueError("selected publication has no text")
        if result.returncode or publication["issues"]:
            return {LOCATOR: outcomes("UNCHECKED", "selected method has local recovery issues")}
    except (KeyError, ValueError, TypeError, AttributeError) as error:
        return {LOCATOR: outcomes("SETUP_ERROR", f"method collection failed: {error}: {result.stderr.strip()}")}
    return {LOCATOR: harness["replay"](source, timeout)}
