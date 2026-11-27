"""Assemble independently authored JVM declarations, code and JAR layouts."""

import io
from pathlib import Path
from struct import pack
import sys
from zipfile import ZIP_DEFLATED, ZIP_STORED, ZipFile


def u2(*items):
    return pack(">" + "H" * len(items), *items)


def u4(number):
    return pack(">I", number)


def mutf8(text):
    encoded = bytearray()
    units = text.encode("utf-16-be", errors="surrogatepass")
    for offset in range(0, len(units), 2):
        unit = int.from_bytes(units[offset : offset + 2], "big")
        if 0 < unit < 128:
            encoded.append(unit)
        elif unit < 2048:
            encoded.extend((0xC0 | (unit >> 6), 0x80 | (unit & 63)))
        else:
            encoded.extend((0xE0 | (unit >> 12), 0x80 | ((unit >> 6) & 63), 0x80 | (unit & 63)))
    return bytes(encoded)


class Class:
    def __init__(self, name, parent, *, access=0x0021, interfaces=(), major=49):
        self.pool = [b""]
        self.indices = {}
        self.self_class = self.class_ref(name)
        self.parent = self.class_ref(parent)
        self.access = access
        self.major = major
        self.interfaces = [self.class_ref(name) for name in interfaces]
        self.fields = []
        self.methods = []
        self.attributes = []

    def constant(self, tag, payload, slots=1):
        item = bytes((tag,)) + payload
        if item not in self.indices:
            self.indices[item] = len(self.pool)
            self.pool.append(item)
            if slots == 2:
                self.pool.append(b"")
        return self.indices[item]

    def utf8(self, text):
        encoded = mutf8(text)
        return self.constant(1, u2(len(encoded)) + encoded)

    def class_ref(self, name):
        return self.constant(7, u2(self.utf8(name)))

    def string(self, text):
        return self.constant(8, u2(self.utf8(text)))

    def reference(self, tag, owner, name, descriptor):
        signature = self.constant(12, u2(self.utf8(name), self.utf8(descriptor)))
        return self.constant(tag, u2(self.class_ref(owner), signature))

    def attribute(self, name, content):
        return u2(self.utf8(name)) + u4(len(content)) + content

    def member(self, access, name, descriptor, attributes):
        return u2(access, self.utf8(name), self.utf8(descriptor)) + sequence(attributes)

    def field(self, access, name, descriptor, constant=None):
        attributes = []
        if constant is not None:
            attributes.append(self.attribute("ConstantValue", u2(constant)))
        self.fields.append(self.member(access, name, descriptor, attributes))

    def method(self, access, name, descriptor, code=None, *, stack=0, locals=0):
        attributes = []
        if code is not None:
            contents = u2(stack, locals) + u4(len(code)) + code + u2(0, 0)
            attributes.append(self.attribute("Code", contents))
        self.methods.append(self.member(access, name, descriptor, attributes))

    def bytes(self):
        # Version 49 permits this small source to omit StackMapTable.
        return b"".join(
            (
                b"\xca\xfe\xba\xbe",
                u2(0, self.major),
                sequence(self.pool[1:], count=len(self.pool)),
                u2(self.access, self.self_class, self.parent),
                u2(len(self.interfaces), *self.interfaces),
                sequence(self.fields),
                sequence(self.methods),
                sequence(self.attributes),
            )
        )


def sequence(items, *, count=None):
    return u2(len(items) if count is None else count) + b"".join(items)


