#!/usr/bin/env python3
"""Request-driven latest-state watch proof; scripted owned clients, no models/GUI."""
import argparse
import array
import fcntl
import json
import os
from pathlib import Path
import socket
import threading
import time
from adapter import Adapter
from e2e import Connection, require, sha256, complete_journal_records
from human_admission import stale_fence
from load_evidence import execute
from load_metrics import owned_resources, RSS_CAP
from scoped_observation import MeasuredConnection, scoped

PROFILE='namtab.session-watch/0'


def checked(response, client, request, previous=None):
    require(response['ok'] and response.get('error') is None and all(response.get(k) is None for k in ['snapshot','observation','receipt','resume']),'watch response mixed/refused')
    f=response.get('watch');require(f and f['version']=='namtab.mk1/1' and f['profile']==PROFILE and f['epoch']==client.epoch and f['endpoint']==client.endpoint and not f['history_complete'],'watch header binding/history')
    c=f['cursor'];require(type(c['sample']) is int and 0<c['sample']<=9_007_199_254_740_991 and c['revision'].startswith('sha256:') and len(c['revision'])==71 and all(x in '0123456789abcdef' for x in c['revision'][7:]),'watch cursor')
    kind=request['kind'];body=f['body'];state=body['kind']
    allowed={'open':['baseline'],'next':['replaced','unchanged','resync_required'],'resync':['baseline'],'close':['closed']}
    require(state in allowed[kind],'unrequested baseline/state; resync must be explicit')
    if kind=='open':require(state=='baseline' and c['sample']==1 and f['session']==request['session'],'initial baseline')
    else:require(previous and f['subscription']==previous['subscription'] and f['session']==previous['session'],'subscription/target mismatch')
    if state in ['baseline','replaced','unchanged']:
        require(f['coverage']=='latest_published_view' and isinstance(f['canonical_pending'],bool),'missing sample completeness')
        if previous:require(c['sample']==previous['cursor']['sample']+1,'sample cut gap')
        if state!='unchanged':require(body['view']['id']==f['session'],'wrong selected view')
        if state=='replaced':require(kind=='next' and body['from']==request['cursor'] and c['revision']!=previous['cursor']['revision'],'replacement base/target')
        if state=='unchanged':require(kind=='next' and c['revision']==previous['cursor']['revision'] and f['canonical_pending']==previous['canonical_pending'] and f['degraded']==previous['degraded'] and set(body)=={'kind'},'unchanged projection contradiction')
    else:
        require(state in ['resync_required','closed'] and f['coverage']=='no_observation' and f['canonical_pending'] is None and f['degraded'] is None and set(body)=={'kind'},'gap/close may not attest data')
        require(c==previous['cursor'],'no-observation cursor advanced')
        require((state=='closed' and kind=='close') or (state=='resync_required' and kind=='next'),'wrong response state')
    return f


def applied(client, operation):
    rid=client.next_id;r=client.call(operation);receipt=r.get('receipt')
    require(r['ok'] and receipt and receipt['state']=='applied' and receipt['id']==rid and receipt['endpoint']==client.endpoint and receipt['operation']==operation['op'],'immutable applied receipt binding')
    return r


