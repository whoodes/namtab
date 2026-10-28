#!/usr/bin/env python3
"""MANUAL ONLY: owner-authorized existing Codex/OpenAI, synthetic-only fixture.
Never used by CI. No credential/config inspection, install, login or persistent setup.
"""
import argparse
import json
import os
from pathlib import Path
import select
import shutil
import subprocess
import time
from adapter import Adapter
from e2e import require,sha256
from owned_process import close_child
from model_gate import TASK,GOAL,ARGV

class Live(Adapter):
    def journey(self):
        require(shutil.which('codex') is not None,'Existing Codex CLI missing; no installation/auth fallback')
        for source in [Path(__file__),Path(__file__).with_name('model_gate.py'),Path(__file__).with_name('owned_process.py')]:
            (self.artifacts/source.name).write_bytes(source.read_bytes())
            self.record.setdefault('live_harness_hashes',[]).append({'path':str(source),'sha256':sha256(source)})
        epoch=self.snapshot()['epoch']
        # Fresh nested repository prevents automatic discovery of Namtab root.
        subprocess.run(['git','init','--quiet','--initial-branch=synthetic-fixture',str(self.workspace)],check=True)
        proxy=(Path(__file__).with_name('model_gate.py')).resolve()
        record=self.artifacts/'live-mcp.jsonl'
        config={
            'model_provider':'openai','project_doc_max_bytes':0,'project_doc_fallback_filenames':[],
            'features.shell_tool':False,'features.shell_snapshot':False,'features.plugins':False,'features.memories':False,
            'web_search':'disabled','log_dir':str(self.artifacts/'codex-logs'),
            'mcp_servers.namtab.command':shutil.which('python3'),
            'mcp_servers.namtab.args':['-B',str(proxy),'--bridge',str(self.bin_dir/'namtab-mcp'),'--socket',str(self.socket_path),'--epoch',epoch,'--record',str(record)],
            'mcp_servers.namtab.required':True,'mcp_servers.namtab.default_tools_approval_mode':'prompt',
        }
        prompt=('Synthetic-only Namtab adapter test. Do not read or change any files. Do not use shell, patch, web, resources or any tool except the Namtab MCP tools. '
                'Call namtab_task exactly with '+json.dumps({'task':TASK,'goal':GOAL,'expected_artifacts':[]})+'. '
                'Use its returned exact task revision to call namtab_run once with task '+TASK+' and argv '+json.dumps(ARGV)+'. '
                'Then poll namtab_observe for the returned attempt ID until exit0 and stdout NAMTAB_REAL_CODEX_20261027. '
                'Do not retry a failed or uncertain run. Report the observed attempt, origin and result; do not claim other tools are mediated.')
        cmd=['codex','--no-daemon','exec','--ephemeral','--ignore-user-config','--strict-config','--json','--sandbox','read-only','--skip-git-repo-check','--cd',str(self.workspace)]
        for key,value in config.items():cmd+=['-c',key+'='+json.dumps(value)]
        cmd+=[prompt]
        self.record['model_execution']='ATTEMPTED REAL CODEX/OPENAI; verdict follows actual CLI and broker evidence'
        self.record['codex_version']=subprocess.check_output(['codex','--version'],text=True).strip()
        self.record['codex_invocation']=cmd
        self.record['data_boundary']={'workspace':'Fresh nested synthetic repository; project AGENTS discovery disabled; user config ignored for this invocation; authentication remains native CLI-managed','tools':'Command execution/plugins/memories/web disabled for this invocation. MCP test gate permits fixed synthetic task/printf/observe only. No sandbox bypass or security settings change.','limits':'Observed tool-event audit is not a proof of universal Codex sandbox enforcement; no product claim that other agent tools are mediated.'}
        stderr=(self.artifacts/'codex.stderr').open('xb');self.codex=subprocess.Popen(cmd,cwd=self.workspace,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=stderr)
        events=[];pending=b'';deadline=time.monotonic()+120
        try:
            while True:
                remaining=deadline-time.monotonic()
                if remaining<=0:raise AssertionError('Real Codex validation timed out; no automatic retry')
                if not select.select([self.codex.stdout],[],[],min(remaining,1))[0]:
                    if self.codex.poll() is not None:break
                    continue
                raw=os.read(self.codex.stdout.fileno(),8192)
                if not raw:break
                pending+=raw
                if len(pending)>1_048_576:raise AssertionError('Codex event buffer cap')
                while b'\n' in pending:
                    line,pending=pending.split(b'\n',1)
                    if not line:continue
                    row=json.loads(line);events.append(row)
                    if len(events)>128:raise AssertionError('Codex event count cap')
                    kind=row.get('item',{}).get('type')
                    if kind in ['command_execution','file_change','web_search']:
                        raise AssertionError('Codex attempted a bypass tool; validation stopped, no claim of mediation')
            code=self.codex.wait(timeout=5)
            (self.artifacts/'codex-events.json').write_text(json.dumps(events,indent=2)+'\n')
            require(code==0,'Real Codex exited nonzero; inspect safe local stderr, no login/scope fallback')
            require(any(r.get('item',{}).get('type')=='mcp_tool_call' and r.get('item',{}).get('tool')=='namtab_run' for r in events),'No real model run call; inspect approval/error events. Discovery is not execution.')
            snap=self.wait_snapshot(lambda s:len(s['attempts'])==1 and s['attempts'][0]['state']=='exited','real model-mediated command result')
            a=snap['attempts'][0];require(a['stdout']=='NAMTAB_REAL_CODEX_20261027\n' and a['exit_code']==0 and a['task_revision'],'No actual model-directed attributed Namtab result')
            transcript=[json.loads(l) for l in record.read_text().splitlines()]
            calls=[r for r in transcript if r.get('method')=='tools/call']
            require(any(r.get('tool')=='namtab_run' and not r['blocked'] for r in calls),'No Namtab MCP run was actually forwarded')
            require(not any(r['blocked'] for r in calls),'Real validation attempted unapproved MCP action')
            receipts=[r['response'].get('result',{}).get('structuredContent',{}) for r in calls]
            require(any(v.get('binding',{}).get('endpoint')==a['origin']['endpoint'] and v.get('binding',{}).get('broker_request_id')==a['origin']['request_id'] for v in receipts),'Model tool/receipt/attempt attribution differs')
            self.record['checks'].append({'name':'real_codex_openai_synthetic_mcp_run','verdict':'PASS','attempt':a,'mcp_tool_calls':len(calls),'model_tool_event_types':sorted(set(r.get('item',{}).get('type','') for r in events))})
            self.record['model_execution']='PASS: actual Codex/OpenAI model directed fixed synthetic Namtab operations; not a scripted model result'
            self.record['limitations']=['One model, one managed attempt through a test-only allowlist gate; no terminal model input or fleet capacity proof','Only Namtab tool calls are mediated; disabled/bypass paths are not production policy enforcement','No repository files intentionally supplied; fresh synthetic workspace and event audit, not universal data-isolation certification']
        finally:
            self.finish_fixture(epoch, stderr)
    def finish_fixture(self, epoch, stderr):
        # Always attempt public stop, even if owned provider-child cleanup fails.
        try:
            close_child(self.codex, timeout=5)
        finally:
            stderr.close()
            self.stop_fixture(epoch)
    def stop_fixture(self, epoch):
        result=subprocess.run([str(self.bin_dir/'namtabctl'),'--socket',str(self.socket_path),'stop','--epoch',epoch],capture_output=True,timeout=5)
        (self.artifacts/'stop-live.log').write_bytes(result.stdout+result.stderr)
        if result.returncode==0:self.process.wait(timeout=5)
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--allow-live-codex-synthetic',action='store_true',required=True);p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--terminfo',type=Path,default=Path('.'));p.add_argument('--artifacts',type=Path,required=True)
    args=p.parse_args();args.native=False;Live(args).run()
