"""Exercise authored and recovered guarded boolean returns."""

from pathlib import Path

from corpus_models import fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "GuardedBoolean.java", binary, artifact, timeout)
