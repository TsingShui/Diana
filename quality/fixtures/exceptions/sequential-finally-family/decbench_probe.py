"""Check selected cleanup ordering and exception identity with recording stubs."""

from pathlib import Path

from corpus_models import fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "FinallyFamily.java", binary, artifact, timeout)
