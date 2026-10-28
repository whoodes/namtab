"""Synthetic-only MCP test gate; not a production adapter or a model provider.
It forwards real Namtab calls but permits only a fixed task and printf argv.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys
TASK='live-codex-fixture'
GOAL='Synthetic printf only'
ARGV=['/usr/bin/printf','NAMTAB_REAL_CODEX_20261027\n']

def admitted(params):
    name=params.get('name');args=params.get('arguments',{})
    if not isinstance(args,dict):return False
    if name=='namtab_task':return args=={'task':TASK,'goal':GOAL,'expected_artifacts':[]}
    if name=='namtab_run':return set(args)=={'task','task_revision','argv'} and args['task']==TASK and args['argv']==ARGV and isinstance(args['task_revision'],str) and args['task_revision'].startswith('sha256:')
    if name=='namtab_observe':return args=={} or set(args)=={'attempt'} and isinstance(args['attempt'],str) and len(args['attempt'])<=80
    return False

def main():
    p=argparse.ArgumentParser();p.add_argument('--bridge',required=True);p.add_argument('--socket',required=True);p.add_argument('--epoch',required=True);p.add_argument('--record',type=Path,required=True);a=p.parse_args()
    with a.record.open('x') as record:
        child=subprocess.Popen([a.bridge,'--socket',a.socket,'--epoch',a.epoch],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=sys.stderr)
        try:
            for _ in range(128):
                line=sys.stdin.buffer.readline(65537)
                if not line:break
                if len(line)>65536 or not line.endswith(b'\n'):raise RuntimeError('test gate input cap')
                q=json.loads(line);blocked=q.get('method')=='tools/call' and not admitted(q.get('params',{}))
                if blocked:
                    response={'jsonrpc':'2.0','id':q['id'],'result':{'content':[{'type':'text','text':'Synthetic-only fixture gate refused this call; no action dispatched.'}],'isError':True}}
                else:
                    child.stdin.write(line);child.stdin.flush()
                    if 'id' not in q:continue
                    raw=child.stdout.readline(262145)
                    if not raw or len(raw)>262144 or not raw.endswith(b'\n'):raise RuntimeError('test gate response cap')
                    response=json.loads(raw)
                    if q.get('method')=='tools/list' and 'result' in response:
                        response['result']['tools']=[t for t in response['result']['tools'] if t['name'] in ['namtab_task','namtab_run','namtab_observe']]
                row={'method':q.get('method'),'tool':q.get('params',{}).get('name'),'id':q.get('id'),'blocked':blocked,'response':response}
                record.write(json.dumps(row)+'\n');record.flush()
                sys.stdout.write(json.dumps(response)+'\n');sys.stdout.flush()
        finally:
            child.stdin.close()
            try:child.wait(timeout=3)
            except subprocess.TimeoutExpired:child.terminate();child.wait(timeout=3)
if __name__=='__main__':main()
