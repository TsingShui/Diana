"""Check the authored DEX first-use ordering against bounded recording hooks."""

import hashlib
from pathlib import Path
from tempfile import TemporaryDirectory
from zipfile import ZIP_STORED, ZipFile, ZipInfo

from corpus_oracles import code_instructions
from dex_calls import method_code, method_ref
from dex_witness import (
    Execution, Fault, Raised, Ref, Scenario, decode_instructions, program, register_words, show_value,
)
from dex_truth import Dex, U32


OWNER = "Lfixture/AllocationStages;"
ITEM = "Lfixture/AllocationItem;"
OBJECT_INIT = ("Ljava/lang/Object;", "<init>", "()V")
ITEM_INIT = (ITEM, "<init>", "(III)V")


def prove_item_bodies(dex):
    classes = {
        dex.descriptor(U32.unpack_from(dex.data, dex.classes.offset(index))[0]): index
        for index in range(dex.classes.count)
    }
    proven = set()
    for index in range(dex.method_ids.count):
        owner, name, descriptor = method_ref(dex, index)
        if owner != ITEM or name not in {"<clinit>", "<init>"}:
            continue
        code = method_code(dex, classes[owner], index, f"{owner}->{name}{descriptor}")
        start, count = code_instructions(dex, code)
        instructions = list(decode_instructions(dex, start, count).values())
        if name == "<init>":
            base_ctor, *instructions = instructions
            assert descriptor == "(III)V"
            assert base_ctor.opcode == 0x70 and register_words(base_ctor) == (0,)
            assert method_ref(dex, base_ctor.operands[0]) == OBJECT_INIT
            hook, registers = (OWNER, "construct", "(III)V"), (1, 2, 3)
        else:
            assert descriptor == "()V"
            hook, registers = (OWNER, "initialized", "()V"), ()
        invoke, done = instructions
        assert invoke.opcode == 0x71 and register_words(invoke) == registers
        assert method_ref(dex, invoke.operands[0]) == hook
        assert done.opcode == 0x0E
        proven.add(name)
    assert proven == {"<clinit>", "<init>"}


def runtime_fault(label, descriptor):
    return Fault(label, descriptor, ("Ljava/lang/RuntimeException;", "Ljava/lang/Exception;",
                                     "Ljava/lang/Throwable;", "Ljava/lang/Object;"))


