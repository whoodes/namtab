"""Verify original failures, prior archives and all pre-EA1 assessment evidence."""
import hashlib
import json
from pathlib import Path
import tarfile
ROOT=Path(__file__).resolve().parents[2]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
m=json.loads(next((ROOT/'evidence/archive').glob('pre-ea1-*.manifest.json')).read_text())
assert sha(ROOT/m['archive'])==m['sha256']
for path,digest in m['existing_archive_sha256'].items():assert sha(ROOT/path)==digest,path
with tarfile.open(ROOT/m['archive']) as t:
    for f in m['files']:
        assert hashlib.sha256(t.extractfile(f['path']).read()).hexdigest()==f['sha256'],f['path']
    unchanged=[]
    for f in m['files']:
        n=f['path']
        if n.startswith('evidence/engine-assessment/') or n in ['tests/system/engine-probe.mjs','tests/fixtures/terminal/continuations.json','tests/fixtures/manifest.json','evidence/engine-continuation.json']:
            assert sha(ROOT/n)==f['sha256'],n
            unchanged.append(n)
prior=ROOT/'evidence/archive/post-cleanup-foundation-checks.json'
assert all(c['verdict']=='PASS' for c in json.loads(prior.read_text())['checks'])
record={'verdict':'PASS','pre_ea1_archive_members_verified':len(m['files']),'prior_archives_verified':len(m['existing_archive_sha256']),'unchanged_original_paths':unchanged,'cleanup_check_archive':{'path':str(prior.relative_to(ROOT)),'sha256':sha(prior)}}
(ROOT/'evidence/ea1/preservation.json').write_text(json.dumps(record,indent=2)+'\n')
print('Prior archives and',len(m['files']),'checkpoint members verified;',len(unchanged),'original oracle/fixture/result and assessment evidence files byte-unchanged.')
