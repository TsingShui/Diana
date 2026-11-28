"""Replay exact original DEX invokes with deterministic Object conversion hooks."""

from dex_witness import Fault, Raised, Ref


OWNER = "Lfixture/BuilderConversion;"
STRING = "Ljava/lang/String;"
OBJECT = "Ljava/lang/Object;"
FAMILIES = {"Ljava/lang/StringBuilder;", "Ljava/lang/StringBuffer;"}
ANCESTORS = ("Ljava/lang/RuntimeException;", "Ljava/lang/Exception;",
             "Ljava/lang/Throwable;", "Ljava/lang/Object;")


class Recorder:
    def __init__(self):
        self.events = []
        self.allocations = []
        self.builders = {}
        self.terms = {}
        self.faults = {
            label: Fault(label, f"Ljava/lang/{kind};", ANCESTORS)
            for label, kind in (
                ("first-producer", "IllegalArgumentException"),
                ("second-producer", "IllegalArgumentException"),
                ("first-conversion", "IllegalStateException"),
                ("second-conversion", "IllegalStateException"),
                ("unmatched", "UnsupportedOperationException"),
            )
        }

    def allocate(self, owner):
        if owner not in FAMILIES:
            raise ValueError(f"unexpected allocation {owner}")
        receiver = Ref(f"builder-{len(self.allocations)}", owner)
        self.allocations.append(receiver)
        return receiver

    def static_field(self, field):
        raise ValueError(f"unexpected field {field}")

    def produce(self, stage, mode):
        if stage not in (1, 2):
            raise ValueError("unexpected producer stage")
        self.events.append(f"produce:{stage}")
        if mode & (1 << (stage - 1)):
            raise Raised(self.faults["first-producer" if stage == 1 else "second-producer"])
        if stage == 2 and mode & 32:
            raise Raised(self.faults["unmatched"])
        if stage == 1 and mode & 16:
            return 0
        term = Ref(f"term-{len(self.terms)}", OBJECT)
        self.terms[term] = stage, mode
        return term

    def convert(self, term):
        if term == 0:
            return "null"
        stage, mode = self.terms[term]
        self.events.append(f"convert:{stage}")
        if mode & (1 << (stage + 1)):
            raise Raised(self.faults["first-conversion" if stage == 1 else "second-conversion"])
        return "left" if stage == 1 else "/right"

    def identity(self, fault):
        return next(label for label, original in self.faults.items() if fault is original)

    def call(self, member, receiver, args):
        owner, name, descriptor = member
        if owner in FAMILIES and receiver in self.allocations:
            if (name, descriptor) == ("<init>", "()V"):
                self.builders[receiver] = ""
                return None
            if (name, descriptor) == ("append", f"({OBJECT}){owner}"):
                term, = args
                self.builders[receiver] += self.convert(term)
                return receiver
            if (name, descriptor) == ("toString", f"(){STRING}"):
                return self.builders[receiver]
        if owner == OWNER and receiver is None:
            if (name, descriptor) == ("produce", f"(II){OBJECT}"):
                return self.produce(*args)
            if (name, descriptor) == ("caught", f"({STRING}Ljava/lang/Throwable;){STRING}"):
                path, fault = args
                self.events.append(f"caught:{path}:{self.identity(fault)}")
                return path
        raise ValueError(f"unexpected operation {member}")

    def finish(self, result, fault):
        self.events.append(f"escape:{self.identity(fault)}" if fault else f"result:{result}")
