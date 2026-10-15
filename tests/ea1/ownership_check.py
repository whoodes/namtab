"""Compile the actual wrapper: normal ownership works; moving/sharing it must fail.
No new dependency or negative-test macro. Metadata-only compiler invocations never run probes.
"""
import json
import os
from pathlib import Path
import subprocess
from evidence_dir import EVIDENCE
ROOT=Path(__file__).resolve().parents[2]
build=subprocess.run(['cargo','build','--locked','--offline','-p','project-x-engine','--message-format=json'],cwd=ROOT,text=True,capture_output=True,check=True)
artifacts=[json.loads(s) for s in build.stdout.splitlines() if s.startswith('{')]
extern={}
for item in artifacts:
    if item.get('reason')=='compiler-artifact' and item['target']['name'] in ['project_x_protocol','serde_json']:
        for path in item['filenames']:
            if path.endswith('.rlib'):extern[item['target']['name']]=path
assert set(extern)=={'project_x_protocol','serde_json'}
folder=ROOT/'.artifacts/ea1/ownership';folder.mkdir(parents=True,exist_ok=True)
cases={'owned':('fn checked() {let mut t=ffi::Terminal::new(12,5).unwrap();let frame=t.frame().unwrap();drop(t);drop(frame);}',True),'not_send':('fn checked(t:ffi::Terminal) {std::thread::spawn(move || drop(t));}',False),'not_sync':('fn checked() {fn require<T:Sync>(){} require::<ffi::Terminal>();}',False)}
results=[]
for name,(body,passes) in cases.items():
    source=folder/(name+'.rs');source.write_text('#[path="'+str(ROOT/'crates/engine/src/ffi.rs')+'"] mod ffi;\n'+body+'\n')
    command=['rustc','--edition=2024','--crate-type=lib','--emit=metadata','--error-format=json','-L','dependency='+str(ROOT/'target/debug/deps'),'-o',str(folder/(name+'.rmeta'))]
    for n,path in extern.items():command+=['--extern',n+'='+path]
    command+=[str(source)]
    r=subprocess.run(command,cwd=ROOT,text=True,capture_output=True,timeout=30)
    messages=[json.loads(s) for s in r.stderr.splitlines() if s.startswith('{')]
    errors=[m for m in messages if m.get('level')=='error' and m.get('code')]
    codes=[m['code']['code'] for m in errors]
    assert (r.returncode==0)==passes,(name,r.stderr)
    if not passes:assert codes and set(codes)=={'E0277'},(name,r.stderr)
    results.append({'name':name,'verdict':'PASS','compiler_exit':r.returncode,'expected_compile_success':passes,'diagnostic_codes':codes,'command':command,'diagnostics':messages})
(EVIDENCE/'ownership.json').write_text(json.dumps({'verdict':'PASS','results':results},indent=2)+'\n')
print('Ownership compiler checks: normal lifetime PASS, cross-thread move rejected, shared access rejected (E0277).')
