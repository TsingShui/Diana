"""Execute the authored method and its recovered body against the same traces."""

from pathlib import Path

from corpus_models import fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "GuardedEffectJoin.java", binary, artifact, timeout)
