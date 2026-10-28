#!/usr/bin/env python3
"""Real bounded scoped-pull proof. Scripted clients, no model or GUI launch."""
import argparse
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time
import traceback
from adapter import Adapter
from e2e import Connection, require, sha256, git_metadata

PROFILE = 'namtab.scoped-observation/0'
CAP = 'scoped_observe_v0'

class CountedReader:
    def __init__(self, reader): self.reader=reader; self.last_bytes=0
    def readline(self, limit):
        line=self.reader.readline(limit);self.last_bytes=len(line);return line
    def close(self): self.reader.close()

class MeasuredConnection(Connection):
    def call(self, operation, request_id=None):
        if not isinstance(self.reader, CountedReader): self.reader=CountedReader(self.reader)
        start=time.perf_counter_ns();result=super().call(operation,request_id)
        self.last_sample={'elapsed_ns':time.perf_counter_ns()-start,'response_wire_bytes':self.reader.last_bytes}
        return result

def scoped(client, kind, target=None):
    selection={'kind':kind}
    if target is not None: selection[kind]=target
    r=client.call({'op':'observe_scoped','epoch':client.epoch,'scope':selection})
    require(r['ok'] and not r['snapshot'] and not r['receipt'] and not r['resume'],'scoped response mixed with full snapshot/mutation/credentials')
    o=r['observation'];require(o['version']=='namtab.mk1/1' and o['profile']==PROFILE and o['epoch']==client.epoch and o['endpoint']==client.endpoint,'projection header binding')
    v=o['view'];require(v['kind']==kind,'wrong projection kind')
    if target is not None:require(v[kind]['id']==target,'wrong selected identity')
    return v

def quantiles(samples):
    v=sorted(s['elapsed_ns'] for s in samples)
    return {'n':len(v),'p50_ns':v[int((len(v)-1)*.50)],'p95_ns':v[int((len(v)-1)*.95)],'p99_ns':v[int((len(v)-1)*.99)],'max_ns':max(v),'quantile':'sorted lower-rank floor((n-1)*p), no interpolation'}

