"""Independent reachability and effect checks for bounded raw DEX witnesses."""

from dataclasses import replace
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from zipfile import ZipFile

from dex_truth import methods
from dex_witness import (
    Execution, Fault, Instruction, MAX_STEPS, Program, Raised, Ref, Scenario, Unavailable, evaluate,
)
from evidence import file_digest

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures" / "exceptions" / "response-close-snapshots" / "classes.dex"
OWNER = "Lfixture/ResponseClose;"
MEMBER = OWNER + "->response(I)I"
HANDLE = Ref("handle", "Lfixture/ViewHandle;")
THROWABLE = "Ljava/lang/Throwable;"
OBJECT = "Ljava/lang/Object;"
EXCEPTION = "Ljava/lang/Exception;"
IO = Fault("io", "Ljava/io/IOException;", (EXCEPTION, THROWABLE, OBJECT))
INTERRUPTED = Fault("interrupted", "Ljava/io/InterruptedIOException;", (IO.descriptor, EXCEPTION, THROWABLE, OBJECT))
CLOSE = Fault("close", "Ljava/lang/IllegalStateException;", ("Ljava/lang/RuntimeException;", EXCEPTION, THROWABLE, OBJECT))


class ResponseFixture:
    def __init__(self, mode):
        self.mode = mode
        self.events = []

    def static_field(self, member):
        raise Unavailable(f"unmodeled field {member}")

    def allocate(self, owner):
        raise Unavailable(f"unmodeled owner {owner}")

    def call(self, member, receiver, arguments):
        if member == (OWNER, "acquire", "(I)" + HANDLE.descriptor) and arguments == (self.mode,):
            self.events.append("acquire")
            if self.mode == 4:
                raise Raised(IO)
            return HANDLE
        if member == (OWNER, "log", "(" + THROWABLE + ")V") and arguments == (IO,):
            self.events.append("log-io")
            return None
        if receiver != HANDLE:
            raise Unavailable("unexpected handle identity")
        if member == (HANDLE.descriptor, "ready", "()Z"):
            self.events.append("ready")
            return int(self.mode != 1)
        if member == (HANDLE.descriptor, "read", "()I"):
            self.events.append("read")
            if self.mode == 2:
                raise Raised(IO)
            if self.mode in (3, 5):
                raise Raised(INTERRUPTED)
            if self.mode == 6:
                raise Unavailable("fixture does not model this read")
            return 7
        if member == (HANDLE.descriptor, "close", "()V"):
            self.events.append("close")
            if self.mode == 5:
                raise Raised(CLOSE)
            return None
        raise Unavailable(f"unmodeled call {member}")

    def finish(self, value, fault):
        self.events.append("throw-" + fault.name if fault else f"return-{value}")


def behavior(mode, trace):
    return Scenario(str(mode), (mode,), lambda: ResponseFixture(mode), tuple(trace.split()))


class DexWitnessTest(unittest.TestCase):
    def run_fixture(self, scenarios, checksum=None, member=MEMBER):
        with TemporaryDirectory() as directory:
            artifact = Path(directory) / "input.apk"
            with ZipFile(artifact, "w") as archive:
                archive.write(FIXTURE, "classes.dex")
            at = next(at for at, name in methods(FIXTURE) if name == MEMBER)
            return evaluate(artifact, input_sha256=checksum or file_digest(artifact),
                            at=at, member=member, scenarios=scenarios)

    def test_branch_and_catch_order_from_existing_dex(self):
        rows = self.run_fixture([
            behavior(0, "acquire ready read close return-7"),
            behavior(1, "acquire ready close return--1"),
            behavior(2, "acquire ready read log-io close return--1"),
            behavior(3, "acquire ready read close throw-interrupted"),
            behavior(4, "acquire log-io return--1"),
            behavior(5, "acquire ready read close throw-close"),
            behavior(6, "acquire ready read"),
        ])
        self.assertEqual([row["status"] for row in rows], ["PASS"] * 6 + ["UNCHECKED"])
        traces = [row["trace"] for row in rows]
        self.assertTrue(any(site["op"] == "branch" for site in traces[1]))
        io = [item["clause"] for item in traces[2] if item["op"] == "catch"]
        self.assertEqual(io, [IO.descriptor])
        interrupted = [item["clause"] for item in traces[3] if item["op"] == "catch"]
        self.assertEqual(interrupted, [INTERRUPTED.descriptor, None])
        self.assertEqual(traces[5][-1]["fault"]["identity"], "close")
        self.assertFalse(any(item.get("member", [None]*3)[1] == "close" for item in traces[4]))
        self.assertIn("does not model this read", rows[6]["detail"])

    def test_inconsistent_raw_control_is_an_oracle_setup_error(self):
        scenario = behavior(1, "acquire ready close return--1")
        wrong = replace(scenario, expected=("acquire", "return-7"))
        result, = self.run_fixture([wrong])
        self.assertEqual(result["status"], "SETUP_ERROR")
        self.assertEqual(result["observed"], list(scenario.expected))
        self.assertIn("authored scenario", result["detail"])

    def test_original_physical_identity_and_digest_are_required(self):
        scenario = behavior(0, "acquire ready read close return-7")
        cases = [
            (self.run_fixture([scenario], checksum="0" * 64), "pinned"),
            (self.run_fixture([scenario], member=OWNER + "->resource(I)I"), "authored member"),
        ]
        for rows, detail in cases:
            with self.subTest(detail=detail):
                self.assertEqual(rows[0]["status"], "SETUP_ERROR")
                self.assertIn(detail, rows[0]["detail"])
                self.assertEqual(rows[0]["trace"], [])

    def test_may_not_follow_a_nonterminating_guest_beyond_its_bound(self):
        scenario = Scenario("cyclic", (), lambda: ResponseFixture(0), ())
        program = Program(None, 0, (), "V", {0: Instruction(0, 0x28, (), 0)}, ())
        run = Execution(program, scenario)
        with self.assertRaisesRegex(Unavailable, "step bound"):
            run.run()
        self.assertEqual(len(run.trace), MAX_STEPS)


if __name__ == "__main__":
    unittest.main()
