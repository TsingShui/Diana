"""Replay only the exact authored DEX with the independent DecBench evaluator."""

from dex_witness import Fault, Raised, Ref


OWNER = "Lfixture/ArrayOrder;"
PROTECTED_OWNER = "Lfixture/ArrayOrderProtected;"
OBJECT = "Ljava/lang/Object;"
THROWABLE = "Ljava/lang/Throwable;"
ALLOCATION_FAULT = Fault("allocation", "Ljava/lang/OutOfMemoryError;", (
    "Ljava/lang/VirtualMachineError;", "Ljava/lang/Error;", THROWABLE, OBJECT,
))
OPERATION_FAULT = Fault("operation", "Ljava/lang/RuntimeException;", (
    "Ljava/lang/Exception;", THROWABLE, OBJECT,
))
FIRST = Ref("first", OBJECT)
SECOND = Ref("second", OBJECT)


class ArrayModel:
    def __init__(self, failing_event):
        self.failing_event = failing_event
        self.fault = ALLOCATION_FAULT if failing_event.startswith("allocate:") else OPERATION_FAULT
        self.events = []
        self.arrays = {}
        self.published = None

    def step(self, event):
        self.events.append(event)
        if event == self.failing_event:
            raise Raised(self.fault)

    def allocate_array(self, descriptor, length):
        expected = {"[I": ("int", 2), "[Ljava/lang/Object;": ("object", 3)}
        name, expected_length = expected[descriptor]
        if length != expected_length:
            raise ValueError("unexpected modeled array length")
        self.step(f"allocate:{name}:{length}")
        array = Ref(f"array-{len(self.arrays)}", descriptor)
        self.arrays[array] = [0] * length
        return array

    def store_array(self, array, index, value):
        self.arrays[array][index] = value

    def marker(self, reference):
        if reference == 0:
            return "null"
        if reference not in (FIRST, SECOND):
            raise ValueError("unexpected marker identity")
        return reference.name

    def references(self, array):
        first, second, alias = self.arrays[array]
        if first != alias:
            raise ValueError("array does not preserve the shared original reference")
        return ":".join(map(self.marker, (first, second, alias)))

    def call(self, member, receiver, arguments):
        if receiver is not None:
            raise ValueError("unexpected original receiver")
        owner, name, descriptor = member
        if member == (OWNER, "next", "(I)I"):
            tag, = arguments
            self.step(f"next:{tag}")
            return tag * 10 + 1
        if owner in (OWNER, PROTECTED_OWNER) and (name, descriptor) == ("make", f"(I){OBJECT}"):
            stage, = arguments
            self.step(f"make:{stage}")
            return {10: FIRST, 20: SECOND}[stage]
        if member == (OWNER, "publish", f"([{OBJECT})V"):
            array, = arguments
            self.step(f"publish:{self.references(array)}")
            self.published = array
            return None
        if member == (OWNER, "caught", f"(I{OBJECT}{OBJECT}{THROWABLE})V"):
            stage, first, second, error = arguments
            if error is not self.fault:
                raise ValueError("the staged catch did not preserve the fault identity")
            self.step(f"caught:{stage}:{self.marker(first)}:{self.marker(second)}:same-fault")
            return None
        if member == (PROTECTED_OWNER, "caught", "(ILjava/lang/RuntimeException;)V"):
            stage, error = arguments
            if error is not self.fault:
                raise ValueError("the typed catch did not preserve the fault identity")
            self.step(f"caught:{stage}:same-fault")
            return None
        raise ValueError(f"unexpected original invoke {member}")

    def finish(self, value, fault):
        if fault is not None:
            if fault is not self.fault:
                raise ValueError("unexpected escaped exception identity")
            self.step("escape:same-fault")
        elif value == 0:
            self.step("return:null")
        elif value.descriptor == "[I":
            self.step("return:" + ":".join(map(str, self.arrays[value])))
        else:
            if self.published is not None and self.published != value:
                self.step("different-return-array")
            self.step(f"return:{self.references(value)}")
