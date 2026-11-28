"""Independent smoke expectations and malformed inputs for the DEX inventory oracle."""

from pathlib import Path
from struct import pack_into, unpack_from
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch
from warnings import catch_warnings, simplefilter
from zipfile import ZIP_DEFLATED, ZIP_STORED, ZipFile

from dex_truth import ApkMalformed, ApkUnsupported, DexMalformed, DexUnsupported, apk_methods, display, methods


FIXTURES = Path(__file__).resolve().parent.parent / "fixtures"
CLEANUP = FIXTURES / "exceptions" / "handler-cleanup-boundary" / "classes.dex"


class DexTruthTests(unittest.TestCase):
    def test_exact_definitions_and_prototypes(self):
        self.assertEqual(
            methods(CLEANUP),
            [
                ("e0:l0:c0:m0", "Lfixture/HandlerCleanupBoundary;->acquire(I)I"),
                ("e0:l0:c0:m1", "Lfixture/HandlerCleanupBoundary;->audit(I)V"),
                ("e0:l0:c0:m2", "Lfixture/HandlerCleanupBoundary;->close(I)V"),
                ("e0:l0:c0:m3", "Lfixture/HandlerCleanupBoundary;->log(Ljava/lang/Throwable;)V"),
                ("e0:l0:c0:m4", "Lfixture/HandlerCleanupBoundary;->probe()V"),
                ("e0:l0:c0:m5", "Lfixture/HandlerCleanupBoundary;->read(I)I"),
                ("e0:l0:c0:m6", "Lfixture/HandlerCleanupBoundary;->run(I)I"),
            ],
        )

    def test_unreferenced_definitions_are_not_inferred_from_id_table(self):
        data = bytearray(CLEANUP.read_bytes())
        class_def = unpack_from("<I", data, 0x64)[0]
        pack_into("<I", data, class_def + 24, 0)
        self.assertEqual(methods(self.write(data)), [])

    def test_truncated_image_and_invalid_class_data_are_malformed(self):
        data = bytearray(CLEANUP.read_bytes())
        with self.assertRaisesRegex(DexMalformed, "truncated"):
            methods(self.write(data[:-1]))
        class_def = unpack_from("<I", data, 0x64)[0]
        pack_into("<I", data, class_def + 24, len(data))
        with self.assertRaisesRegex(DexMalformed, "ULEB128"):
            methods(self.write(data))

    def test_container_and_extended_image_are_unsupported(self):
        data = bytearray(CLEANUP.read_bytes())
        with self.assertRaisesRegex(DexUnsupported, "bytes after"):
            methods(self.write(data + b"extra"))
        data[4:7] = b"041"
        with self.assertRaisesRegex(DexUnsupported, "041"):
            methods(self.write(data))

    def test_modified_utf8_supplementary_character(self):
        fixture = FIXTURES / "branches" / "mutable-tail-guards" / "classes.dex"
        data = bytearray(fixture.read_bytes())
        original = b"\x08extended\0"
        offset = data.index(original)
        replacement = b"\x04\xed\xa0\x81\xed\xb0\x80go\0"
        self.assertEqual(len(replacement), len(original))
        data[offset:offset + len(original)] = replacement
        self.assertIn(
            ("e0:l0:c1:m1", "Lfixture/MutableGuardRoutes;->\U00010400go(II)I"),
            methods(self.write(data)),
        )

    def test_lone_surrogates_and_controls_have_reversible_display(self):
        self.assertEqual(display("A\uD801\\\t\r\n\0"), r"A\ud801\\\t\r\n\0")

    def test_apk_compares_physical_entries_including_duplicate_root_names(self):
        # The first two entries are unrelated, and nested or invalid root
        # names are intentionally unreadable to catch incorrect root selection.
        path = self.apk([
            ("AndroidManifest.xml", b"manifest fixture", ZIP_STORED),
            ("assets/classes.dex", b"not a root", ZIP_STORED),
            ("classes.dex", CLEANUP.read_bytes(), ZIP_DEFLATED),
            ("classes01.dex", b"not a root", ZIP_STORED),
            ("classes2.dex", CLEANUP.read_bytes(), ZIP_STORED),
            ("classes2.dex", CLEANUP.read_bytes(), ZIP_DEFLATED),
        ])
        first = ("l0:c0:m0", "Lfixture/HandlerCleanupBoundary;->acquire(I)I")
        self.assertEqual(
            [row for row in apk_methods(path) if row[1] == first[1]],
            [(f"e{entry}:{first[0]}", first[1]) for entry in (2, 4, 5)],
        )
        self.assertEqual(len(apk_methods(path)), 21)

    def test_apk_does_not_publish_a_complete_inventory_when_one_root_is_malformed(self):
        path = self.apk([
            ("classes.dex", CLEANUP.read_bytes(), ZIP_STORED),
            ("classes2.dex", b"not DEX bytes", ZIP_STORED),
        ])
        with self.assertRaisesRegex(DexMalformed, "e1 classes2.dex: missing DEX magic"):
            apk_methods(path)

    def test_bad_zip_and_oversized_root_get_different_classifications(self):
        wrong = self.write(b"broken ZIP", name="bad.apk")
        with self.assertRaisesRegex(ApkMalformed, "ZIP directory"):
            apk_methods(wrong)
        valid = self.apk([("classes.dex", CLEANUP.read_bytes(), ZIP_DEFLATED)])
        with patch("dex_truth.APK_ROOT_BYTES", 1):
            with self.assertRaisesRegex(ApkUnsupported, "e0 classes.dex:.*bounds"):
                apk_methods(valid)

    def test_rootless_zip_is_outside_the_apk_oracles_scope(self):
        path = self.apk([("assets/classes.dex", CLEANUP.read_bytes(), ZIP_STORED)])
        with self.assertRaisesRegex(ApkUnsupported, "no root"):
            apk_methods(path)

    def apk(self, entries):
        path = self.write(b"", name="input.apk")
        with catch_warnings():
            simplefilter("ignore", UserWarning)  # The fixture uses duplicate physical ZIP records.
            with ZipFile(path, "w") as archive:
                for name, data, compression in entries:
                    archive.writestr(name, data, compression)
        return path

    def write(self, payload, name="classes.dex"):
        temp = TemporaryDirectory(prefix="dex-truth-test-")
        self.addCleanup(temp.cleanup)
        path = Path(temp.name) / name
        path.write_bytes(payload)
        return path


if __name__ == "__main__":
    unittest.main()