def make_main(message, *, damaged_member=False, damaged_code=False):
    owner = "fixture/jvm/Main"
    parent = "fixture/jvm/Base"
    result = Class(owner, parent, interfaces=("fixture/jvm/Marker",))
    hello = result.string(message)
    long_constant = result.constant(5, pack(">q", 0x100000001), slots=2)
    double_constant = result.constant(6, pack(">d", -0.0), slots=2)
    result.field(0x0019, "MESSAGE", "Ljava/lang/String;", hello)
    result.field(0x0019, "FIXTURE_LONG", "J", long_constant)
    result.field(0x0019, "NEGATIVE_ZERO", "D", double_constant)
    result.field(0x0002, "value", "I")
    parent_init = u2(result.reference(10, parent, "<init>", "()V"))
    value = u2(result.reference(9, owner, "value", "I"))
    twice = u2(result.reference(10, "fixture/jvm/Helper", "twice", "(I)I"))
    # Add a constant-pool-only method reference independent of the instructions.
    result.reference(10, "fixture/jvm/Helper", "unused", "()V")
    result.method(
        0x0001, "<init>", "()V",
        b"\x2a\xb7" + parent_init + b"\x2a\x06\xb5" + value + b"\xb1",
        stack=2, locals=1,
    )
    forward = b"\x1a\x99\x00\x08\x1a\xb8" + twice + b"\xac\x03\xac"
    if damaged_code:
        forward += b"\xcb"
    result.method(
        0x0009, "forward", "(I)I",
        forward,
        stack=1, locals=1,
    )
    result.method(0x0009, "forward", "(J)J", b"\x1e\xad", stack=2, locals=2)
    result.method(0x0009, "message", "()Ljava/lang/String;", b"\x13" + u2(hello) + b"\xb0", stack=1)
    result.method(0x0001, "read", "()I", b"\x2a\xb4" + value + b"\xac", stack=1, locals=1)
    if damaged_member:
        method = result.methods[1]
        result.methods[1] = method[:2] + u2(result.self_class) + method[4:]
    result.attributes.append(result.attribute("FixtureOpaque", b"\x00\xff"))
    return result.bytes()


def make_support():
    base = Class("fixture/jvm/Base", "java/lang/Object", access=0x0020)
    parent_init = u2(base.reference(10, "java/lang/Object", "<init>", "()V"))
    base.method(0, "<init>", "()V", b"\x2a\xb7" + parent_init + b"\xb1", stack=1, locals=1)
    marker = Class("fixture/jvm/Marker", "java/lang/Object", access=0x0600)
    marker.method(0x0401, "read", "()I")
    helper = Class("fixture/jvm/Helper", "java/lang/Object", access=0x0020)
    helper.method(0x0008, "twice", "(I)I", b"\x1a\x04\x78\xac", stack=2, locals=1)
    return base.bytes(), marker.bytes(), helper.bytes()


def make_archive(destination, *, damaged=False):
    main = make_main("root\0\U0001f331")
    overlay = make_main("overlay\0\U0001f331")
    base, marker, helper = make_support()
    with ZipFile(destination, "x") as output:
        output.writestr("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\nMulti-Release: true\r\n\r\n")
        output.writestr("fixture/jvm/Main.class", main, ZIP_DEFLATED)
        output.writestr("fixture/jvm/Base.class", base, ZIP_STORED)
        output.writestr("fixture/jvm/Marker.class", marker, ZIP_DEFLATED)
        output.writestr("misleading/NamedFromPath.class", helper, ZIP_STORED)
        output.writestr("META-INF/versions/17/fixture/jvm/Main.class", overlay, ZIP_DEFLATED)
        output.writestr("assets/demo/lang/en_us.json", b'{"message":"hello"}\n')
        if damaged:
            output.writestr("fixture/jvm/Short.class", b"\xca\xfe\xba\xbe\x00\x00\x00\x31\x00\x04\x01\x00")
            output.writestr("fixture/jvm/Damaged.class", helper, ZIP_STORED)
            output.writestr("fixture/jvm/After.class", make_named("fixture/jvm/After"), ZIP_DEFLATED)
    if damaged:
        # Corrupt the CRC in both headers of one otherwise readable class.
        data = bytearray(destination.read_bytes())
        name = b"fixture/jvm/Damaged.class"
        local = data.index(name) - 30
        directory = data.index(name, local + 30 + len(name)) - 46
        assert data[local : local + 4] == b"PK\x03\x04"
        assert data[directory : directory + 4] == b"PK\x01\x02"
        data[local + 14] ^= 1
        data[directory + 16] ^= 1
        destination.write_bytes(data)


def make_named(name):
    return Class(name, "java/lang/Object").bytes()


