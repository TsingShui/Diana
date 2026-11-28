"""Exercise first-use initialization and exception state through construction."""

import json
from pathlib import Path
import runpy

from corpus_models import fixture_method_probe
from evidence import method


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    manifest = json.loads((FIXTURE / "decbench.json").read_text(encoding="utf-8"))
    selected = manifest["methods"][0]
    captured = method(binary, artifact, selected["at"], timeout, member=selected["member"])
    text = captured.get("text", "")
    if "allocate-instance " not in text and "init-direct " not in text:
        return fixture_method_probe(FIXTURE, FIXTURE / "AllocationStages.java", binary, artifact, timeout)
    scenarios = json.loads((FIXTURE / "scenarios.json").read_text(encoding="utf-8"))["run"]
    explicit = runpy.run_path(str(FIXTURE / "explicit_recovery.py"))
    return {selected["at"]: explicit["compare_explicit"](
        FIXTURE, artifact, manifest, scenarios, captured, timeout,
    )}