class Recorder:
    def __init__(self, mode, values=(7, 11)):
        self.events = []
        self.operations = []
        self.mode = mode
        self.values = values
        self.initialized = False
        self.erroneous = False
        self.allocations = []
        self.constructed = set()
        self.faults = {
            "prepare": runtime_fault("before", "Ljava/lang/IllegalStateException;"),
            "init": Fault("class-init", "Ljava/lang/LinkageError;", ("Ljava/lang/Error;",
                          "Ljava/lang/Throwable;", "Ljava/lang/Object;")),
            "erroneous": Fault("erroneous", "Ljava/lang/NoClassDefFoundError;", ("Ljava/lang/LinkageError;",
                          "Ljava/lang/Error;", "Ljava/lang/Throwable;", "Ljava/lang/Object;")),
            "argument": runtime_fault("argument", "Ljava/lang/IllegalArgumentException;"),
            "constructor": runtime_fault("constructor", "Ljava/lang/UnsupportedOperationException;"),
            "publish": runtime_fault("publish", "Ljava/lang/ArithmeticException;"),
        }

    def static_field(self, member):
        raise ValueError(f"unexpected field {member}")

    def check_fault(self, phase):
        if phase == self.mode:
            raise Raised(self.faults[phase])

    def allocate(self, owner):
        self.operations.append({"op": "allocate", "owner": owner})
        if owner != ITEM:
            raise ValueError(f"unexpected allocation {owner}")
        if self.erroneous:
            raise Raised(self.faults["erroneous"])
        if not self.initialized:
            self.events.append("class-init")
            if self.mode == "init":
                self.erroneous = True
            self.check_fault("init")
            self.initialized = True
        instance = Ref(str(len(self.allocations)), owner)
        self.allocations.append(instance)
        self.operations[-1]["result"] = show_value(instance)
        return instance

    def call(self, member, receiver, arguments):
        self.operations.append({
            "op": "call", "member": list(member), "receiver": show_value(receiver),
            "arguments": [show_value(value) for value in arguments],
        })
        if member == ITEM_INIT:
            if receiver not in self.allocations:
                raise ValueError("unknown constructor receiver")
            prepared, argument, stage = arguments
            self.events.append(f"constructor:{prepared}:{argument}:{stage}")
            self.check_fault("constructor")
            self.constructed.add(receiver)
            return None
        owner, name, _ = member
        if owner != OWNER or receiver is not None:
            raise ValueError(f"unexpected invoke {member}")
        if name == "prepare":
            self.events.append("prepare")
            self.check_fault("prepare")
            return self.values[0]
        if name == "argument":
            stage, = arguments
            self.events.append(f"argument:{stage}")
            self.check_fault("argument")
            return self.values[1]
        if name == "publish":
            item, = arguments
            if item not in self.constructed:
                raise ValueError("publish precedes construction")
            self.events.append("publish")
            self.check_fault("publish")
            return None
        if name == "caught":
            stage, actual = arguments
            label = next(name for name, fault in self.faults.items() if actual is fault)
            self.events.append(f"caught:{stage}:{self.faults[label].name}")
            return None
        raise ValueError(f"unexpected fixture hook {name}")

    def finish(self, value, fault):
        if value is not None or fault is not None:
            raise ValueError("selected original escapes unexpectedly")


def run_cases(selected, scenarios, values=(7, 11)):
    outcomes = []
    for scenario in scenarios:
        mode, calls = scenario["arguments"]
        recorder = Recorder(mode, values)
        traces = []
        for invocation in range(2 if calls == "repeat" else 1):
            recorder.operations.append({"op": "entry", "invocation": invocation + 1})
            run = Scenario(scenario["id"], (), lambda: recorder, tuple(scenario["expected"]))
            execution = Execution(selected, run)
            traces.append(execution.run())
            recorder.finish(*execution.outcome)
            recorder.mode = "none"
        expected = [event.replace("constructor:7:11:", f"constructor:{values[0]}:{values[1]}:")
                    for event in scenario["expected"]]
        outcomes.append({
            "id": scenario["id"], "status": "PASS" if expected == recorder.events else "SETUP_ERROR",
            "expected": expected, "observed": recorder.events, "trace": traces,
            "operations": recorder.operations,
        })
    return outcomes


def prove(artifact, manifest, scenarios, *, values=((7, 11),)):
    with artifact.open("rb") as stream:
        data = stream.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or hashlib.sha256(data).hexdigest() != manifest["input_sha256"]:
        raise ValueError("prepared DEX differs from the authored allocation fixture")
    prove_item_bodies(Dex(data))
    with TemporaryDirectory(prefix="decbench-allocation-dex-") as temporary:
        archive = Path(temporary) / "original.zip"
        entry = ZipInfo("classes.dex", (1980, 1, 1, 0, 0, 0))
        entry.compress_type = ZIP_STORED
        with ZipFile(archive, "w") as wrapped:
            wrapped.writestr(entry, data)
        sha256 = hashlib.sha256(archive.read_bytes()).hexdigest()
        spec = manifest["methods"][0]
        # The retained DEX has the same logical/class/member coordinates in the one-entry ZIP.
        selected = program(archive, sha256, spec["at"], spec["member"])
        return {
            "at": spec["at"], "member": spec["member"], "sha256": manifest["input_sha256"],
            "raw_table": [{"start": row.start, "end": row.end, "clauses": row.clauses}
                          for row in selected.protection],
            "checks": [run_cases(selected, scenarios, pair) for pair in values],
        }
