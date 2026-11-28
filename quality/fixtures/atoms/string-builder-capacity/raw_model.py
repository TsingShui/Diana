"""Interpret the exact retained DEX with explicit library and producer models."""

from dex_witness import Fault, Raised, Ref


OWNER = "Lfixture/BuilderCapacity;"
BUILDERS = {"Ljava/lang/StringBuilder;", "Ljava/lang/StringBuffer;"}
ANCESTORS = ("Ljava/lang/RuntimeException;", "Ljava/lang/Exception;",
             "Ljava/lang/Throwable;", "Ljava/lang/Object;")


class Recorder:
    def __init__(self):
        self.events = []
        self.allocations = []
        self.states = {}
        self.faults = {
            "negative-size": Fault("negative-size", "Ljava/lang/NegativeArraySizeException;", ANCESTORS),
            "first": Fault("first", "Ljava/lang/IllegalStateException;", ANCESTORS),
            "second": Fault("second", "Ljava/lang/IllegalStateException;", ANCESTORS),
            "other": Fault("other", "Ljava/lang/IllegalArgumentException;", ANCESTORS),
        }

    def allocate(self, owner):
        if owner not in BUILDERS:
            raise ValueError(f"unexpected builder owner {owner}")
        receiver = Ref(f"allocated-{len(self.allocations)}", owner)
        self.allocations.append(receiver)
        return receiver

    def static_field(self, member):
        raise ValueError(f"unexpected field {member}")

    def call(self, member, receiver, arguments):
        owner, name, descriptor = member
        if owner in BUILDERS and receiver in self.allocations:
            if (name, descriptor) == ("<init>", "(I)V"):
                capacity, = arguments
                if capacity < 0:
                    raise Raised(self.faults["negative-size"])
                self.states[receiver] = ""
                return None
            if (name, descriptor) == ("append", f"(Ljava/lang/String;){owner}"):
                value, = arguments
                self.states[receiver] += value
                return receiver
            if (name, descriptor) == ("toString", "()Ljava/lang/String;"):
                return self.states[receiver]
        if owner == OWNER and receiver is None:
            if (name, descriptor) == ("produce", "(II)Ljava/lang/String;"):
                stage, mode = arguments
                if stage not in (1, 2):
                    raise ValueError("unexpected producer argument")
                self.events.append(f"producer:{stage}")
                if stage == mode:
                    raise Raised(self.faults["first" if stage == 1 else "second"])
                if mode == 3 and stage == 1:
                    raise Raised(self.faults["other"])
                return "prefix" if stage == 1 else "/suffix"
            if (name, descriptor) == ("caught", "(Ljava/lang/String;Ljava/lang/Throwable;)Ljava/lang/String;"):
                path, fault = arguments
                self.events.append(f"caught:{path}:{self.identity(fault)}")
                return path
        raise ValueError(f"unsupported raw fixture hook {member}")

    def identity(self, actual):
        return next(label for label, fault in self.faults.items() if fault is actual)

    def finish(self, value, fault):
        if fault is not None:
            self.events.append(f"escape:{self.identity(fault)}")
        else:
            self.events.append(f"result:{value}")
