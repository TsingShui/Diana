"""Compare result identity, cleanup order, and exception suppression."""

from pathlib import Path

from corpus_models import fixture_method_probe

FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "ReturnLanes.java", binary, artifact, timeout)
