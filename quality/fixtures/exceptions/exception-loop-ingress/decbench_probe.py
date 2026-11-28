"""Compare values and effects across catches sharing a loop continuation."""

from pathlib import Path

from corpus_models import fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "ExceptionLoopIngress.java", binary, artifact, timeout)
