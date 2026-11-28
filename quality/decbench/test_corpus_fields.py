"""Verify field accessor evidence against checked-in independent fixture bytes."""

from hashlib import sha256
from pathlib import Path
import unittest

import corpus_fields
from corpus_oracles import Unavailable
from dex_calls import method_code
from dex_truth import Dex, U16

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"
BASIC = FIXTURES / "atoms/field-basic"
OWNER = "Lfixture/FieldBasic;"
READ = OWNER + "->readInstance()I"
WRITE = OWNER + "->writeInstance(I)V"


class FieldWitnessTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = (BASIC / "FieldBasic.java").read_bytes()
        cls.data = (BASIC / "classes.dex").read_bytes()
        cls.dex = Dex(cls.data)

    def case(self, member=READ, method_index=3, first=21, last=23, source=None):
        source = self.source if source is None else source
        return {"at": f"e0:l0:c0:m{method_index}", "member": member, "source": {
            "start_line": first, "end_line": last, "sha256": sha256(source).hexdigest(),
        }}

    def getter(self):
        return corpus_fields.verify(self.dex, self.case(), self.source)

    def setter(self):
        return corpus_fields.verify(self.dex, self.case(WRITE, 6, 25, 27), self.source)

    def check(self, witness, text):
        return corpus_fields.compare(witness, {"text": text})[0]

    def test_accessor_source_and_dex_agree_on_field_and_entry_receiver(self):
        getter, setter = self.getter(), self.setter()
        self.assertEqual(getter["original"], {"kind": "read", "receiver": "this", "owner": OWNER,
                                              "name": "value", "descriptor": "I", "pc": 0})
        self.assertEqual(setter["original"], {**getter["original"], "kind": "write"})
        self.assertEqual(self.check(getter, "public int readInstance() { return this.value; }")["status"],
                         "PASS")
        self.assertEqual(self.check(setter, "public void writeInstance(int value) { this.value = value; return; }")["status"],
                         "PASS")

    def test_unchanged_source_owner_and_pinned_span_are_required(self):
        wrong = self.source.replace(b"package fixture;", b"package different;")
        with self.assertRaisesRegex(Unavailable, "original declaring owner"):
            corpus_fields.verify(self.dex, self.case(source=wrong), wrong)
        with self.assertRaisesRegex(ValueError, "fingerprint"):
            corpus_fields.verify(self.dex, self.case(), wrong)

    def test_static_and_modified_receiver_are_unsupported(self):
        with self.assertRaisesRegex(Unavailable, "implicit instance receiver"):
            corpus_fields.original_accessor(self.dex, 0, 5, OWNER + "->readStatic()I")
        changed = bytearray(self.data)
        offset = method_code(self.dex, 0, 3, READ) + 16
        first = U16.unpack_from(changed, offset)[0]
        U16.pack_into(changed, offset, first & 0x0fff)
        with self.assertRaisesRegex(Unavailable, "entry receiver"):
            corpus_fields.original_accessor(Dex(bytes(changed)), 0, 3, READ)

    def test_unknown_names_inherited_receivers_and_aliases_are_unchecked(self):
        original = self.getter()
        for body in (
            "return this.renamed;",
            "return super.value;",
            "return other.value;",
            "int copy = this.value; return copy;",
            "if (ready) return this.value; return 0;",
            "try { return this.value; } finally { record(); }",
        ):
            with self.subTest(body=body):
                check = self.check(original, f"public int readInstance() {{ {body} }}")
                self.assertEqual(check["status"], "UNCHECKED")
                self.assertEqual(check["expected"], original["original"])

        self.assertEqual(self.check(self.setter(), "void writeInstance(int value) { value = value; }")["status"],
                         "UNCHECKED")

    def test_wrong_known_original_field_is_a_display_mismatch(self):
        fixture = FIXTURES / "generics/complex-generics"
        dex = Dex((fixture / "classes.dex").read_bytes())
        member = "Lfixture/ComplexGenerics$Pair;->first()Ljava/lang/Object;"
        original, fields = corpus_fields.original_accessor(dex, 2, 5, member)
        self.assertEqual((original["name"], fields["second"]), ("first", "Ljava/lang/Object;"))
        publication = "public Object first() { return this.second; }"
        result = self.check({"member": member, "original": original, "fields": fields}, publication)
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(result["observed"]["name"], "second")
