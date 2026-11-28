"""Check elapsed state, cancel count and outward throwable identity."""

from pathlib import Path

from corpus_models import fixture_method_probe

FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "BackoffSplit.java", binary, artifact, timeout)
