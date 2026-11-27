"""Build closely related class files with independent instruction layouts."""

import io
from pathlib import Path
from struct import pack
import sys
from zipfile import ZIP_DEFLATED, ZIP_STORED, ZipFile

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "physical-inventory"))
from prepare import Class, sequence, u2, u4


OWNER = "fixture/diff/Methods"
STRING = "Ljava/lang/String;"


def method(unit, name, descriptor, instructions, *, stack=0, locals_=0, handlers=()):
    contents = (
        u2(stack, locals_) + u4(len(instructions)) + instructions
        + sequence([u2(*handler) for handler in handlers]) + u2(0)
    )
    unit.methods.append(unit.member(0x0009, name, descriptor, [unit.attribute("Code", contents)]))


def ldc(index):
    return bytes([0x12, index]) if index < 256 else b"\x13" + u2(index)


def switch(changed):
    # iload_0 at 0, lookupswitch at 1, two bytes of padding, four bodies at 36.
    rows = [(-7, 36), (6, 38), (23, 42)]
    default = 42 if changed else 40
    return (
        b"\x1a\xab\x00\x00"
        + pack(">ii", default - 1, len(rows))
        + b"".join(pack(">ii", key, target - 1) for key, target in rows)
        + b"\x03\xac\x04\xac\x05\xac\x06\xac"
    )


def make_methods(*, padded=False, changed=False, damaged=False):
    unit = Class(OWNER, "java/lang/Object", major=49)
    if padded:
        for ordinal in range(275):
            unit.utf8(f"Unused index {ordinal}")
    same = unit.string("shared\0\U0001f331")
    method(unit, "same", "()" + STRING, ldc(same) + b"\xb0", stack=1)
    text = unit.string("after" if changed else "before")
    method(unit, "message", "()" + STRING, ldc(text) + b"\xb0", stack=1)

    absolute = unit.reference(10, "java/lang/Math", "abs", "(I)I")
    path = 11 if changed else 9
    method(unit, "branch", "(I)I",
           b"\x1a\x99" + u2(path - 1) + b"\x1a\xb8" + u2(absolute)
           + b"\xac\x02\xac\x03\xac", stack=1, locals_=1)
    method(unit, "route", "(I)I", switch(changed), stack=1, locals_=1)

    npe = unit.class_ref("java/lang/NullPointerException")
    throwable = unit.class_ref("java/lang/Throwable")
    handlers = [(0, 2, 2, npe), (1, 2, 5, throwable)]
    if changed:
        handlers.reverse()
    method(unit, "caught", "()I", b"\x01\xbf\x57\x04\xac\x57\x05\xac",
           stack=1, handlers=handlers)

    generic = "Ljava/lang/Integer;" if changed else STRING
    exception = "java/lang/IllegalStateException" if changed else "java/io/IOException"
    attributes = [
        unit.attribute("Signature", u2(unit.utf8(f"()Ljava/util/List<{generic}>;"))),
        unit.attribute("Exceptions", u2(1, unit.class_ref(exception))),
    ]
    unit.methods.append(unit.member(0x0109, "generic", "()Ljava/util/List;", attributes))

    # The malformed suffix is after a known changed-site prefix.
    code = (b"\x08" if changed else b"\x07") + b"\xac"
    if damaged:
        code += b"\xcb"
    method(unit, "partial", "()I", code, stack=1)
    return unit.bytes()


def make_linkage(*, padded=False, changed=False):
    unit = Class("fixture/diff/Linkage", "java/lang/Object", major=55)
    if padded:
        for ordinal in range(275):
            unit.utf8(f"Reserved for offset shift {ordinal}")
    owner = "fixture/diff/Bootstrap"
    lookup = "Ljava/lang/invoke/MethodHandles$Lookup;"
    type_ = "Ljava/lang/Class;"
    bootstrap = unit.constant(15, b"\x06" + u2(unit.reference(
        10, owner, "value", f"({lookup}{STRING}{type_}{STRING}){STRING}",
    )))
    name = unit.constant(12, u2(unit.utf8("result"), unit.utf8(STRING)))
    unused = unit.string("unused")
    argument = unit.string("new argument" if changed else "old argument")
    node = unit.constant(17, u2(1 if padded else 0, name))
    other = len(unit.pool)
    unit.pool.append(unit.pool[node])
    columns = [u2(bootstrap, 1, argument), u2(bootstrap, 1, unused)]
    if padded:
        columns.reverse()
    unit.attributes.append(unit.attribute("BootstrapMethods", sequence(columns)))

    method(unit, "argument", "()" + STRING, ldc(node) + b"\xb0", stack=1)
    # This method exposes a sharing change even when the bootstrap arguments
    # agree within one version. The after input resolves a second CP entry.
    method(unit, "sharing", "()" + STRING,
           ldc(node) + b"\x57" + ldc(other if changed else node) + b"\xb0", stack=1)
    return unit.bytes()


def archive(file, rows, stored=False):
    with ZipFile(file, "w", compression=ZIP_STORED if stored else ZIP_DEFLATED) as jar:
        for name, value in rows:
            jar.writestr(name, value)


def prepare(directory):
    directory.mkdir(parents=True, exist_ok=True)
    old = make_methods()
    repacked = make_methods(padded=True)
    new = make_methods(padded=True, changed=True)
    damaged = make_methods(padded=True, changed=True, damaged=True)
    (directory / "old.class").write_bytes(old)
    (directory / "width.class").write_bytes(repacked)
    (directory / "changed.class").write_bytes(new)
    (directory / "damaged.class").write_bytes(damaged)
    before_linkage = make_linkage()
    repacked_linkage = make_linkage(padded=True)
    after_linkage = make_linkage(padded=True, changed=True)
    (directory / "linkage-old.class").write_bytes(before_linkage)
    (directory / "linkage-width.class").write_bytes(repacked_linkage)
    (directory / "linkage-changed.class").write_bytes(after_linkage)
    archive(directory / "old.jar", [("original.class", old), ("Linkage.class", before_linkage)])
    archive(directory / "changed.jar", [("readme.txt", b"resource"), ("Linkage.class", after_linkage),
                                       ("renamed.class", new)], stored=True)
    archive(directory / "damaged.jar", [("broken.class", b"\xca\xfe\xba\xbe"),
                                       ("partial.class", damaged),
                                       ("Linkage.class", after_linkage)])
    archive(directory / "duplicates-old.jar", [("A.class", old), ("B.class", old)])
    archive(directory / "duplicates-new.jar", [("B.class", old), ("A.class", old)], stored=True)
    archive(directory / "empty.zip", [("resource.txt", b"file")])

    for variant, rows in [("old", [("A.class", old)]), ("new", [("B.class", new)])]:
        nested = io.BytesIO()
        archive(nested, rows)
        archive(directory / f"bundle-{variant}.jar", [
            ("fabric.mod.json", b'{"jars":[{"file":"lib.jar"}]}'),
            ("lib.jar", nested.getvalue()),
        ])


if __name__ == "__main__":
    prepare(Path(sys.argv[1]))
