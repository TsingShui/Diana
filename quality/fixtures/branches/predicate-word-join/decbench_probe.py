"""Execute predicate and integer joins against authored integer traces."""

from pathlib import Path

from corpus_models import fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "PredicateWordJoin.java", binary, artifact, timeout)
