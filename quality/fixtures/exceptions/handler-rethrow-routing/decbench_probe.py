"""Exercise original exception routes through nested cleanup clauses."""

from pathlib import Path

from corpus_models import fixture_method_probe


FIXTURE = Path(__file__).resolve().parent


def probe(binary, artifact, timeout):
    return fixture_method_probe(FIXTURE, FIXTURE / "HandlerRethrowRouting.java", binary, artifact, timeout)
