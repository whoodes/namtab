"""Pure provenance failure injection: never runs a broker, CLI or model."""
import tempfile
import unittest
from pathlib import Path
from scoped_observation import finish_cli_provenance

A = 'ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb'

class Provenance(unittest.TestCase):
    def check(self, content):
        root=Path(__file__).resolve().parents[2]/'.artifacts';root.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='scope-hash-',dir=root) as tmp:
            p=Path(tmp)/'synthetic-not-an-executable'
            if content is not None:p.write_bytes(content)
            record={'executed_cli':{'path':str(p),'sha256_before':A}}
            result=finish_cli_provenance(record)
            return result,record['executed_cli']

    def test_known_unchanged_bytes_match(self):
        ok,data=self.check(b'a');self.assertTrue(ok);self.assertEqual(data['sha256_after'],A)

    def test_changed_bytes_do_not_replace_initial_attestation(self):
        ok,data=self.check(b'b');self.assertFalse(ok);self.assertEqual(data['sha256_before'],A)
        self.assertNotEqual(data['sha256_after'],A)

    def test_missing_binary_retains_explicit_failure_without_throwing(self):
        ok,data=self.check(None);self.assertFalse(ok);self.assertIsNone(data['sha256_after'])
        self.assertEqual(data['after_error'],'FileNotFoundError')

    def test_no_cli_stage_does_not_fabricate_an_attestation(self):
        record={};self.assertTrue(finish_cli_provenance(record));self.assertEqual(record,{})

if __name__=='__main__':unittest.main()