def make_bundle(destination, *, damaged=False):
    outer_class = Class("fixture/jvm/Outer", "java/lang/Object", access=0x0020)
    outer_class.method(0x0008, "increment", "(I)I", b"\x1a\x04\x60\xac", stack=2, locals=1)
    nested_class = Class("fixture/jvm/Nested", "java/lang/Object", access=0x0020)
    increment = u2(nested_class.reference(10, "fixture/jvm/Outer", "increment", "(I)I"))
    nested_class.method(0x0008, "probe", "(I)I", b"\x1a\xb8" + increment + b"\xac", stack=1, locals=1)
    payload = io.BytesIO()
    with ZipFile(payload, "w") as nested:
        nested.writestr("fixture/jvm/Nested.class", nested_class.bytes(), ZIP_DEFLATED)
    with ZipFile(destination, "x") as output:
        # The first column is part of the launcher's metadata format. Its value
        # has no role in the fixture's archive or class analysis.
        catalog = b"0" * 64 + b"\tdemo\tdemo/server.jar\n"
        if damaged:
            catalog += b"missing columns\n"
        output.writestr("META-INF/versions.list", catalog)
        output.writestr("META-INF/versions/demo/server.jar", payload.getvalue(), ZIP_DEFLATED)
        output.writestr("fixture/jvm/Outer.class", outer_class.bytes())
        output.writestr("assets/stray.jar", payload.getvalue())


def make_dynamic(*, damaged=False):
    result = Class("fixture/jvm/Dynamic", "java/lang/Object", major=55)
    owner = "fixture/jvm/Bootstrap"
    result_type = "Ljava/lang/String;"
    prefix = "(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;"
    string = result.string("nested-bootstrap-note")

    def handle(kind, name, descriptor):
        return result.constant(15, bytes((kind,)) + u2(result.reference(10, owner, name, descriptor)))

    produce_descriptor = prefix + "Ljava/lang/Class;" + result_type * 3 + ")" + result_type
    if damaged:
        produce = result.reference(10, owner, "produce", produce_descriptor)
    else:
        produce = handle(6, "produce", produce_descriptor)
    link = handle(6, "link", prefix + "Ljava/lang/invoke/MethodType;" + result_type + ")Ljava/lang/invoke/CallSite;")
    seed = handle(6, "seed", prefix + "Ljava/lang/Class;" + result_type + ")" + result_type)

    def dynamic(tag, bootstrap, name, descriptor):
        signature = result.constant(12, u2(result.utf8(name), result.utf8(descriptor)))
        return result.constant(tag, u2(bootstrap, signature))

    nested = dynamic(17, 2, "nested", result_type)
    load = dynamic(17, 0, "loaded", result_type)
    invoke = dynamic(18, 1, "called", "()" + result_type)
    result.attributes.append(result.attribute("BootstrapMethods", sequence((
        u2(produce, 3, string, nested, nested),
        u2(link, 1, nested),
        u2(seed, 1, string),
    ))))
    result.method(0x0009, "load", "()" + result_type, b"\x13" + u2(load) + b"\xb0", stack=1)
    result.method(0x0009, "invoke", "()" + result_type, b"\xba" + u2(invoke) + b"\x00\x00\xb0", stack=1)
    return result.bytes()


def prepare(directory):
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / "Main.class").open("xb") as output:
        output.write(make_main("root\0\U0001f331"))
    with (directory / "DamagedMember.class").open("xb") as output:
        output.write(make_main("root\0\U0001f331", damaged_member=True))
    with (directory / "DamagedCode.class").open("xb") as output:
        output.write(make_main("root\0\U0001f331", damaged_code=True))
    (directory / "Dynamic.class").write_bytes(make_dynamic())
    (directory / "DamagedDynamic.class").write_bytes(make_dynamic(damaged=True))
    make_archive(directory / "inventory.jar")
    make_archive(directory / "damaged.jar", damaged=True)
    make_bundle(directory / "bundle.jar")
    make_bundle(directory / "damaged-bundle.jar", damaged=True)


if __name__ == "__main__":
    prepare(Path(sys.argv[1]))
