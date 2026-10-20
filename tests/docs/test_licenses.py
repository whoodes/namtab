"""Challenge actual notice bytes, source boundaries and static-asset coverage."""
from pathlib import Path
import importlib.util
import json
import shutil
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('licensed_docs', Path(__file__).resolve().parents[2] / 'scripts/public-docs.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class LicenseArtifactTests(unittest.TestCase):
    def setUp(self):
        (module.ROOT / '.artifacts').mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=module.ROOT / '.artifacts', prefix='license-test-')
        self.root = Path(self.temp.name)
        self.site = self.root / 'site'
        self.site.mkdir()
        (self.site / 'search').mkdir()
        (self.site / 'index.html').write_text('<html>Original documentation.</html>')
        (self.site / 'search/search_index.json').write_text('{"docs": []}')
        self.inventory = module.docs_licenses()
        for item in self.inventory['components']:
            dest = self.site / item['file']
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(module.ROOT / 'docs' / item['file'], dest)
        package = Path(module.mkdocs.__file__).parent.parent
        for item in self.inventory['assets']:
            dest = self.site / item['file']
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(package / item['wheel_member'], dest)

    def tearDown(self):
        self.temp.cleanup()

    def test_complete_pinned_bundle_passes_both_guards(self):
        self.assertEqual(module.audit_licenses(self.site), [])
        self.assertEqual(module.audit_site(self.site)[1], [])
        self.assertEqual((self.site / 'licenses/Apache-2.0.txt').read_bytes(), (module.ROOT / 'LICENSE').read_bytes())

    def test_each_notice_is_required_in_full_not_just_a_header_or_link(self):
        for item in self.inventory['components']:
            dest = self.site / item['file']
            original = dest.read_bytes()
            for mutation in [b'License: see upstream URL\n', original[:100], original + b'Changed\n']:
                with self.subTest(file=item['file'], mutation=mutation[:20]):
                    dest.write_bytes(mutation)
                    self.assertIn({'file': item['file'], 'rule': 'missing-or-modified-licensed-file'}, module.audit_licenses(self.site))
            dest.unlink()
            self.assertIn({'file': item['file'], 'rule': 'missing-or-modified-licensed-file'}, module.audit_licenses(self.site))
            dest.write_bytes(original)

    def test_real_javascript_font_map_and_image_bytes_are_bound(self):
        for item in self.inventory['assets']:
            with self.subTest(file=item['file']):
                dest = self.site / item['file']
                original = dest.read_bytes()
                dest.write_bytes(original[:-1])
                self.assertIn({'file': item['file'], 'rule': 'missing-or-modified-licensed-file'}, module.audit_licenses(self.site))
                dest.write_bytes(original)

    def test_new_static_assets_need_explicit_provenance_review(self):
        (self.site / 'js/extra.js').write_text('/* New unreviewed library */')
        self.assertIn({'file': 'js/extra.js', 'rule': 'unattributed-static-asset'}, module.audit_licenses(self.site))

    def test_plaintext_allowance_is_only_for_exact_notice_paths(self):
        (self.site / 'licenses/extra.txt').write_text('Do not upload unreviewed files')
        self.assertIn({'file': 'licenses/extra.txt', 'rule': 'unexpected-file-type'}, module.audit_site(self.site)[1])

    def test_notices_do_not_bypass_privacy_or_symlink_guards(self):
        dest = self.site / 'licenses/MkDocs.txt'
        original = dest.read_bytes()
        dest.write_bytes(b'-----BEGIN ' + b'PRIVATE KEY-----')
        self.assertIn({'file': 'licenses/MkDocs.txt', 'rule': 'private-key'}, module.audit_site(self.site)[1])
        dest.unlink()
        # Only a test-owned source, never an existing user's private file.
        source = self.root / 'notice.txt'
        source.write_bytes(original)
        dest.symlink_to(source)
        self.assertIn({'file': 'licenses/MkDocs.txt', 'rule': 'symlink'}, module.audit_site(self.site)[1])
        self.assertIn({'file': 'licenses/MkDocs.txt', 'rule': 'missing-or-modified-licensed-file'}, module.audit_licenses(self.site))

    def test_source_inventory_refuses_gaps_paths_duplicates_and_license_drift(self):
        fixture = self.root / 'source'
        (fixture / 'third_party').mkdir(parents=True)
        shutil.copyfile(module.ROOT / 'LICENSE', fixture / 'LICENSE')
        for item in self.inventory['components']:
            dest = fixture / 'docs' / item['file']
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(module.ROOT / 'docs' / item['file'], dest)
        manifest = fixture / 'third_party/docs-manifest.json'
        manifest.write_text(json.dumps(self.inventory))
        self.assertEqual(module.docs_licenses(fixture), self.inventory)
        mutations = []
        for case in ['missing', 'duplicate', 'escape', 'uncovered', 'bad-component']:
            data = json.loads(json.dumps(self.inventory))
            if case == 'missing':
                data['components'].pop()
            elif case == 'duplicate':
                data['components'][1]['file'] = data['components'][0]['file']
                data['components'][1]['sha256'] = data['components'][0]['sha256']
            elif case == 'escape':
                data['components'][0]['file'] = '../LICENSE'
            elif case == 'uncovered':
                data['assets'][0]['components'] = []
            else:
                data['assets'][0]['components'] = ['unknown-library']
            mutations.append((case, data))
        for case, data in mutations:
            with self.subTest(case=case):
                manifest.write_text(json.dumps(data))
                with self.assertRaises(ValueError):
                    module.docs_licenses(fixture)
        manifest.write_text(json.dumps(self.inventory))
        (fixture / 'LICENSE').write_text('A different license must not silently replace the project grant')
        with self.assertRaises(ValueError):
            module.docs_licenses(fixture)


if __name__ == '__main__':
    unittest.main()
