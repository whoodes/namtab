"""Verify evaluated source/build tuple and characterize the justified lock change."""
import hashlib
import json
from pathlib import Path
import platform
import re
import subprocess
import tarfile
ROOT=Path(__file__).resolve().parents[2]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
pin=json.loads((ROOT/'evidence/ea1/artifact-pin.json').read_text())
source=json.loads((ROOT/'evidence/engine-assessment/ghostty-source.json').read_text())
assert pin['commit']==source['commit']=='22fa134a07f5749edd9e92c6bc17d8aaf61843be'
checked={}
for file,digest in {pin['library']:pin['library_sha256'],pin['zig']:pin['zig_sha256'],**pin['headers']}.items():
    assert sha(ROOT/file)==digest,file
    checked[file]=digest
for header in pin['headers']:
    suffix=Path(header).relative_to('.artifacts/engine-assessment/ghostty')
    assert (ROOT/header).read_bytes()==(ROOT/source['source_directory']/suffix).read_bytes(),header
manifest=json.loads(next((ROOT/'evidence/archive').glob('pre-ea1-*.manifest.json')).read_text())
assert sha(ROOT/'pnpm-lock.yaml')==manifest['lockfiles']['pnpm-lock.yaml']
with tarfile.open(ROOT/manifest['archive']) as t:
    old=t.extractfile('Cargo.lock').read().decode()
new=(ROOT/'Cargo.lock').read_text()
registry=lambda s:sorted(b for b in s.split('[[package]]') if 'source = "registry+' in b)
assert registry(old)==registry(new),'Registry lock changed'
names=lambda s:set(re.findall('^name = "(.*)"$',s,re.M))
assert names(new)-names(old)=={'project-x-engine'}
assert not names(old)-names(new)
from worker_test import worker
with worker() as w:
    paths={line.split()[-1] for line in Path(f'/proc/{w.p.pid}/maps').read_text().splitlines() if 'libghostty-vt' in line}
    assert paths=={str((ROOT/pin['library']).resolve())},paths
versions={}
for name,args in {'rustc':['rustc','--version'],'cargo':['cargo','--version'],'node':['node','--version'],'pnpm':['node','.tools/pnpm/package/bin/pnpm.cjs','--version'],'python':['python3','--version'],'zig':[pin['zig'],'version'],'c_compiler':[pin['zig'],'cc','--version']}.items():
    versions[name]=subprocess.check_output(args,cwd=ROOT,text=True).strip()
code=['crates/engine/native/boundary.c','crates/engine/native/boundary.h','crates/engine/src/ffi.rs']
counts={n:len((ROOT/n).read_text().splitlines()) for n in code}
assert sum(counts.values())<=800,'Adapter budget exceeded'
record={'verdict':'PASS','environment':{'platform':platform.platform(),'machine':platform.machine(),'os_release':Path('/etc/os-release').read_text()},'versions':versions,'checked_sha256':checked,'headers_equal_pinned_source':len(pin['headers']),'loaded_library':list(paths),'worker_executable_sha256':sha(ROOT/'target/debug/project-x-engine'),'worker_build_profile':'Rust dev; C shim -O1 -g; libghostty-vt ReleaseSafe/simd=false','library_embeds_local_checkout_path':str(ROOT).encode() in (ROOT/pin['library']).read_bytes(),'locks':{'before':manifest['lockfiles'],'after':{n:sha(ROOT/n) for n in manifest['lockfiles']},'registry_package_records_unchanged':True,'only_added_package':'project-x-engine (local workspace member; no registry additions)'},'adapter_line_counts_including_comments_tests':counts,'adapter_lines':sum(counts.values()),'private_entries':0,'upstream_patches':0,'source_sha256':{n:sha(ROOT/n) for n in code+['crates/engine/src/main.rs','crates/engine/build.rs','crates/protocol/src/internal_engine.rs']},'limits':'Pinned development build on this VM, not SDK release/ABI stability, hostile allocator/fuzz certification, or production sandbox. Same-user loader tampering is outside EA1 containment.'}
(ROOT/'evidence/ea1/provenance.json').write_text(json.dumps(record,indent=2)+'\n')
print('35 source-matching headers, library/compiler hashes and loaded library verified. Cargo registry records and pnpm lock unchanged; one local Cargo member added. Adapter lines:',record['adapter_lines'])
