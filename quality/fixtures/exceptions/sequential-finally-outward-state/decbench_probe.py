"""Check which state and throwable reach the enclosing handler."""

from pathlib import Path

from corpus_models import fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(
        FIXTURE, FIXTURE / "FinallyOutwardState.java", binary, artifact, timeout,
    )
