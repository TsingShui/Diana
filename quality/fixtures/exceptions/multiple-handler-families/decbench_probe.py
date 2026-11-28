"""Execute the original and recovered paths through the shared protection."""

from pathlib import Path

from corpus_models import fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "MultipleHandlerFamilies.java", binary, artifact, timeout)
