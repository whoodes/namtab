#!/usr/bin/env python3
"""Bounded engine qualification; scripted public clients, no model/native GUI."""
import argparse
import json
import os
import select
from pathlib import Path
import socket
import array
import fcntl
import termios
import threading
import time
from adapter import Adapter
from e2e import Connection,require,sha256,complete_journal_records
from scoped_observation import scoped
from load_evidence import execute
from load_metrics import PROFILE,PHASES,RSS_CAP,SERVICE_NS,assess,assess_workload,owned_resources

class Timed(Connection):
    def __init__(self,*a,**kw):self.samples=[];super().__init__(*a,**kw)
    def call(self,op,request_id=None):
        started=time.monotonic_ns();sample={'op':op['op'],'request_id':self.next_id if request_id is None else request_id,'start_ns':started};self.samples.append(sample)
        try:
            r=super().call(op,request_id);sample.update(elapsed_ns=time.monotonic_ns()-started,ok=r['ok'],receipt_state=(r.get('receipt')or{}).get('state'));self.last=sample;return r
        except BaseException as e:sample.update(elapsed_ns=time.monotonic_ns()-started,ok=False,error=type(e).__name__);raise

def applied(client,op):
    r=client.call(op);receipt=r.get('receipt');require(r['ok'] and receipt and receipt['state']=='applied' and receipt['endpoint']==client.endpoint and receipt['id']==client.last['request_id'] and receipt['operation']==op['op'],'applied receipt binding');return r

