"""Compile the actual wrapper; Cargo owns dependency-instance selection.
No native probe program is linked/run. Positive compilation guards negative oracles.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import uuid
from evidence_dir import EVIDENCE

ROOT = Path(__file__).resolve().parents[2]
metadata = json.loads(subprocess.check_output(
    ['cargo', 'metadata', '--locked', '--offline', '--no-deps', '--format-version', '1'],
    cwd=ROOT, text=True))
engine = next(p for p in metadata['packages'] if p['name'] == 'project-x-engine')
serde_json = next(d['req'] for d in engine['dependencies'] if d['name'] == 'serde_json')
assert serde_json.startswith('=')
folder = ROOT / '.artifacts/ea1/ownership' / uuid.uuid4().hex
(folder / 'src/bin').mkdir(parents=True)
manifest = folder / 'Cargo.toml'
manifest.write_text(
    '[package]\nname = "ea1-ownership-probe"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n'
    '[workspace]\n[dependencies]\nproject-x-protocol = { path = '
    + json.dumps(str(ROOT / 'crates/protocol')) + ' }\nserde_json = ' + json.dumps(serde_json) + '\n')
shutil.copyfile(ROOT / 'Cargo.lock', folder / 'Cargo.lock')
env = {**os.environ, 'CARGO_TARGET_DIR': str(ROOT / 'target/ea1-ownership')}
cases = {
    'owned': ('fn checked() {let mut t=ffi::Terminal::new(12,5).unwrap();let frame=t.frame().unwrap();drop(t);drop(frame);}', True),
    'not_send': ('fn checked(t:ffi::Terminal) {std::thread::spawn(move || drop(t));}', False),
    'not_sync': ('fn checked() {fn require<T:Sync>(){} require::<ffi::Terminal>();}', False),
}
for name, (body, _) in cases.items():
    source = folder / 'src/bin' / (name + '.rs')
    source.write_text('#[path=' + json.dumps(str(ROOT / 'crates/engine/src/ffi.rs')) + '] mod ffi;\n' + body + '\nfn main() {}\n')
subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)],
               cwd=ROOT, env=env, check=True, capture_output=True, text=True)

def registry_records(path):
    records = set()
    for block in path.read_text().split('[[package]]')[1:]:
        fields = dict(re.findall(r'^(name|version|source|checksum) = "([^"\n]+)"$', block, re.M))
        if fields.get('source', '').startswith('registry+'):
            records.add(tuple(fields[k] for k in ['name', 'version', 'source', 'checksum']))
    return records

probe_records = registry_records(folder / 'Cargo.lock')
workspace_records = registry_records(ROOT / 'Cargo.lock')
assert probe_records and workspace_records and probe_records <= workspace_records, 'Probe resolved unreviewed registry records'
results = []
for name, (body, passes) in cases.items():
    source = folder / 'src/bin' / (name + '.rs')
    command = ['cargo', 'check', '--locked', '--offline', '--manifest-path', str(manifest), '--bin', name, '--message-format=json']
    result = subprocess.run(command, cwd=ROOT, env=env, text=True, capture_output=True, timeout=120)
    artifacts = [json.loads(s) for s in result.stdout.splitlines() if s.startswith('{')]
    messages = [m['message'] for m in artifacts if m.get('reason') == 'compiler-message']
    codes = [m['code']['code'] for m in messages if m.get('level') == 'error' and m.get('code')]
    assert (result.returncode == 0) == passes, (name, result.stderr, messages)
    if not passes:
        assert codes and set(codes) == {'E0277'}, (name, messages)
    results.append({'name': name, 'verdict': 'PASS', 'compiler_exit': result.returncode,
                    'expected_compile_success': passes, 'diagnostic_codes': codes,
                    'command': command, 'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
                    'diagnostics': messages})
record = {'verdict': 'PASS', 'results': results, 'probe_manifest': str(manifest.relative_to(ROOT)),
          'probe_lock_sha256': hashlib.sha256((folder / 'Cargo.lock').read_bytes()).hexdigest(),
          'registry_closure_subset_of_workspace_lock': True,
          'probe_registry_package_count': len(probe_records),
          'workspace_registry_package_count': len(workspace_records),
          'limits': 'Cargo check only, no probe execution or native linking; does not prove runtime memory safety.'}
with (EVIDENCE / 'ownership.json').open('x') as output:
    json.dump(record, output, indent=2)
    output.write('\n')
print('Ownership compiler checks: normal lifetime PASS; cross-thread move and shared access rejected (E0277). Cargo resolved matching dependency instances; registry closure stays within Cargo.lock.')
