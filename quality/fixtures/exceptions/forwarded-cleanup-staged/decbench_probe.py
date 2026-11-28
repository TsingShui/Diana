"""Check the selected cleanup path and the identity of its outward exception."""

from pathlib import Path

from corpus_models import fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "ForwardedCleanupStaged.java", binary, artifact, timeout)
