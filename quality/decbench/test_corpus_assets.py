from dataclasses import replace
from io import BytesIO
from pathlib import Path
import json
import tarfile
import tempfile
import unittest
from unittest.mock import patch
from zipfile import ZipFile

from corpus_assets import Asset, PROJECT_ID, load_projects, prepare_assets


class Download(BytesIO):
    def __init__(self, data, content_length=None):
        super().__init__(data)
        self.headers = {} if content_length is None else {"Content-Length": str(content_length)}


class CorpusAssetsTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.cache = self.root / "cache"
        self.project = load_projects()["antennapod-3.12.3"]
        self.apk = self.root / "local.apk"
        self.source = self.root / "local.tar.gz"
        with ZipFile(self.apk, "w") as archive:
            archive.writestr("classes.dex", b"APK archive sample")
        with tarfile.open(self.source, "w:gz") as archive:
            entry = tarfile.TarInfo(f"{self.project.source_root}/Example.java")
            contents = b"class Example {}"
            entry.size = len(contents)
            archive.addfile(entry, BytesIO(contents))

    def test_catalog_is_pinned_and_cache_is_explicitly_offline(self):
        self.assertEqual(self.project.provenance["version_code"], 3120395)
        self.assertIn(self.project.source_commit, self.project.source.url)
        self.assertIsNotNone(self.project.apk.sha256)
        with patch("corpus_assets.open_download") as network:
            with self.assertRaisesRegex(ValueError, "missing .*input.apk.*--download"):
                prepare_assets(self.project, self.cache)
            network.assert_not_called()
        self.assertFalse(self.cache.exists())

    def test_project_catalog_keeps_separate_case_manifests_out_of_asset_loading(self):
        path = self.root / self.project.manifest.name
        path.write_bytes(self.project.manifest.read_bytes())
        path.with_suffix(".cases.json").write_text("{\"methods\": []}", encoding="utf-8")
        self.assertEqual(load_projects(self.root)[self.project.id].manifest, path)

    def test_local_import_and_offline_reopen_copy_the_same_bytes(self):
        with patch("corpus_assets.open_download") as network:
            paths = prepare_assets(self.project, self.cache, apk=self.apk, source=self.source)
            self.assertEqual(prepare_assets(self.project, self.cache), paths)
            self.assertEqual(prepare_assets(self.project, self.cache, download=True), paths)
            network.assert_not_called()
        self.assertEqual(paths.apk.read_bytes(), self.apk.read_bytes())
        self.assertEqual(paths.source.read_bytes(), self.source.read_bytes())

    def test_failed_local_import_preserves_previously_cached_asset(self):
        paths = prepare_assets(self.project, self.cache, apk=self.apk, source=self.source)
        wrong = self.root / "wrong.apk"
        wrong.write_bytes(b"not an archive")
        with self.assertRaisesRegex(ValueError, "cannot acquire input.apk.*ZIP container"):
            prepare_assets(self.project, self.cache, apk=wrong)
        self.assertEqual(paths.apk.read_bytes(), self.apk.read_bytes())
        self.assertEqual(sorted(path.name for path in paths.apk.parent.iterdir()), ["input.apk", "source.tar.gz"])

    def test_opt_in_download_publishes_assets_only_after_validation(self):
        with patch("corpus_assets.open_download", side_effect=[
            Download(self.apk.read_bytes(), self.apk.stat().st_size),
            Download(self.source.read_bytes()),
        ]) as network:
            paths = prepare_assets(self.project, self.cache, download=True)
            self.assertEqual([call.args for call in network.call_args_list], [
                (self.project.apk.url,), (self.project.source.url,),
            ])
        self.assertEqual(paths.apk.read_bytes(), self.apk.read_bytes())
        self.assertEqual(paths.source.read_bytes(), self.source.read_bytes())

    def test_oversized_or_incomplete_download_leaves_no_cache_file(self):
        for data, length, message in (
            (self.apk.read_bytes(), 4096, "at most 256 bytes"),
            (b"X" * 4096, None, "exceeds the 256-byte limit"),
            (self.apk.read_bytes(), 255, "ended before the announced asset"),
        ):
            with self.subTest(message=message):
                project = replace(self.project, apk=Asset(self.project.apk.url, 256))
                with patch("corpus_assets.open_download", return_value=Download(data, length)):
                    with self.assertRaisesRegex(ValueError, message):
                        prepare_assets(project, self.cache, download=True)
                self.assertEqual(list((self.cache / project.id).iterdir()), [])

    def test_catalog_rejects_escape_paths_insecure_urls_and_unpinned_source(self):
        self.assertIsNone(PROJECT_ID.fullmatch("../elsewhere"))
        catalog = self.root / "manifests"
        catalog.mkdir()
        path = catalog / self.project.manifest.name
        original = json.loads(self.project.manifest.read_text(encoding="utf-8"))
        for section, field, bad, message in (
            (None, "id", "../other", "project ID must match"),
            (None, "source_root", "root/../elsewhere", "single archive root"),
            ("apk", "url", "http://example.com/app.apk", "must be HTTPS"),
            ("source", "url", "https://example.com/archive/main.tar.gz", "pinned commit"),
        ):
            with self.subTest(field=field):
                value = json.loads(json.dumps(original))
                target = value if section is None else value[section]
                target[field] = bad
                path.write_text(json.dumps(value), encoding="utf-8")
                with self.assertRaisesRegex(ValueError, message):
                    load_projects(catalog)


if __name__ == "__main__":
    unittest.main()
