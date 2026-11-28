"""Check original DEX and Java-shaped builder text and reference observations."""

import json
from pathlib import Path
import runpy

from corpus_models import attach_raw_controls, fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    manifest = json.loads((FIXTURE / "decbench.json").read_text(encoding="utf-8"))
    scenarios = json.loads((FIXTURE / "scenarios.json").read_text(encoding="utf-8"))
    raw = runpy.run_path(str(FIXTURE / "raw_proof.py"))["verify"](artifact, manifest, scenarios)
    results = fixture_method_probe(FIXTURE, FIXTURE / "BuilderIdentity.java", binary, artifact, timeout)
    for location, checks in results.items():
        attach_raw_controls(checks, {row["id"]: row for row in raw[location]})
    return results
