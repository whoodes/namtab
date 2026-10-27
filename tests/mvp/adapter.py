#!/usr/bin/env python3
"""Real local MCP/broker/process journey driven by a labelled deterministic fixture.
--native adds the real window; neither path invokes a model/provider.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time
import sys
from native import Native
from mcp_client import MCP
from e2e import Connection,require,sha256

class Adapter(Native):
    def __init__(self,args):
        super().__init__(args);self.native=args.native;self.bridge=None
    def start(self):
        if self.native:
            super().start();self.wait(lambda:self.x.focus()==self.window_id,'owned native initial focus')
            self.event('focus_event',predicate=lambda d:d['focused'])
        else:
            self.window=None;self.x=None;self.sink=None
            self.daemon_log=(self.artifacts/'daemon.log').open('xb')
            self.process=subprocess.Popen([str(self.bin_dir/'namtabd'),'--socket',str(self.socket_path),'--workspace',str(self.workspace),'--worker',str(self.bin_dir/'namtab-terminal-worker'),'--pty-helper',str(self.bin_dir/'namtab-pty-helper')],stdin=subprocess.DEVNULL,stdout=self.daemon_log,stderr=subprocess.STDOUT,start_new_session=True)
            self.wait(lambda:self.socket_path.exists(),'owned broker startup')
            self.observer=Connection(self.socket_path,'adapter-test-observer','human')
        for source in [Path(__file__),Path(__file__).with_name('mcp_client.py'),Path(__file__).with_name('owned_process.py')]:
            (self.artifacts/source.name).write_bytes(source.read_bytes())
        self.record['fixture_support_hashes']={name:sha256(Path(__file__).with_name(name)) for name in ['mcp_client.py','owned_process.py']}
        self.record['adapter_harness_sha256']=sha256(Path(__file__))
        self.record['model_execution']='NOT RUN: deterministic protocol fixture'
        self.record['binary_hashes']={n:sha256(self.bin_dir/n) for n in ['namtabd','namtab-mcp','namtabctl','namtab-terminal-worker','namtab-pty-helper']}
    def journey(self):
        epoch=self.snapshot()['epoch'];self.bridge=MCP(self.bin_dir/'namtab-mcp',self.socket_path,epoch,self.artifacts/'mcp.log',Path.cwd())
        m=self.bridge
        catalog=m.call('tools/list',{})['result']['tools'];require(len(catalog)==9,'unexpected tool set')
        bad=m.tool('namtab_run',{'task':'fixture','argv':['/bin/true']},ok=False);require(bad['result']['isError'],'missing exact task revision admitted')
        t=m.value('namtab_task',{'task':'MCP-fixture','goal':'Synthetic MCP workflow; no model reasoning','expected_artifacts':['fixture.txt']})
        rev=t['receipt']['result']['revision'];endpoint=t['receipt']['endpoint']
        run=m.tool('namtab_run',{'task':'MCP-fixture','task_revision':rev,'argv':['/bin/sh','-c',"set -C; printf synthetic > fixture.txt; printf MCP_REAL_STDOUT; printf MCP_REAL_STDERR >&2"]})
        v=run['result']['structuredContent'];attempt=v['observation_or_response']['receipt']['result']['attempt'];origin=v['binding']
        duplicate=m.call('tools/call',{'name':'namtab_run','arguments':{'task':'MCP-fixture','task_revision':rev,'argv':['/bin/true']}},repeat_id=m.next-1)
        require('error' in duplicate,'reused RPC id dispatched')
        self.wait_snapshot(lambda s:len(s['attempts'])==1 and s['attempts'][0]['state']=='exited','MCP real process exit and drain')
        a=m.value('namtab_observe',{'attempt':attempt})
        require(a['stdout_preview']=='MCP_REAL_STDOUT' and a['stderr_preview']=='MCP_REAL_STDERR','MCP stream attribution mismatch')
        require(a['origin']['endpoint']==endpoint==origin['endpoint'] and a['origin']['request_id']==origin['broker_request_id'] and a['task_revision']==rev,'MCP receipt/attempt binding mismatch')
        require((self.workspace/'fixture.txt').read_bytes()==b'synthetic','actual command file effect wrong')
        m.value('namtab_artifact',{'task':'MCP-fixture','path':'fixture.txt'})
        m.value('namtab_create_terminal',{'name':'MCP-fixture-terminal'})
        session=self.wait_snapshot(lambda s:any(x['name']=='MCP-fixture-terminal' for x in s['sessions']),'MCP terminal startup')
        sid=next(x['id'] for x in session['sessions'] if x['name']=='MCP-fixture-terminal')
        self.wait_session(sid,self.at_prompt,'MCP terminal actual shell prompt')
        s=m.value('namtab_observe',{'session':sid});m.value('namtab_claim_terminal',{'session':sid,'generation':s['generation']});s=m.value('namtab_observe',{'session':sid});oldgen=s['generation']
        # Exercise successful observed-context terminal delivery, then use a real
        # file effect to distinguish execution from echoed command text.
        def input_args(s,intent):return {'session':sid,'generation':s['generation'],'revision':s['screen']['revision'],'mode_revision':s['screen']['mode_revision'],'intent':intent}
        m.value('namtab_input',input_args(s,{'kind':'text','text':'printf y>mcp-input-count'}))
        self.wait_session(sid,lambda x:any('mcp-input-count' in l for l in x['screen']['lines']),'MCP command text echoed')
        s=m.value('namtab_observe',{'session':sid})
        m.value('namtab_input',input_args(s,{'kind':'key','key':'Enter','ctrl':False,'alt':False,'shift':False}))
        self.wait(lambda:(self.workspace/'mcp-input-count').exists(),'MCP terminal command actual side effect')
        require((self.workspace/'mcp-input-count').read_bytes()==b'y','MCP terminal input repeated or corrupted')
        self.wait_session(sid,self.at_prompt,'MCP terminal prompt after command');s=m.value('namtab_observe',{'session':sid})
        if self.native:
            self.capture('08-mcp-work.png');self.hotkey('2');self.hotkey('Tab');self.arm();human=self.native_endpoint
            self.wait_session(sid,lambda x:x['owner']==human,'native takeover of MCP endpoint');self.capture('09-mcp-takeover.png')
        else:
            r=self.observer.call({'op':'take_control','session':sid,'generation':oldgen,'force':True});require(r['ok'],'human takeover refused')
        stale=m.tool('namtab_input',{'session':sid,'generation':oldgen,'revision':s['screen']['revision'],'mode_revision':s['screen']['mode_revision'],'intent':{'kind':'text','text':'MCP_STALE'}},ok=False)
        require(stale['result']['isError'],'stale MCP input accepted')
        receipt=stale['result']['structuredContent']['observation_or_response']['receipt'];require(receipt['delivered_bytes']==0,'stale MCP write extent not zero')
        force=m.tool('namtab_claim_terminal',{'session':sid,'generation':oldgen,'force':True},ok=False);require(force['result']['isError'],'agent force field accepted')
        s=m.value('namtab_observe',{'session':sid});refused=m.tool('namtab_claim_terminal',{'session':sid,'generation':s['generation']},ok=False);require(refused['result']['isError'],'agent stole human-owned terminal')
        # A second direct managed child stays live across MCP EOF, then is stopped by explicit lifecycle.
        m.value('namtab_run',{'task':'MCP-fixture','task_revision':rev,'argv':['/bin/sleep','30']})
        transcript=m.transcript; m.close();self.bridge=None
        self.wait_snapshot(lambda s:any(a['state']=='running' for a in s['attempts']),'MCP EOF must detach, not cancel')
        (self.artifacts/'mcp-transcript.json').write_text(json.dumps(transcript,indent=2)+'\n')
        require(not any(line=='MCP_STALE' for x in self.snapshot()['sessions'] for line in x['screen']['lines']),'stale MCP bytes reached terminal')
        # Repeated status reuses one restricted lifecycle identity.
        for _ in range(20):
            r=subprocess.run([str(self.bin_dir/'namtabctl'),'--socket',str(self.socket_path),'info','--epoch',epoch],capture_output=True,text=True,timeout=4);require(r.returncode==0,'repeated lifecycle query exhausted identities')
        self.record['retained_endpoint_count']=len(self.snapshot()['endpoints']);require(self.record['retained_endpoint_count']<=4,'status consumed new identities')
        wrong=subprocess.run([str(self.bin_dir/'namtabctl'),'--socket',str(self.socket_path),'stop','--epoch','wrong-epoch'],capture_output=True,timeout=3);require(wrong.returncode!=0 and self.process.poll() is None,'wrong epoch stopped broker')
        agent=Connection(self.socket_path,'shutdown-agent-refusal','agent');r=agent.call({'op':'shutdown'});require(not r['ok'],'agent shutdown accepted');agent.close()
        self.descendants();direct=[]
        children=Path('/proc')/str(self.process.pid)/'task'/str(self.process.pid)/'children'
        for pid in children.read_text().split():
            p=Path('/proc')/pid
            if p.exists():direct.append({'pid':int(pid),'comm':(p/'comm').read_text().strip(),'starttime':(p/'stat').read_text().rsplit(')',1)[1].split()[19]})
        manifest={'socket':str(self.socket_path),'workspace':str(self.workspace),'epoch':epoch,'daemon_pid':self.process.pid}
        (self.runtime/'launch.json').write_text(json.dumps(manifest)+'\n')
        env={k:v for k,v in os.environ.items() if k!='DISPLAY'};env['CARGO_TARGET_DIR']=str(self.bin_dir.parent)
        stopped=subprocess.run(['python3','-B','scripts/mvp.py','stop','--run',str(self.runtime)],env=env,capture_output=True,text=True,timeout=10)
        (self.artifacts/'stop.log').write_text(stopped.stdout+stopped.stderr)
        require(stopped.returncode==0,'headless explicit launcher stop failed')
        require(self.process.wait(timeout=5)==0 and not self.socket_path.exists(),'owned broker did not exit/remove its socket')
        for item in direct:
            p=Path('/proc')/str(item['pid'])
            require(not p.exists() or (p/'stat').read_text().rsplit(')',1)[1].split()[19]!=item['starttime'],'owned direct child survived orderly exit')
        require((self.workspace/'fixture.txt').exists() and (self.runtime/'launch.json').exists(),'stop deleted run evidence')
        if self.native:
            self.event('connection_ready');time.sleep(.2);self.hotkey('1');self.capture('10-stopped-disconnected.png')
        self.record['checks'].extend([{'name':x,'verdict':'PASS'} for x in ['actual_mcp_stdio_schema_lifecycle_and_duplicate_ids','real_process_stream_task_revision_and_endpoint_attribution','agent_terminal_claim_human_takeover_stale_zero_byte_refusal','mcp_eof_detaches_without_cancelling_work','repeated_status_does_not_exhaust_identities','wrong_epoch_and_agent_stop_refused','explicit_headless_launcher_stop_reaps_owned_direct_children_retains_files']])
        self.record['shutdown_direct_children']=direct
        self.record['limitations']=['Deterministic MCP protocol fixture, not model execution','Direct-child stop only; descendants/external side effects and pipe drain not guaranteed','Native variant uses targeted synthetic X11 events, not physical keyboard/IME/a11y or fleet capacity qualification']
    def cleanup(self):
        if self.bridge:
            self.bridge.close();self.bridge=None
        super().cleanup()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--terminfo',type=Path,default=Path('.'));p.add_argument('--artifacts',type=Path,required=True);p.add_argument('--native',action='store_true')
    Adapter(p.parse_args()).run()
