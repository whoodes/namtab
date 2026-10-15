"""Record bounded local local engineering commands; stdout contains synthetic/tool output only."""
import argparse
import datetime
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parent.parent
p = argparse.ArgumentParser()
p.add_argument('--name', required=True)
p.add_argument('--cwd', default='.')
p.add_argument('--evidence-dir', default='evidence/engine-assessment/commands')
p.add_argument('--input', action='append', default=[])
p.add_argument('--timeout', type=int, default=300)
p.add_argument('command', nargs=argparse.REMAINDER)
a = p.parse_args()
command = a.command[1:] if a.command[:1] == ['--'] else a.command
cwd = (ROOT / a.cwd).resolve()
folder = (ROOT / a.evidence_dir).resolve()
if not folder.is_relative_to(ROOT / 'evidence') or not a.name or any(c not in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in a.name):
    raise ValueError('Expected evidence subdirectory and lowercase command name')
if not cwd.is_relative_to(ROOT) or not command:
    raise ValueError('Expected project-local cwd and command')
env = dict(os.environ)
node = (ROOT / '.node-version').read_text().strip()
env.update({'PATH':str(ROOT / f'.tools/node/node-v{node}-linux-x64/bin')+os.pathsep+env['PATH'], 'CARGO_HOME':str(ROOT/'.cache/cargo'), 'PIP_CACHE_DIR':str(ROOT/'.cache/pip'), 'XDG_CACHE_HOME':str(ROOT/'.cache/xdg'), 'XDG_DATA_HOME':str(ROOT/'.cache/xdg-data'), 'XDG_STATE_HOME':str(ROOT/'.cache/xdg-state'), 'ZIG_GLOBAL_CACHE_DIR':str(ROOT/'.cache/engine-assessment/zig-global'), 'ZIG_LOCAL_CACHE_DIR':str(ROOT/'.cache/engine-assessment/zig-local'), 'NPM_CONFIG_USERCONFIG':'/dev/null','NPM_CONFIG_GLOBALCONFIG':'/dev/null'})
inputs={}
import hashlib
for name in ['tests/assessment/ghostty-proof.c','tests/assessment/ghostty-assessment.mjs','tests/assessment/xterm-assessment.mjs','tests/assessment/expectations.json','tests/fixtures/terminal/continuations.json'] + a.input:
    file=ROOT/name
    if file.exists():inputs[name]=hashlib.sha256(file.read_bytes()).hexdigest()
start = time.monotonic()
error = None
try:
    result = subprocess.run(command,cwd=cwd,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=a.timeout)
    code,stdout,stderr=result.returncode,result.stdout,result.stderr
except subprocess.TimeoutExpired as e:
    code,stdout,stderr=None,str(e.stdout or ''),str(e.stderr or '')
    error='timeout'
record={'input_file_sha256':inputs,'command':command,'cwd':str(cwd.relative_to(ROOT)),'recorded_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'elapsed_seconds':round(time.monotonic()-start,3),'timeout_seconds':a.timeout,'exit':code,'verdict':'PASS' if code==0 else 'FAIL','error':error,'environment_overrides':{k:v for k,v in env.items() if k in ['CARGO_HOME','PIP_CACHE_DIR','ZIG_GLOBAL_CACHE_DIR','ZIG_LOCAL_CACHE_DIR','XDG_CACHE_HOME','XDG_DATA_HOME','XDG_STATE_HOME','NPM_CONFIG_USERCONFIG','NPM_CONFIG_GLOBALCONFIG']},'stdout':stdout[-4000000:],'stderr':stderr[-4000000:],'output_truncated':len(stdout)>4000000 or len(stderr)>4000000}
folder.mkdir(parents=True, exist_ok=True)
index=1
while (folder/f'{a.name}-{index}.json').exists():index+=1
path=folder/f'{a.name}-{index}.json';path.write_text(json.dumps(record,indent=2)+'\n')
print(stdout[-12000:],end='');print(stderr[-12000:],end='');print(f'\n{a.name}: {record["verdict"]}; evidence {path.relative_to(ROOT)}',flush=True)
raise SystemExit(code if code is not None and code>=0 else 1)
