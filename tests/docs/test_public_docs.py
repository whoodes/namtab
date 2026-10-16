"""Meaningful refusal cases for the public artifact guard, without network calls."""
from pathlib import Path
import importlib.util
import gzip
import tempfile
import unittest

import yaml

spec = importlib.util.spec_from_file_location('public_docs', Path(__file__).resolve().parents[2] / 'scripts/public-docs.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PublicArtifactTests(unittest.TestCase):
    def setUp(self):
        (module.ROOT / '.artifacts').mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=module.ROOT / '.artifacts', prefix='public-docs-test-')
        self.site = Path(self.temporary.name)
        (self.site / 'search').mkdir()
        (self.site / 'index.html').write_text('<html>Reviewed documentation.</html>')
        (self.site / 'search/search_index.json').write_text('{"docs": []}')

    def tearDown(self):
        self.temporary.cleanup()

    def test_normal_site_has_hashed_inventory(self):
        manifest, findings = module.audit_site(self.site)
        self.assertEqual(findings, [])
        self.assertEqual(len(manifest), 2)
        self.assertTrue(all(len(item['sha256']) == 64 for item in manifest))

    def test_search_index_is_scanned_without_exposing_value(self):
        marker = 'ghp_' + 'A' * 36
        (self.site / 'search/search_index.json').write_text('{"text": "' + marker + '"}')
        _, findings = module.audit_site(self.site)
        self.assertEqual(findings, [{'file': 'search/search_index.json', 'rule': 'github-token'}])
        self.assertNotIn(marker, str(findings))

    def test_local_paths_and_contact_are_refused(self):
        (self.site / 'index.html').write_text('/home/example/private example' + '@' + 'gmail.com')
        _, findings = module.audit_site(self.site)
        self.assertIn({'file': 'index.html', 'rule': 'local-home-path'}, findings)
        self.assertIn({'file': 'index.html', 'rule': 'private-contact'}, findings)

    def test_private_file_and_link_are_not_uploaded(self):
        (self.site / '.env').write_text('not uploaded')
        (self.site / 'linked.html').symlink_to(self.site / 'index.html')
        _, findings = module.audit_site(self.site)
        self.assertIn({'file': '.env', 'rule': 'unexpected-file-type'}, findings)
        self.assertIn({'file': 'linked.html', 'rule': 'symlink'}, findings)

    def test_empty_site_fails_closed(self):
        (self.site / 'index.html').unlink()
        _, findings = module.audit_site(self.site)
        self.assertIn({'file': '(site)', 'rule': 'missing-page-or-search-index'}, findings)

    def test_compressed_content_is_scanned(self):
        with gzip.open(self.site / 'sitemap.xml.gz', 'wb') as stream:
            stream.write(b'-----BEGIN ' + b'PRIVATE KEY-----')
        _, findings = module.audit_site(self.site)
        self.assertIn({'file': 'sitemap.xml.gz', 'rule': 'private-key'}, findings)

    def test_external_script_is_refused_but_citation_is_allowed(self):
        (self.site / 'index.html').write_text('<a href="https://example.invalid/reference">Reference</a>')
        self.assertEqual(module.audit_site(self.site)[1], [])
        (self.site / 'index.html').write_text('<script src="https://example.invalid/library.js"></script>')
        self.assertIn({'file': 'index.html', 'rule': 'external-runtime-resource'}, module.audit_site(self.site)[1])

    def test_project_root_links_and_fragments_resolve(self):
        (self.site / 'index.html').write_text('<h1 id="hello">Hi</h1><a href="/namtab/#hello">Home</a>')
        self.assertEqual(module.audit_site(self.site)[1], [])
        (self.site / 'index.html').write_text('<a href="/other-project/">Wrong project</a><a href="#missing">Missing</a>')
        rules = {item['rule'] for item in module.audit_site(self.site)[1]}
        self.assertEqual(rules, {'escaping-site-link', 'missing-site-anchor'})


class WorkflowContractTests(unittest.TestCase):
    def test_only_default_branch_can_upload_or_deploy_with_narrow_permissions(self):
        workflow = yaml.load((module.ROOT / '.github/workflows/docs-pages.yml').read_text(), Loader=yaml.BaseLoader)
        self.assertNotIn('pull_request_target', workflow['on'])
        self.assertEqual(workflow['on']['push']['branches'], ['main'])
        self.assertEqual(workflow['permissions'], {'contents': 'read'})
        jobs = workflow['jobs']
        self.assertEqual(jobs['deploy']['needs'], 'build')
        self.assertEqual(jobs['deploy']['permissions'], {'pages': 'write', 'id-token': 'write'})
        gate = "github.event_name != 'pull_request' && github.ref == 'refs/heads/main'"
        self.assertEqual(jobs['deploy']['if'], gate)
        upload = next(step for step in jobs['build']['steps'] if step.get('uses', '').startswith('actions/upload-pages-artifact@'))
        self.assertEqual(upload['if'], gate)
        self.assertEqual(upload['with']['path'], '.artifacts/public-docs/site')
        for job in jobs.values():
            for step in job['steps']:
                if 'uses' in step:
                    self.assertRegex(step['uses'], r'^actions/[a-z-]+@[0-9a-f]{40}$')


if __name__ == '__main__':
    unittest.main()