class EngineLoad(Adapter):
    def __init__(self,args):
        super().__init__(args);self.qualify=args.qualify;self.a=None;self.b=None;self.agent_lock=threading.Lock();self.stop_threads=threading.Event();self.faults=[];self.resources=[];self.reads=[];self.threads=[];self.record['profile']=PROFILE;self.record['interventions']=[];self.window=None;self.x=None;self.sink=None;self.root_pidfd=None;self.fixture_files=[Path(__file__),Path(__file__).with_name('load_metrics.py'),Path(__file__).with_name('load_evidence.py'),Path(__file__).with_name('load_payload.py'),Path(__file__).with_name('adapter.py'),Path(__file__).with_name('native.py'),Path(__file__).with_name('scoped_observation.py'),Path(__file__).with_name('mcp_client.py'),Path(__file__).with_name('owned_process.py'),Path(__file__).with_name('x11.py'),Path(__file__).parents[1]/'mk1/e2e.py']
    def run(self):return execute(self)
    def start(self):
        self.deadline=time.monotonic()+45
        available=next(int(l.split()[1])*1024 for l in Path('/proc/meminfo').read_text().splitlines() if l.startswith('MemAvailable:'))
        require(available>=2*1024**3,'insufficient memory before starting owned processes')
        super().start()
    def guard(self):
        require(not self.faults,'owned monitor/client failed: '+str(self.faults));require(time.monotonic()<self.deadline,'finite fixture45s deadline');require(not (self.workspace/'pty-failure').exists(),'controlled PTY query failure')
    def observe(self,scope,target=None):
        self.guard();return scoped(self.observer,scope,target)
    def loop_reads(self):
        try:
            n=0
            while not self.stop_threads.is_set():
                with self.agent_lock:
                    scope,target=[('work_export',None),('session',self.sid),('attempt',self.aid)][n%3]
                    stage=self.current_stage;started=time.monotonic_ns();row={'repetition':stage[0],'phase':stage[1],'scope':scope,'start_ns':started,'ok':False};self.reads.append(row);v=scoped(self.a,scope,target);row.update(elapsed_ns=time.monotonic_ns()-started,ok=True)
                    if scope=='attempt':require(v['attempt']['origin']==self.origins[self.aid] and v['attempt']['task_revision']==self.revision,'read attribution')
                n+=1;self.stop_threads.wait(.1)
        except BaseException as e:self.faults.append('reader:'+type(e).__name__+':'+str(e))
    def loop_resources(self):
        try:
            while not self.stop_threads.is_set():
                row=owned_resources({self.process.pid:self.root_pidfd,**self.owned_pidfds});self.resources.append(row)
                require(row['rss_bytes']<=RSS_CAP,'sampled1GiBRSS guard');self.stop_threads.wait(.05)
        except BaseException as e:self.faults.append('resources:'+type(e).__name__+':'+str(e))
    def session(self,sid):
        result=self.observe('session',sid)['session'];self.record.setdefault('last_session_views',{})[sid]=result;return result
    def probe(self,rep,phase,cycle,warmup=False):
        self.guard();sample={'repetition':rep,'phase':phase,'cycle':cycle,'ok':False}
        if not warmup:self.record['interventions'].append(sample)
        before=self.session(self.sid);oldgen=before['generation'];ticks={k:(self.workspace/('ticks-'+k)).stat().st_size for k in ['A','B']}
        started=time.monotonic_ns();sample['start_ns']=started;applied(self.observer,{'op':'take_control','session':self.sid,'generation':oldgen,'force':True});sample['takeover_ns']=time.monotonic_ns()-started
        sample['stale_bytes']=self.stale_input(before)
        current=self.session(self.sid);token=f'{rep}:{phase}:{cycle}' if not warmup else f'warmup:{cycle}';old=(self.workspace/'probes').read_text().splitlines() if (self.workspace/'probes').exists() else [];command="printf '%s\\n' '"+token+"' >> probes"
        def write(intent):
            s=self.session(self.sid);started=time.monotonic_ns();r=applied(self.observer,{'op':'input','session':self.sid,'generation':s['generation'],'revision':s['screen']['revision'],'mode_revision':s['screen']['mode_revision'],'policy':'human_live','intent':intent});return time.monotonic_ns()-started,r
        sample['text_ns'],typed=write({'kind':'text','text':command});require(typed['receipt']['delivered_bytes']==len(command.encode()),'positive text extent');started=time.monotonic_ns();sample['enter_ns'],entered=write({'kind':'key','key':'Enter','ctrl':False,'alt':False,'shift':False});require(entered['receipt']['delivered_bytes']==1,'positive Enter extent')
        self.wait(lambda:(self.workspace/'probes').exists() and (self.workspace/'probes').read_text().splitlines()==old+[token],'ordered unique real probe effect',timeout=2);sample['probe_ns']=time.monotonic_ns()-started;sample['probe_exact']=True
        self.wait(lambda:all((self.workspace/('ticks-'+k)).stat().st_size>ticks[k] for k in ['A','B']),'both independent jobs forward progress',timeout=2);sample['peer_progress_a']=True;sample['peer_progress_b']=True
        current=self.session(self.sid);require(all('STALE_LOAD_CANARY' not in line for line in current['screen']['lines']),'stale bytes echoed into canonical session')
        applied(self.observer,{'op':'release_control','session':self.sid,'generation':current['generation']});current=self.session(self.sid)
        with self.agent_lock:applied(self.a,{'op':'take_control','session':self.sid,'generation':current['generation'],'force':False})
        sample['ok']=True
    def stale_input(self,before):
        with self.agent_lock:
            request_id,endpoint=self.a.next_id,self.a.endpoint
            stale=self.a.call({'op':'input','session':self.sid,'generation':before['generation'],'revision':before['screen']['revision'],'mode_revision':before['screen']['mode_revision'],'policy':'observed','intent':{'kind':'text','text':'STALE_LOAD_CANARY'}})
        # Another poll can now update Timed.last. Only this immutable transaction
        # may establish the zero-effect refusal, never that mutable diagnostic.
        receipt=stale.get('receipt')
        require(not stale['ok'] and stale['id']==request_id and receipt and receipt['endpoint']==endpoint and receipt['id']==request_id and receipt['operation']=='input' and receipt['state']=='rejected' and receipt['delivered_bytes']==0,'stale owner effect/receipt')
        return receipt['delivered_bytes']
    def unread_b(self):
        pending=array.array('i',[0]);fcntl.ioctl(self.b.socket.fileno(),termios.FIONREAD,pending,True);return pending[0]
    def stall(self):
        self.b.socket.setsockopt(socket.SOL_SOCKET,socket.SO_RCVBUF,4096);self.stalled_requests=[]
        for _ in range(8):
            rid=self.b.next_id;self.b.next_id+=1;body=json.dumps({'version':'namtab.mk1/1','id':rid,'operation':{'op':'observe'}}).encode()+b'\n';self.b.socket.sendall(body);self.stalled_requests.append(rid)
        self.wait(lambda:self.unread_b()>0,'owned stalled consumer has unread bytes',timeout=2)
        self.record.setdefault('stalls',[]).append({'endpoint':self.b.endpoint,'requests':self.stalled_requests,'read_responses':0,'unread_bytes_start':self.unread_b(),'socket_rcvbuf_actual':self.b.socket.getsockopt(socket.SOL_SOCKET,socket.SO_RCVBUF),'meaning':'intentional stopped consumer; no inference of exact kernel wait; requests are read-only'})
    def resume_b(self):
        endpoint=self.b.endpoint;resume=self.b.resume;next_id=self.b.next_id;self.b.close();self.b=None
        self.wait(lambda:not next(e for e in self.observe('work')['endpoints'] if e['id']==endpoint)['connected'],'stalled connection detached',timeout=2)
        self.b=Timed(self.socket_path,'script-B-resume','agent',resume=resume,next_id=next_id);self.record['client_samples']['B'].append(self.b.samples);require(self.b.endpoint==endpoint,'explicit B resume relabeled origin');self.discover(self.b);require(scoped(self.b,'attempt',self.bid)['attempt']['origin']==self.origins[self.bid],'resumed B positive origin/read proof')
    def discover(self,client):
        d=client.observe();require(d['epoch']==client.epoch and d['endpoint']==client.endpoint and all(c in d['capabilities'] for c in ['shared_tasks_v1','attempt_origin_v1','scoped_observe_v0','scoped_work_export_v0']),'required scoped/task capabilities absent; no fallback')
    def journey(self):
        self.current_phase='warmup';self.current_stage=(-1,'warmup');self.record['runtime_admission']={'capture':'off','live_connections':3,'declared_scripted_agents':2,'managed_attempts':2,'ptys':2,'model_requests':'not invoked; external unknown','subscriptions':0,'capacity_expansion':False}
        self.a=Timed(self.socket_path,'load-script-A','agent');self.b=Timed(self.socket_path,'load-script-B','agent')
        # Replace only this fixture's read observer with timed metadata; reuse its live identity in RAM.
        old=self.observer;endpoint,resume,next_id=old.endpoint,old.resume,old.next_id;old.close();self.observer=None;time.sleep(.03);self.observer=Timed(self.socket_path,'load-human','human',resume=resume,next_id=next_id);require(self.observer.endpoint==endpoint,'observer identity changed');self.record['client_samples']={'human':[self.observer.samples],'A':[self.a.samples],'B':[self.b.samples]}
        self.discover(self.a);self.discover(self.b);self.discover(self.observer)
        sources=[Path(__file__),Path(__file__).with_name('load_metrics.py'),Path(__file__).with_name('load_payload.py')];self.record['source_hashes']={p.name:sha256(p) for p in sources}
        for p in sources:(self.artifacts/p.name).write_bytes(p.read_bytes())
        (self.workspace/'payload.py').write_bytes(Path(__file__).with_name('load_payload.py').read_bytes());(self.workspace/'phase').write_text('quiet');(self.workspace/'artifact').write_bytes(b'A'*65536)
        task=applied(self.a,{'op':'define_task','task':'load-shared','goal':'Synthetic engine-load control qualification','expected_artifacts':['artifact']});self.revision=task['receipt']['result']['revision']
        for client,name in [(self.a,'A'),(self.b,'B')]:applied(client,{'op':'create_session','name':name})
        sessions=self.snapshot()['sessions'];self.sid=next(s['id'] for s in sessions if s['name']=='A');bsid=next(s['id'] for s in sessions if s['name']=='B')
        for client,sid in [(self.a,self.sid),(self.b,bsid)]:
            self.wait_session(sid,self.at_prompt,'controlled initial prompt');s=self.session(sid);applied(client,{'op':'take_control','session':sid,'generation':s['generation'],'force':False})
        ids=[]
        for client,label in [(self.a,'A'),(self.b,'B')]:ids.append(applied(client,{'op':'run','task':'load-shared','task_revision':self.revision,'argv':['/usr/bin/python3','-u','payload.py','managed-'+label]})['receipt']['result']['attempt'])
        self.aid,bid=ids;self.bid=bid;origins={self.aid:{'endpoint':self.a.endpoint,'request_id':next(x['request_id'] for x in self.a.samples if x['op']=='run')},bid:{'endpoint':self.b.endpoint,'request_id':next(x['request_id'] for x in self.b.samples if x['op']=='run')}};self.origins=origins;self.record['attribution']={'attempt_origins':origins,'task_revision':self.revision}
        s=self.session(bsid);applied(self.b,{'op':'input','session':bsid,'generation':s['generation'],'revision':s['screen']['revision'],'mode_revision':s['screen']['mode_revision'],'policy':'observed','intent':{'kind':'text','text':'/usr/bin/python3 -u payload.py pty'}})
        self.wait_session(bsid,lambda x:'payload.py pty' in ''.join(x['screen']['lines']),'B command text echo');s=self.session(bsid);applied(self.b,{'op':'input','session':bsid,'generation':s['generation'],'revision':s['screen']['revision'],'mode_revision':s['screen']['mode_revision'],'policy':'observed','intent':{'kind':'key','key':'Enter','ctrl':False,'alt':False,'shift':False}})
        self.wait(lambda:(self.workspace/'pty-ready').exists() and (self.workspace/'ticks-B').exists(),'actual owned jobs ready')
        applied(self.a,{'op':'artifact','task':'load-shared','path':'artifact'})
        self.descendants();require(self.process.poll() is None,'owned broker exited');self.root_pidfd=os.pidfd_open(self.process.pid);require(self.process.poll() is None,'owned broker died during pidfd capture');self.record['owned_processes_at_start']=[{'pid':pid,'name':(Path('/proc')/str(pid)/'comm').read_text().strip()} for pid in self.owned_pidfds if (Path('/proc')/str(pid)/'comm').exists()]
        self.expected=owned_resources({self.process.pid:self.root_pidfd,**self.owned_pidfds})['identities'];self.record['expected_processes']=self.expected;require(len(self.expected)==7,'admitted process census is broker+6supervisedchildren');self.windows=[];self.record['stage_windows']=self.windows;self.phase_progress=[];self.record['phase_progress']=self.phase_progress
        self.record['owned_executables']=[]
        for pid,fd in {self.process.pid:self.root_pidfd,**self.owned_pidfds}.items():
            require(not select.select([fd],[],[],0)[0],'owned process exited before provenance sample');exe=(Path('/proc')/str(pid)/'exe').resolve(strict=True);metadata={'pid':pid,'path':str(exe),'sha256':sha256(exe)};require(not select.select([fd],[],[],0)[0],'owned process exited during provenance read');self.record['owned_executables'].append(metadata)
        self.record['binary_hashes_before']=dict(self.record['binary_hashes'])
        self.record['host']={'kernel':os.uname().release,'machine':os.uname().machine,'cpus':os.cpu_count(),'affinity':sorted(os.sched_getaffinity(0)),'loadavg':os.getloadavg(),'meminfo':{l.split(':')[0]:int(l.split()[1])*1024 for l in Path('/proc/meminfo').read_text().splitlines() if l.startswith(('MemTotal:','MemAvailable:'))}}
        require(self.record['host']['meminfo']['MemAvailable']>=2*1024**3,'insufficient available memory for shared-host experiment')
        for i in range(2):self.probe(-1,'warmup',i,warmup=True)
        for target in [self.loop_reads,self.loop_resources]:t=threading.Thread(target=target);self.threads.append(t);t.start()
        self.wait(lambda:bool(self.resources),'resource monitor ready',timeout=2)
        for rep in range(3):
            for phase in PHASES:
                self.current_phase=phase;self.current_stage=(rep,phase);queries_before=len((self.workspace/'replies.jsonl').read_text().splitlines()) if (self.workspace/'replies.jsonl').exists() else 0;(self.workspace/'phase').write_text(phase);
                if phase!='quiet':self.wait(lambda:(self.workspace/'replies.jsonl').exists() and len((self.workspace/'replies.jsonl').read_text().splitlines())>queries_before,'actual noisy terminal query before samples',timeout=2)
                started=time.monotonic();tick=(self.workspace/'ticks-B').stat().st_size
                if phase=='stalled':self.stall()
                window={'repetition':rep,'phase':phase,'start_ns':time.monotonic_ns()};self.windows.append(window);progress={'repetition':rep,'phase':phase,'a_start':(self.workspace/'ticks-A').stat().st_size,'b_start':(self.workspace/'ticks-B').stat().st_size};self.phase_progress.append(progress)
                for cycle in range(6):
                    while time.monotonic()-started<cycle*.125:self.guard();time.sleep(.005)
                    self.probe(rep,phase,cycle)
                while time.monotonic()-started<.75:self.guard();time.sleep(.02)
                require((self.workspace/'ticks-B').stat().st_size>tick,'phase peer made no progress')
                window['end_ns']=time.monotonic_ns();progress.update(a_end=(self.workspace/'ticks-A').stat().st_size,b_end=(self.workspace/'ticks-B').stat().st_size)
                if phase=='stalled':
                    self.record['stalls'][-1]['unread_bytes_end']=self.unread_b();self.resume_b()
        self.stop_threads.set()
        for t in self.threads:t.join(timeout=3);require(not t.is_alive(),'owned monitor remained active')
        self.guard();(self.workspace/'finish').write_bytes(b'finish');self.wait_snapshot(lambda s:all(a['state']=='exited' for a in s['attempts']),'managed attempts exited/drained');self.wait_session(bsid,self.at_prompt,'PTY workload returned to owned shell')
        work=self.observe('work');require(all(a['origin']==origins[a['id']] and a['task_revision']==self.revision for a in work['attempts']),'post-resume origin/task changed')
        details={}
        for attempt in work['attempts']:
            detail=self.observe('attempt',attempt['id'])['attempt'];details[attempt['id']]=detail;other='ONLY_B_' if attempt['id']==self.aid else 'ONLY_A_';require(other not in detail['stdout'],'cross-attributed stream')
        assess_workload(work['attempts'],details,origins,self.revision,self.phase_progress,{self.aid:'ONLY_A_',self.bid:'ONLY_B_'})
        replies=[json.loads(l) for l in (self.workspace/'replies.jsonl').read_text().splitlines()];require(replies and [r['seq'] for r in replies]==list(range(1,len(replies)+1)) and all(r['reply_hex']=='1b5b333b3752' for r in replies),'terminal query ordering')
        records=complete_journal_records(Path(str(self.socket_path)+'.journal').read_bytes());reply_records=[r for r in records if r['operation']=='query_reply'];require(sum(r['state']=='applied' for r in reply_records)==len(replies) and sum(r['state']=='reserved' for r in reply_records)==len(replies),'single reply authority/journal extent')
        filler=Connection(self.socket_path,'load-cap-filler','agent')
        newcomer=None
        try:
            fifth=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);fifth.settimeout(2);fifth.connect(str(self.socket_path))
            try:
                fifth.sendall(b'{"version":"namtab.mk1/1","id":1,"operation":{"op":"hello","label":"new-agent","role":"agent","resume":null}}\n')
                response=json.loads(fifth.makefile('rb').readline(1_048_577))
                require(not response['ok'] and response['error']=='control_admission_requires_human_or_lifecycle_hello','fifth Agent admitted')
            finally:fifth.close()
            newcomer=Connection(self.socket_path,'load-fresh-human','human')
            require(bool(scoped(newcomer,'work')) and bool(self.observe('work')),'fresh or existing Human path failed')
            self.record['full_slots_probe']={'outcome':'four-dispatch-plus-one-bounded-handoff','fresh_human_observe_ok':True,'existing_human_observe_ok':True,'limit':'Same-UID role declaration. Only eligible unbound/Agent idle-read transport replaced; no fifth dispatch slot or universal admission guarantee.'}
        finally:
            if newcomer:newcomer.close()
            filler.close()
        self.record.update(read_samples=self.reads,resources=self.resources,query_count=len(replies),human_call_samples=self.observer.samples,agent_a_call_samples=self.a.samples,agent_b_call_samples=self.b.samples)
        self.record['binary_hashes_after']={n:sha256(self.bin_dir/n) for n in self.record['binary_hashes_before']};require(self.record['binary_hashes_after']==self.record['binary_hashes_before'],'product binary changed during qualification')
        result=assess(self.record['interventions'],self.resources,self.reads,self.expected,self.windows);self.record['read_takeover_overlap_count']=sum(any(r['start_ns']<x['start_ns']+x['takeover_ns'] and r['start_ns']+r['elapsed_ns']>x['start_ns'] for r in self.reads) for x in self.record['interventions']);self.record['assessment']=result;self.record['qualification']='NOT_REQUESTED_CI_FUNCTIONAL_ONLY'
        if self.qualify:
            require(self.record['host']['cpus']==4 and self.record['host']['kernel']=='6.8.0-138-generic' and self.record['host']['machine']=='x86_64' and self.record['host']['meminfo']['MemTotal']==16721039360 and self.record['host']['affinity']==[0,1,2,3],'qualification host tuple differs; not admitted');require(result['control_candidate']=='PASS','predeclared100mscontrolp95gateFAILED');self.record['qualification']='PASS_NARROW_RECORDED_HOST_ONLY'
        self.record['checks'].extend({'name':n,'verdict':'PASS'} for n in ['finite_source_bound_profile_no_cap_increase','54ordered_human_probes_and_stale_zero_byte_fences','two_exact_attempt_origins_stream_exclusion_peer_progress','scoped_read_progress_and_stalled_reader_resume_no_replay','single_ordered_terminal_reply_authority','bounded_sampled_owned_process_memory','functional_service_deadlines_preserve_all_denominators'])
        self.record['limitations']=['Scripted public clients, same-UID declared human, no actual model/principal or physicalhuman/GUIpaint','54interventions/18percohort cannot qualify raretail/SLO/fleet fairness','RSS20Hz sum not instantaneous peak/allocation enforcement; source finite queues not measured scheduling proof','Stalledconsumer reads nothing; no exact kernel wait inferred; explicit resume carries RAM-only fixture cap','No subscriptions/history/deltas,production scheduler,capacity expansion or broadH0 acceptance']
    def cleanup(self):
        self.stop_threads.set()
        try:
            if any(t.is_alive() for t in self.threads):
                for c in [self.a,self.b]:
                    if c:
                        try:c.socket.shutdown(socket.SHUT_RDWR)
                        except OSError:pass
            for t in self.threads:t.join(timeout=3)
            require(not any(t.is_alive() for t in self.threads),'owned monitor remained alive at cleanup')
        finally:
            self.record.setdefault('read_samples',self.reads);self.record.setdefault('resources',self.resources);self.record['monitor_faults']=self.faults
            try:
                if self.a:self.a.close();self.a=None
                if self.b:self.b.close();self.b=None
            finally:
                if self.root_pidfd is not None:os.close(self.root_pidfd);self.root_pidfd=None
                super().cleanup()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--terminfo',type=Path,default=Path('.'));p.add_argument('--artifacts',type=Path,required=True);p.add_argument('--qualify',action='store_true');p.set_defaults(native=False)
    raise SystemExit(EngineLoad(p.parse_args()).run())
