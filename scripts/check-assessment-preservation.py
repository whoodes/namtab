"""Verify prior evidence archive and the untouched independent original X4 oracle."""
import hashlib
import json
from pathlib import Path
import tarfile

root=Path(__file__).resolve().parent.parent
manifests=sorted((root/'evidence/archive').glob('pre-engine-assessment-*.manifest.json'))
if len(manifests)!=1:raise ValueError('Select the intended archive explicitly if more than one exists')
m=json.loads(manifests[0].read_text());path=root/m['archive']
if hashlib.sha256(path.read_bytes()).hexdigest()!=m['sha256']:raise ValueError('Archive digest changed')
with tarfile.open(path) as tar:
    for item in m['files']:
        if hashlib.sha256(tar.extractfile(item['path']).read()).hexdigest()!=item['sha256']:raise ValueError('Archive member digest changed')
    for name in ['tests/system/engine-probe.mjs','tests/fixtures/terminal/continuations.json','tests/fixtures/manifest.json']:
        if (root/name).read_bytes()!=tar.extractfile(name).read():raise ValueError('Original independent oracle/fixture changed '+name)
print(f"Prior {len(m['files'])}-file archive and three original oracle/fixture files verified unchanged.")
