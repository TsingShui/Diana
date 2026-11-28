"""Exercise crossed overload selection with typed reference overloads."""

from pathlib import Path

from corpus_models import fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "CrossedNumerics.java", binary, artifact, timeout)
