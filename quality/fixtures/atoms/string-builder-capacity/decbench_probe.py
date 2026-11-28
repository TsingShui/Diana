"""Compare the original DEX and Java-like output against independent scenarios."""

from hashlib import sha256
import json
from pathlib import Path
import runpy
from tempfile import TemporaryDirectory
from zipfile import ZipFile

from corpus_models import fixture_method_probe
from dex_witness import Scenario, evaluate
from evidence import file_digest


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    manifest = json.loads((FIXTURE / "decbench.json").read_bytes())
    scenarios = json.loads((FIXTURE / "scenarios.json").read_bytes())
    with artifact.open("rb") as stream:
        data = stream.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or sha256(data).hexdigest() != manifest["input_sha256"]:
        raise ValueError("the bounded fixture DEX differs from its manifest")

    observations = fixture_method_probe(FIXTURE, FIXTURE / "BuilderCapacity.java", binary, artifact, timeout)
    recorder = runpy.run_path(str(FIXTURE / "raw_model.py"))["Recorder"]
    with TemporaryDirectory(prefix="decbench-builder-capacity-") as directory:
        archive = Path(directory) / "original.zip"
        with ZipFile(archive, "w") as writer:
            writer.writestr("classes.dex", data)
        digest = file_digest(archive)
        for case in manifest["methods"]:
            at, member = case["at"], case["member"]
            name = member.split("->", 1)[1].split("(", 1)[0]
            expected = [
                Scenario(spec["id"], tuple(map(int, spec["arguments"][1:])), recorder, tuple(spec["expected"]))
                for spec in scenarios[name]
            ]
            raw = evaluate(archive, input_sha256=digest, at=at, member=member, scenarios=expected)
            for row, original in zip(observations[at], raw, strict=True):
                row["original_dex"] = original
                if original["status"] != "PASS":
                    row["status"] = original["status"]
                    row["detail"] = original["detail"]
    return observations