class ScopedPull(Adapter):
    def journey(self):
        self.record['profile']=PROFILE
        cli=self.bin_dir/'namtab-agent'
        self.record['executed_cli']={'path':str(cli),'sha256_before':sha256(cli)}
        self.record['binary_hashes']['namtab-agent']=self.record['executed_cli']['sha256_before']
        sources=self.artifacts/'sources';sources.mkdir()
        for p in [Path(__file__),Path(__file__).with_name('adapter.py'),Path(__file__).with_name('native.py'),Path(__file__).parents[1]/'mk1/e2e.py']:
            with (sources/p.name).open('xb') as f:f.write(p.read_bytes())
        self.record['source_hashes']={p.name:sha256(p) for p in sources.iterdir()}
        a=Connection(self.socket_path,'scoped-script-A','agent');b=Connection(self.socket_path,'scoped-script-B','agent')
        measured=None
        try:
            require(CAP in self.observer.observe()['capabilities'],'scope support required, no fallback')
            for client,label in [(a,'A'),(b,'B')]:
                r=client.call({'op':'create_session','name':'scope-'+label});require(r['ok'],'PTY creation')
            sessions=self.snapshot()['sessions'];sid_a,sid_b=[s['id'] for s in sessions]
            for client,sid in [(a,sid_a),(b,sid_b)]:
                s=next(s for s in self.snapshot()['sessions'] if s['id']==sid)
                require(client.call({'op':'take_control','session':sid,'generation':s['generation'],'force':False})['ok'],'initial writer claim')
            task=a.call({'op':'define_task','task':'scope-shared','goal':'Synthetic attributed work only','expected_artifacts':['artifact']})
            require(task['ok'],'task definition');revision=task['receipt']['result']['revision'];ids=[]
            for client,label in [(a,'A'),(b,'B')]:
                program="import pathlib,time,sys;print('SCOPE_%s_READY',flush=True);%s\np=pathlib.Path('ticks-%s')\nwhile not pathlib.Path('finish-%s').exists():\n with p.open('ab') as f:f.write(b'x')\n print('SCOPE_%s_TICK',file=sys.stderr,flush=True);time.sleep(.01)"%(label,"print('ONLY_B_STREAM_'*10000,flush=True)" if label=='B' else '',label,label,label)
                r=client.call({'op':'run','task':'scope-shared','task_revision':revision,'argv':['/usr/bin/python3','-u','-c',program]});require(r['ok'],'managed launch');ids.append(r['receipt']['result']['attempt'])
            aid,bid=ids
            self.wait_snapshot(lambda s:len(s['attempts'])==2 and all('READY' in x['stdout'] for x in s['attempts']) and next(x for x in s['attempts'] if x['id']==bid)['truncated'],'separate attributed processes and bounded B flood')
            payload=self.workspace/'artifact';payload.write_bytes(b'A'*1048576)
            require(a.call({'op':'artifact','task':'scope-shared','path':'artifact'})['ok'],'artifact registration')
            # Bootstrap/discovery is intentionally full and outside timing samples.
            measured=MeasuredConnection(self.socket_path,'scope-meter','agent')
            require(CAP in measured.observe()['capabilities'],'measured endpoint did not negotiate')
            before_size=Path(str(self.socket_path)+'.journal').stat().st_size
            samples={'legacy':[],'work':[]}
            for n in range(110):
                order=('legacy','work') if n%2==0 else ('work','legacy')
                for kind in order:
                    if kind=='legacy': measured.observe()
                    else:
                        v=scoped(measured,'work')
                        raw=json.dumps(v)
                        require(all(k not in raw for k in ['"screen"','"stdout"','"stderr"','"artifacts"','ONLY_B_STREAM_']),'work scope leaked unrequested content')
                    if n>=10:samples[kind].append({**measured.last_sample,'pair':n-10,'order':list(order)})
            require(Path(str(self.socket_path)+'.journal').stat().st_size==before_size,'observation journaled a mutation')
            require(max(s['response_wire_bytes'] for s in samples['work'])<min(s['response_wire_bytes'] for s in samples['legacy']),'work scope did not remove large unrequested payload')
            self.record['measurement']={'workload':'one Ubuntu shared-VM broker/worker,2quiet PTYs,2scripted managed loops (B flooded to existing cap),4live connections,1registered synthetic1MiB artifact; captureoff','samples':samples,'summary':{k:quantiles(v) for k,v in samples.items()},'warmup':'10paired AB/BA iterations excluded;100pairs retained','scope':'Python client encode/send/read/decode roundtrip plus actual response wire line octets; not server-only CPU, human input/paint, model latency or fleet/SLO qualification','host':{'kernel':platform.release(),'machine':platform.machine(),'logical_cpus':os.cpu_count()},'build':'Supported make scoped-test / aggregate uses Cargo dev debug binaries; hashes/source observations identify this run, not a compiler attestation', 'limits':'Shared VM/background noise uncontrolled; no peak/all-process resource or fairness claim. Legacy samples include documented1MiBartifact rehash; scoped work omits file validation.'}
            # Release fourth live slot before one-shot CLI probes. Each one-shot
            # bootstraps Observe and retains one identity; not the measured path.
            measured.close();measured=None
            self.wait_snapshot(lambda s:sum(e['connected'] for e in s['endpoints'])==3,'measurement endpoint detached before CLI admission')
            for command,target in [('work',None),('session',sid_a),('attempt',aid)]:
                args=[str(self.bin_dir/'namtab-agent'),'--socket',str(self.socket_path),command]+([target] if target else [])
                r=subprocess.run(args,capture_output=True,text=True,timeout=5);require(r.returncode==0,'actual scoped CLI failed: '+r.stderr)
                o=json.loads(r.stdout);require(o['profile']==PROFILE and o['view']['kind']==command,'CLI projection profile/kind')
                if target:require(o['view'][command]['id']==target,'CLI target crossed')
                self.wait_snapshot(lambda s:sum(e['connected'] for e in s['endpoints'])==3,'one-shot CLI detached before next admission')
            for client,ident in [(a,aid),(b,bid)]:
                attempt=scoped(client,'attempt',ident)['attempt']
                require(attempt['origin']['endpoint']==client.endpoint and attempt['task_revision']==revision,'scope launch origin/revision mismatch')
                require(('ONLY_B_STREAM_' not in attempt['stdout']) if client==a else attempt['truncated'],'scope stream attribution/bound')
                label,other=('A','B') if client==a else ('B','A')
                require('SCOPE_'+label+'_READY' in attempt['stdout'] and 'SCOPE_'+other+'_READY' not in attempt['stdout'] and 'SCOPE_'+label+'_TICK' in attempt['stderr'] and 'SCOPE_'+other+'_TICK' not in attempt['stderr'],'selected stream canaries crossed')
            # Stale/unknown scopes are read refusals, never grants/receipts.
            for op in [{'op':'observe_scoped','epoch':'stale','scope':{'kind':'work'}},{'op':'observe_scoped','epoch':a.epoch,'scope':{'kind':'session','session':'unknown'}},{'op':'observe_scoped','epoch':a.epoch,'scope':{'kind':'attempt','attempt':'unknown'}}]:
                r=a.call(op);require(not r['ok'] and not r.get('observation') and not r['receipt'],'invalid selection granted data/receipt')
            sa=scoped(a,'session',sid_a)['session'];sb=scoped(b,'session',sid_b)['session'];tick_before=(self.workspace/'ticks-B').stat().st_size
            take=self.observer.call({'op':'take_control','session':sid_a,'generation':sa['generation'],'force':True});require(take['ok'],'human exact takeover')
            stale=a.call({'op':'input','session':sid_a,'generation':sa['generation'],'revision':sa['screen']['revision'],'mode_revision':sa['screen']['mode_revision'],'policy':'observed','intent':{'kind':'text','text':'SHOULD_NOT_RUN'}})
            require(not stale['ok'] and stale['receipt']['delivered_bytes']==0,'stale owner delivered bytes')
            current=scoped(self.observer,'session',sid_a)['session'];peer=scoped(b,'session',sid_b)['session']
            require(current['owner']==self.observer.endpoint and current['generation']>sa['generation'],'wrong takeover target')
            require(peer['owner']==sb['owner'] and peer['generation']==sb['generation'],'peer writer changed')
            self.wait(lambda:(self.workspace/'ticks-B').stat().st_size>tick_before+2,'B continued after takeover')
            for label in ['A','B']:(self.workspace/('finish-'+label)).write_bytes(b'finish')
            self.wait(lambda:all(x['state']=='exited' for x in scoped(self.observer,'work')['attempts']),'both direct attempts exit/drain')
            stable=self.snapshot()
            for attempt in stable['attempts']:
                require(scoped(self.observer,'attempt',attempt['id'])['attempt']==attempt,'quiescent attempt differs from legacy')
            for session in stable['sessions']:
                # The quiet shells receive no further input. Exact equality is
                # asserted only here, not between changing live-output cuts.
                require(scoped(self.observer,'session',session['id'])['session']==session,'quiet session differs from legacy')
            self.record['checks']=[{'name':n,'verdict':'PASS'} for n in ['capability_and_typed_scope','bounded_metadata_no_cells_streams_artifacts','four_connection_interleaved_samples','actual_CLI_bootstrap_profile','live_origin_no_cross_attribution','stale_unknown_scope_refusal','exact_human_takeover_peer_continues','stale_owner_zero_bytes','quiescent_payload_equivalence','read_does_not_journal_mutation']]
            self.record['outcomes']={'task_revision':revision,'origins':{x['id']:x['origin'] for x in stable['attempts']},'human_takeover_receipt':take['receipt'],'stale_receipt':stale['receipt'],'work_summary':scoped(self.observer,'work')}
        finally:
            if measured:measured.close()
            a.close();b.close()

    def run(self):
        status='FAIL'
        try:self.start();self.journey();status='PASS'
        except Exception as e:self.record['error']={'type':type(e).__name__,'message':str(e)};(self.artifacts/'failure.txt').write_text(traceback.format_exc())
        finally:
            try:self.cleanup()
            except Exception as e:status='FAIL';self.record['cleanup_error']={'type':type(e).__name__,'message':str(e)}
            self.record['source_after']=git_metadata(self.root)
            if self.record['source_after']!=self.record['source']:status='FAIL';self.record['source_changed']=True
            if 'executed_cli' in self.record:
                cli=self.record['executed_cli'];cli['sha256_after']=sha256(Path(cli['path']))
                cli['unchanged']=cli['sha256_after']==cli['sha256_before']
                if not cli['unchanged']:status='FAIL';self.record['cli_binary_changed']=True
            self.record['verdict']=status
            (self.artifacts/'checks.json').write_text(json.dumps(self.record,indent=2)+'\n')
        print(json.dumps({'verdict':status,'evidence':str(self.artifacts/'checks.json')}));return 0 if status=='PASS' else 1

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--terminfo',type=Path,default=Path('.'));p.add_argument('--artifacts',type=Path,required=True)
    args=p.parse_args();args.native=False;sys.exit(ScopedPull(args).run())
