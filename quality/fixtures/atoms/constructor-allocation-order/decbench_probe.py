"""Compare constructor allocation timing using the original method body and a trace model."""

from pathlib import Path

from corpus_models import fixture_method_probe

FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "ConstructorPrefix.java", binary, artifact, timeout)