class WatchJourney(Adapter):
    def __init__(self,args):
        super().__init__(args);self.clients=[];self.record['profile']=PROFILE;self.calls=[];self.resources=[];self.faults=[];self.monitor=None;self.stop_monitor=threading.Event();self.root_pidfd=None
        self.fixture_files=[Path(__file__),*[Path(__file__).with_name(n) for n in ['adapter.py','native.py','mcp_client.py','owned_process.py','x11.py','engine_load.py','human_admission.py','load_evidence.py','load_metrics.py','load_payload.py','scoped_observation.py']],Path(__file__).parents[1]/'mk1/e2e.py']
    def run(self):return execute(self)
    def connect(self, label, resume=None, next_id=1):
        c=MeasuredConnection(self.socket_path,label,'agent',resume=resume,next_id=next_id);self.clients.append(c);return c
    def watch(self,c,request,previous=None):
        self.guard();rid=c.next_id;start=time.monotonic_ns();row={'endpoint':c.endpoint,'request_id':rid,'request_kind':request['kind'],'start_ns':start,'ok':False};self.calls.append(row)
        try:
            r=c.call({'op':'session_watch','epoch':c.epoch,'request':request});f=checked(r,c,request,previous)
            require(time.monotonic_ns()-start<=2_000_000_000,'functional watch2s gate')
            row.update(ok=True,response_kind=f['body']['kind'],session=f['session'],subscription=f['subscription'],cursor=f['cursor'],elapsed_ns=time.monotonic_ns()-start,response_wire_bytes=c.last_sample['response_wire_bytes'])
            return f
        except BaseException as e:
            row.update(elapsed_ns=time.monotonic_ns()-start,error=type(e).__name__+': '+str(e));raise
    def next(self,c,f):return self.watch(c,{'kind':'next','subscription':f['subscription'],'cursor':f['cursor'],'credit':1},f)
    def guard(self):
        require(not self.faults,'monitor fault: '+str(self.faults));require(time.monotonic()<self.deadline,'finite25s journey deadline');require(not (self.workspace/'pty-failure').exists(),'owned query/reply failure')
    def write(self,c,sid,intent,human=False):
        s=scoped(c,'session',sid)['session'];r=applied(c,{'op':'input','session':sid,'generation':s['generation'],'revision':s['screen']['revision'],'mode_revision':s['screen']['mode_revision'],'policy':'human_live' if human else 'observed','intent':intent});require(r['receipt']['delivered_bytes']==(len(intent['text'].encode()) if intent['kind']=='text' else 1),'exact input extent');return r
    def journey(self):
        self.deadline=time.monotonic()+25;self.record['admission']={'capability':'session_watch_v0','wire':'namtab.mk1/1','scope':'one session','active_watches_per_transport':1,'open_lifetime_per_transport':16,'credit_per_next':1,'reply_bytes_including_newline':131072,'ptys':2,'managed_attempts':2,'dispatch_connections_max':4,'accepted_auxiliary_max':1,'capture':'off','provider_requests':0,'history':'not retained; no push/deltas/waits'}
        self.record['limitations']=['Scripted same-UID declarations, not models/authenticated principals or physical human/GUI','Latest published view/cut is not a global event/OS/application causal cut; intermediate transitions not retained','Hashing selected scope still costs bounded work; unchanged wire omission is not CPU/latency superiority','20Hz sampled owned RSS misses peaks,1GiB guard is fixture-only, not global allocation enforcement','Functional2s control/read gates and finite cohort do not qualify fullH0/fairness/fleet capacity','Native/MCP consumers remain scoped/legacy pull; no provider export or shell integration']
        a=self.connect('watch-A');b=self.connect('watch-B')
        require('session_watch_v0' in a.observe()['capabilities'] and 'session_watch_v0' in b.observe()['capabilities'],'watch support not negotiated')
        task=applied(a,{'op':'define_task','task':'watch-shared','goal':'Synthetic watch identity/control proof','expected_artifacts':['artifact']});revision=task['receipt']['result']['revision']
        for c,label in [(a,'A'),(b,'B')]:applied(c,{'op':'create_session','name':'watch-'+label})
        sessions=self.snapshot()['sessions'];sa=next(s['id'] for s in sessions if s['name']=='watch-A');sb=next(s['id'] for s in sessions if s['name']=='watch-B')
        for c,sid in [(a,sa),(b,sb)]:
            self.wait_session(sid,self.at_prompt,'owned initial prompt');s=scoped(c,'session',sid)['session'];applied(c,{'op':'take_control','session':sid,'generation':s['generation'],'force':False})
        # Populate a quiet view so byte comparisons describe a declared workload,
        # not a promise that an empty-grid baseline is always larger than metadata.
        self.write(a,sa,{'kind':'text','text':"printf '%064d\\n' {1..22}"});self.wait_session(sa,lambda s:'1..22' in ''.join(s['screen']['lines']),'owned command echo');self.write(a,sa,{'kind':'key','key':'Enter','ctrl':False,'alt':False,'shift':False});self.wait_session(sa,self.at_prompt,'quiet populated prompt')
        (self.workspace/'payload.py').write_bytes(Path(__file__).with_name('load_payload.py').read_bytes());(self.workspace/'phase').write_text('noisy');(self.workspace/'artifact').write_bytes(b'A'*65536)
        origins={}
        for c,label in [(a,'A'),(b,'B')]:
            q=applied(c,{'op':'run','task':'watch-shared','task_revision':revision,'argv':['/usr/bin/python3','-u','payload.py','managed-'+label]});origins[q['receipt']['result']['attempt']]={'endpoint':c.endpoint,'request_id':q['receipt']['id']}
        applied(a,{'op':'artifact','task':'watch-shared','path':'artifact'})
        self.write(b,sb,{'kind':'text','text':'/usr/bin/python3 -u payload.py pty'});self.wait_session(sb,lambda s:'payload.py pty' in ''.join(s['screen']['lines']),'owned B command echo');self.write(b,sb,{'kind':'key','key':'Enter','ctrl':False,'alt':False,'shift':False});self.wait(lambda:(self.workspace/'pty-ready').exists() and (self.workspace/'ticks-B').exists(),'owned noisy processes')
        self.descendants();self.root_pidfd=os.pidfd_open(self.process.pid);descriptors={self.process.pid:self.root_pidfd,**self.owned_pidfds};require(len(descriptors)==7,'owned7process census')
        def monitor():
            try:
                while not self.stop_monitor.is_set():
                    r=owned_resources(descriptors);self.resources.append(r);require(r['rss_bytes']<=RSS_CAP,'sampled1GiBRSS guard');self.stop_monitor.wait(.05)
            except BaseException as e:self.faults.append(type(e).__name__+': '+str(e))
        self.monitor=threading.Thread(target=monitor);self.monitor.start()
        f=self.watch(a,{'kind':'open','session':sa});baseline_bytes=a.last_sample['response_wire_bytes'];g=self.watch(b,{'kind':'open','session':sb});self.record['initial_binding']={'A':{'endpoint':a.endpoint,'session':sa,'watch':f['subscription']},'B':{'endpoint':b.endpoint,'session':sb,'watch':g['subscription']},'task_revision':revision,'attempt_origins':origins}
        unchanged=[];changed_b=0
        # Combined steady Next rate<=10Hz,12samples per scope. No latency gate.
        for _ in range(12):
            f=self.next(a,f);require(f['body']['kind']=='unchanged','peer output contaminated quiet target');unchanged.append(a.last_sample['response_wire_bytes']);g=self.next(b,g);changed_b+=g['body']['kind']=='replaced';time.sleep(.2)
        require(changed_b>0 and max(unchanged)<baseline_bytes,'missing independent changes or workload byte suppression');self.record['wire_comparison']={'baseline_bytes':baseline_bytes,'unchanged_bytes':unchanged,'A_samples':12,'B_samples':12,'changed_B':changed_b,'meaning':'actual NDJSON response bytes inclnewline; no universal size/CPU claim'}
        # Valid raw wrong bases are protocol gaps, never implicit snapshots.
        old=f;f=self.next(a,f)
        gap=self.watch(a,{'kind':'next','subscription':f['subscription'],'cursor':old['cursor'],'credit':1},f);require(gap['body']['kind']=='resync_required','stale base did not gap')
        gap2=self.watch(a,{'kind':'next','subscription':f['subscription'],'cursor':f['cursor'],'credit':1},f);require(gap2['body']['kind']=='resync_required','gap latch bypassed')
        f=self.watch(a,{'kind':'resync','subscription':f['subscription']},f);require(f['body']['kind']=='baseline','resync missing baseline')
        bad=a.call({'op':'session_watch','epoch':a.epoch,'request':{'kind':'next','subscription':f['subscription'],'cursor':f['cursor'],'credit':0}});require(not bad['ok'] and bad['error']=='watch_request_invalid' and not bad.get('watch') and not bad['receipt'],'invalid credit accepted')
        stale_epoch=a.call({'op':'session_watch','epoch':'0'*64,'request':{'kind':'resync','subscription':f['subscription']}});require(not stale_epoch['ok'] and stale_epoch['error']=='watch_epoch_stale','stale epoch accepted')
        wrong=b.call({'op':'session_watch','epoch':b.epoch,'request':{'kind':'close','subscription':f['subscription']}});require(not wrong['ok'] and wrong['error']=='watch_unknown','cross-transport watch accepted')
        f=self.next(a,f)
        # Close only observation, retaining writer and ongoing process work.
        writer=scoped(a,'session',sa)['session']['owner'];self.watch(a,{'kind':'close','subscription':f['subscription']},f);require(scoped(a,'session',sa)['session']['owner']==writer,'watch close revoked writer')
        f=self.watch(a,{'kind':'open','session':sa});old_sub=f['subscription'];resume,next_id,endpoint=a.resume,a.next_id,a.endpoint;a.close();self.clients.remove(a)
        self.wait(lambda:not next(e for e in scoped(self.observer,'work')['endpoints'] if e['id']==endpoint)['connected'],'explicit transport disconnect')
        a=self.connect('watch-A-resume',resume=resume,next_id=next_id);require(a.endpoint==endpoint,'endpoint relabeled');unknown=a.call({'op':'session_watch','epoch':a.epoch,'request':{'kind':'next','subscription':old_sub,'cursor':f['cursor'],'credit':1}});require(not unknown['ok'] and unknown['error']=='watch_unknown','resume restored old subscription')
        f=self.watch(a,{'kind':'open','session':sa});require(f['subscription']!=old_sub and f['cursor']['sample']==1,'new transport did not reset baseline')
        # Fourth owned client stops consuming. Bound the burst; no inferred exact
        # kernel wait or subscriber-queue occupancy from unread bytes alone.
        stalled=self.connect('watch-unread');sf=self.watch(stalled,{'kind':'open','session':sb});stalled.socket.setsockopt(socket.SOL_SOCKET,socket.SO_RCVBUF,4096);frames=[];ids=[]
        for _ in range(127):
            rid=stalled.next_id;stalled.next_id+=1;ids.append(rid);frames.append(json.dumps({'version':'namtab.mk1/1','id':rid,'operation':{'op':'session_watch','epoch':stalled.epoch,'request':{'kind':'resync','subscription':sf['subscription']}}},separators=(',',':')).encode()+b'\n')
        batch=b''.join(frames);require(len(batch)<=32768,'unread request batch cap');stalled.socket.sendall(batch);pending=array.array('i',[0])
        def unread():fcntl.ioctl(stalled.socket.fileno(),0x541B,pending,True);return pending[0]
        self.wait(unread,'actual unread watch response',timeout=.5);self.record['stalled']={'endpoint':stalled.endpoint,'requests':ids,'batch_bytes':len(batch),'unread_bytes_start':pending[0],'responses_consumed':0,'frame_count':127,'batch_cap':32768,'limit':'intentionally unread, not exact kernel blocking proof'}
        interventions=[]
        for i in range(6):
            self.guard();before=scoped(self.observer,'session',sa)['session'];ticks=(self.workspace/'ticks-B').stat().st_size;started=time.monotonic_ns();r=applied(self.observer,{'op':'take_control','session':sa,'generation':before['generation'],'force':True});require(r['ok'] and r['receipt']['state']=='applied','human takeover refused');take_ns=time.monotonic_ns()-started;require(take_ns<2_000_000_000,'functional takeover2s gate')
            old_view=scoped(a,'session',sa)['session'];f=self.next(a,f);require(f['body']['kind']=='replaced' and f['body']['view']['owner']==self.observer.endpoint,'watch omitted takeover');
            stale_id=a.next_id;stale=a.call({'op':'input','session':sa,'generation':before['generation'],'revision':before['screen']['revision'],'mode_revision':before['screen']['mode_revision'],'policy':'observed','intent':{'kind':'text','text':'WATCH_STALE_CANARY'}});stale_fence(stale,a.endpoint,stale_id)
            command=f"printf x >> human-probes";self.write(self.observer,sa,{'kind':'text','text':command},human=True);self.write(self.observer,sa,{'kind':'key','key':'Enter','ctrl':False,'alt':False,'shift':False},human=True)
            self.wait(lambda:(self.workspace/'human-probes').exists() and (self.workspace/'human-probes').read_bytes()==b'x'*(i+1),'unique real human effect',timeout=2);self.wait(lambda:(self.workspace/'ticks-B').stat().st_size>ticks,'peer process progress',timeout=2);self.wait_session(sa,self.at_prompt,'quiet prompt after owned effect')
            s=scoped(self.observer,'session',sa)['session'];r=applied(self.observer,{'op':'release_control','session':sa,'generation':s['generation']});require(r['ok'],'human release');s=scoped(a,'session',sa)['session'];applied(a,{'op':'take_control','session':sa,'generation':s['generation'],'force':False});f=self.next(a,f);interventions.append({'takeover_ns':take_ns,'probe':i,'stale_bytes':0,'effect_bytes':i+1,'peer_progress':True})
        self.record['interventions']=interventions;stalled.close();self.clients.remove(stalled)
        self.wait(lambda:not next(e for e in scoped(self.observer,'work')['endpoints'] if e['id']==stalled.endpoint)['connected'],'unread endpoint cleanup')
        # Polls never refreshed/exported artifacts. Explicit export preserves
        # existing freshness semantics and consent boundary instead.
        (self.workspace/'artifact').write_bytes(b'B'*65536);export=scoped(a,'work_export');require(export['artifacts'][0]['revision']=='sha256:'+sha256(self.workspace/'artifact'),'explicit artifact freshness lost')
        self.watch(a,{'kind':'close','subscription':f['subscription']},f);self.watch(b,{'kind':'close','subscription':g['subscription']},g)
        for _ in range(15):
            x=self.watch(a,{'kind':'open','session':sa});self.watch(a,{'kind':'close','subscription':x['subscription']},x)
        cap=a.call({'op':'session_watch','epoch':a.epoch,'request':{'kind':'open','session':sa}});require(not cap['ok'] and cap['error']=='watch_open_capacity','watch lifetime bound expanded')
        self.stop_monitor.set();self.monitor.join(timeout=3);require(not self.monitor.is_alive() and not self.faults and len(self.resources)>=2,'resource monitor failed')
        (self.workspace/'finish').write_bytes(b'finish');self.wait_snapshot(lambda s:len(s['attempts'])==2 and all(x['state']=='exited' for x in s['attempts']),'both exact attempts drained')
        work=scoped(a,'work');attempts=work['attempts'];require({x['id'] for x in attempts}==set(origins) and len(attempts)==2,'missing/duplicate result population')
        for x in attempts:
            v=scoped(a,'attempt',x['id'])['attempt'];require(v['origin']==origins[x['id']] and v['task_revision']==revision and v['exit_code']==0,'attempt attribution/exit changed');own='ONLY_A_' if v['origin']['endpoint']==endpoint else 'ONLY_B_';other='ONLY_B_' if own=='ONLY_A_' else 'ONLY_A_';require(own in v['stdout'] and other not in v['stdout'],'stream cross-attribution')
        self.wait_session(sb,self.at_prompt,'B controlled query program clean return');require((self.workspace/'replies.jsonl').exists(),'no actual terminal queries')
        replies=[json.loads(line) for line in (self.workspace/'replies.jsonl').read_text().splitlines()];require([x['seq'] for x in replies]==list(range(1,len(replies)+1)) and all(x['reply_hex']=='1b5b333b3752' for x in replies),'single exact query reply stream')
        records=complete_journal_records(Path(str(self.socket_path)+'.journal').read_bytes());reply_records=[x for x in records if x['operation']=='query_reply'];require(sum(x['state']=='applied' for x in reply_records)==len(replies) and sum(x['state']=='reserved' for x in reply_records)==len(replies),'duplicate or missing reply authority');self.record['query_count']=len(replies);require(not any(x['operation']=='session_watch' for x in records),'watch generated mutation journal');require(all('WATCH_STALE_CANARY' not in l for s in self.snapshot()['sessions'] for l in s['screen']['lines']),'stale bytes echoed')
        self.record.update(calls=self.calls,resources=self.resources,monitor_faults=self.faults,checks=[{'name':n,'verdict':'PASS'} for n in ['typed_exact_scope_credit_cursor_and_history','peer_changes_do_not_change_quiet_subscription','unchanged_omits_body_measured_wire_bytes','stale_base_latches_explicit_resync_no_silent_snapshot','cross_transport_epoch_credit_and_open_capacity_refusals','close_and_disconnect_do_not_cancel_or_restore_old_watch','unread_consumer_human_control_stalezero_peerprogress','two_exact_managed_origins_and_streams','explicit_artifact_export_freshness_and_no_watch_journal']]);self.guard()
    def cleanup(self):
        self.stop_monitor.set();faults=[]
        try:
            if self.monitor:
                self.monitor.join(timeout=3)
                if self.monitor.is_alive():faults.append('monitor remained live')
        finally:
            self.record.setdefault('calls',self.calls);self.record.setdefault('resources',self.resources);self.record['monitor_faults']=self.faults
            try:
                observer=self.observer;self.observer=None
                for c in [*self.clients,*([observer] if observer else [])]:
                    try:c.close()
                    except Exception as e:faults.append(type(e).__name__)
                self.clients=[]
                if self.root_pidfd is not None:
                    try:os.close(self.root_pidfd)
                    except OSError as e:faults.append(type(e).__name__)
                    self.root_pidfd=None
            finally:super().cleanup()
        require(not faults,'owned cleanup faults: '+str(faults))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--terminfo',type=Path,default=Path('.'));p.add_argument('--artifacts',type=Path,required=True);p.set_defaults(native=False)
    raise SystemExit(WatchJourney(p.parse_args()).run())
