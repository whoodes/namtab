#!/usr/bin/env python3
"""Finite owned same-UID admission journey. No models, GUI or user resources."""
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
from engine_load import Timed, applied
from load_evidence import execute
from load_metrics import owned_resources, RSS_CAP
from scoped_observation import scoped


def stale_fence(response, endpoint, request_id):
    r=response.get('receipt')
    require(not response['ok'] and response['id']==request_id and response.get('error')=='writer_or_generation_stale' and r and r['endpoint']==endpoint and r['id']==request_id and r['operation']=='input' and r['state']=='rejected' and r['detail']=='writer_or_generation_stale' and r['delivered_bytes']==0,'expected exact stale-writer zero-byte receipt, not another rejection')


def result_population(attempts, origins):
    ids=[a['id'] for a in attempts]
    require(len(ids)==2 and len(set(ids))==2 and set(ids)==set(origins),'missing/duplicate/extra attributed attempt')


def arrival_binding(response, epoch):
    require(response['id']==1 and response['epoch']==epoch,'concurrent Hello correlation/epoch mismatch')
    if response['ok']:
        require(isinstance(response.get('endpoint'),str) and bool(response['endpoint']) and isinstance(response.get('resume'),str) and bool(response['resume']),'missing Hello binding')
    else:
        require(response.get('endpoint') is None and response.get('resume') is None and response.get('error') in ['control_admission_no_eligible_transport','control_admission_state_busy'],'unexpected concurrent refusal')


class Admission(Adapter):
    def __init__(self, args):
        super().__init__(args)
        self.clients = []
        self.samples = []
        self.raw_sockets = []
        self.stop_monitor = threading.Event()
        self.monitor = None
        self.monitor_errors = []
        self.resources = []
        self.root_pidfd = None
        self.record['profile'] = 'namtab.human-admission/0'
        self.fixture_files = [Path(__file__), *[Path(__file__).with_name(n) for n in ['engine_load.py', 'load_payload.py', 'load_metrics.py', 'load_evidence.py', 'adapter.py', 'native.py', 'scoped_observation.py', 'mcp_client.py', 'owned_process.py', 'x11.py']], Path(__file__).parents[1]/'mk1/e2e.py']

    def run(self):
        return execute(self)

    def connect(self, role='agent', **kwargs):
        c = Timed(self.socket_path, 'synthetic-admission-'+role, role, **kwargs)
        self.clients.append(c)
        return c

    def work(self, c):
        return scoped(c, 'work')

    def journal(self):
        return complete_journal_records(Path(str(self.socket_path)+'.journal').read_bytes())

    def raw(self, operation):
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(3)
        s.connect(str(self.socket_path))
        try:
            s.sendall(json.dumps({'version':'namtab.mk1/1','id':1,'operation':operation}).encode()+b'\n')
            return json.loads(s.makefile('rb').readline(1_048_577))
        finally:
            s.close()

    def hello(self, role='human', resume=None):
        return {'op':'hello', 'label':'synthetic-candidate', 'role':role, 'resume':resume}

    def sockets(self):
        # Linux per-process live socket inode sample, clones deduplicated.
        inodes = set()
        for p in (Path('/proc')/str(self.process.pid)/'fd').iterdir():
            try:
                value = os.readlink(p)
            except FileNotFoundError:
                continue
            if value.startswith('socket:['):
                inodes.add(value)
        count = len(inodes)-1  # one known listener; worker uses pipes
        require(0 <= count <= 5, 'sampled accepted transport bound exceeded')
        self.samples.append({'time_ns':time.monotonic_ns(), 'accepted_transports':count})

    def journey(self):
        self.record['caps']['endpoint_identities_used_max'] = 12
        self.record['runtime_admission'] = {'dispatch_connections':4,'accepted_transports_including_auxiliary':5,'ptys':1,'managed_attempts':2,'capture':'off','model_requests':0,'fixture_deadline_s':30,'identity_limit':'15 ordinary +1 lifecycle unchanged','trust':'same effective UID + spoofable role declaration, not human authentication'}
        self.record['limitations'] = ['Synthetic protocol clients, not models/physical human/GUI', 'One finite run is not latency SLO/fairness/security/capacity qualification', 'Socket inode samples miss between-sample peaks; private unit tests independently cover reservation-before-accept', 'No preemption of Binding/Human/mutating handlers or rollback of prior effects; all-protected/busy/exhausted states refuse', 'Capture off; explicit synthetic test evidence is retained; resume secrets stay RAM-only']
        started = time.monotonic()
        setup = self.observer
        setup.close()
        self.observer = None
        agents = [self.connect() for _ in range(4)]
        a, b, c, d = agents
        self.wait(lambda:not next(e for e in self.work(b)['endpoints'] if e['id']==setup.endpoint)['connected'], 'setup endpoint detached')
        self.observer = b
        require('human_admission_v0' in self.work(b)['capabilities'],'admission capability absent')
        task = applied(b, {'op':'define_task','task':'admission','goal':'Synthetic local control admission','expected_artifacts':[]})
        revision = task['receipt']['result']['revision']
        applied(a, {'op':'create_session','name':'owned-control'})
        session = self.wait_snapshot(lambda s:bool(s['sessions']), 'actual owned shell')['sessions'][0]
        sid = session['id']
        self.wait_session(sid, self.at_prompt, 'owned shell ready')
        session = scoped(a, 'session', sid)['session']
        applied(a, {'op':'take_control','session':sid,'generation':session['generation'],'force':False})
        old = scoped(a, 'session', sid)['session']
        (self.workspace/'payload.py').write_bytes(Path(__file__).with_name('load_payload.py').read_bytes())
        (self.workspace/'phase').write_text('noisy')
        origins = {}
        for client, label in [(b,'A'),(c,'B')]:
            r = applied(client, {'op':'run','task':'admission','task_revision':revision,'argv':['/usr/bin/python3','-u','payload.py','managed-'+label]})
            aid = r['receipt']['result']['attempt']
            origins[aid] = {'endpoint':client.endpoint,'request_id':r['receipt']['id']}
        self.wait(lambda:(self.workspace/'ticks-B').exists() and (self.workspace/'ticks-B').stat().st_size>=12, 'bounded noisy process output')
        self.descendants();self.root_pidfd=os.pidfd_open(self.process.pid)
        descriptors={self.process.pid:self.root_pidfd,**self.owned_pidfds}
        require(len(descriptors)==5,'expected owned broker+worker+shell+two managed processes; helper exec replaces itself')
        def monitor():
            try:
                while not self.stop_monitor.is_set():
                    self.sockets();row=owned_resources(descriptors);self.resources.append(row)
                    require(row['rss_bytes']<=RSS_CAP,'sampled owned1GiBRSS guard')
                    self.stop_monitor.wait(.02)
            except BaseException as error:self.monitor_errors.append(type(error).__name__+': '+str(error))
        self.monitor=threading.Thread(target=monitor);self.monitor.start()
        before = set(e['id'] for e in self.work(b)['endpoints'])
        refused = self.raw(self.hello('agent'))
        require(not refused['ok'] and refused['error']=='control_admission_requires_human_or_lifecycle_hello', 'fifth Agent admitted')
        invalid = self.raw(self.hello(resume='synthetic-invalid'))
        require(not invalid['ok'] and invalid['error']=='resume_unknown_in_this_epoch', 'invalid resume admitted')
        require(set(e['id'] for e in self.work(b)['endpoints'])==before and all(e['connected'] for e in self.work(b)['endpoints'] if e['id'] in [x.endpoint for x in agents]), 'failed candidates changed identity/connectivity')
        # Many bounded read frames build actual unread output. Queue one canary
        # behind them, then prove it has no reserved/applied receipt before/after.
        a.socket.setsockopt(socket.SOL_SOCKET,socket.SO_RCVBUF,4096)
        queued = []
        frames=[]
        for _ in range(128):
            rid = a.next_id; a.next_id += 1; queued.append(rid)
            frames.append(json.dumps({'version':'namtab.mk1/1','id':rid,'operation':{'op':'observe'}}).encode()+b'\n')
        canary_id = a.next_id; a.next_id += 1
        canary = {'op':'input','session':sid,'generation':old['generation'],'revision':old['screen']['revision'],'mode_revision':old['screen']['mode_revision'],'policy':'observed','intent':{'kind':'text','text':'QUEUED_ADMISSION_CANARY'}}
        frames.append(json.dumps({'version':'namtab.mk1/1','id':canary_id,'operation':canary}).encode()+b'\n')
        batch=b''.join(frames);require(len(batch)<=16384,'bounded queued request batch');a.socket.sendall(batch)
        pending=array.array('i',[0])
        def unread_bytes():
            fcntl.ioctl(a.socket.fileno(),0x541B,pending,True);return pending[0]
        # sendall is not evidence that the peer has run. Wait only for an
        # observation; never resend the batch or its queued mutation.
        self.wait(unread_bytes,'actual unread response bytes',timeout=.5)
        require(not any(r['endpoint']==a.endpoint and r['id']==canary_id for r in self.journal()), 'fixture did not hold canary before admission')
        t=time.monotonic_ns();human=self.connect('human');elapsed=time.monotonic_ns()-t;self.observer=human
        self.record['fresh_control']={'elapsed_ns':elapsed,'retired_endpoint':a.endpoint,'unread_bytes':pending[0],'queued_read_requests':queued,'canary_request':canary_id,'meaning':'unread consumer, not inferred exact kernel blocking state'}
        require(elapsed<=2_000_000_000,'functional admission2s deadline')
        self.wait(lambda:not next(e for e in self.work(human)['endpoints'] if e['id']==a.endpoint)['connected'], 'retired endpoint disconnected exactly once')
        self.sockets()
        ticks=(self.workspace/'ticks-B').stat().st_size
        current=scoped(human,'session',sid)['session']
        applied(human,{'op':'take_control','session':sid,'generation':current['generation'],'force':True})
        for intent in [{'kind':'text','text':'printf x > human-proof'}, {'kind':'key','key':'Enter','ctrl':False,'alt':False,'shift':False}]:
            current=scoped(human,'session',sid)['session']
            applied(human,{'op':'input','session':sid,'generation':current['generation'],'revision':current['screen']['revision'],'mode_revision':current['screen']['mode_revision'],'policy':'human_live','intent':intent})
        self.wait(lambda:(self.workspace/'human-proof').exists(), 'actual fresh human command effect', timeout=2)
        require((self.workspace/'human-proof').read_bytes()==b'x','effect not exact')
        self.wait(lambda:(self.workspace/'ticks-B').stat().st_size>ticks, 'peer progresses during fresh takeover', timeout=2)
        require(not any(r['endpoint']==a.endpoint and r['id']==canary_id for r in self.journal()),'retired queued mutation dispatched')
        require('QUEUED_ADMISSION_CANARY' not in ''.join(scoped(human,'session',sid)['session']['screen']['lines']), 'retired bytes echoed')
        # Explicit reconnect uses the SAME identity and a new free transport.
        # Close D, not a user session; no queued intent is replayed.
        d.close()
        self.wait(lambda:not next(e for e in self.work(human)['endpoints'] if e['id']==d.endpoint)['connected'],'explicit free slot')
        resumed=self.connect(resume=a.resume,next_id=a.next_id)
        require(resumed.endpoint==a.endpoint and scoped(resumed,'session',sid)['session']['owner']==human.endpoint,'resume relabeled identity or stole writer')
        require(not any(r['endpoint']==a.endpoint and r['id']==canary_id for r in self.journal()),'resume replayed queued intent')
        stale_id=resumed.next_id;stale=resumed.call(canary);stale_fence(stale,a.endpoint,stale_id)
        # A saturation Hello plus a prefetched request in ONE send retains its
        # suffix and binds it once. Never log the Hello's resume capability.
        pipe=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);pipe.settimeout(2);pipe.connect(str(self.socket_path));self.raw_sockets.append(pipe)
        frames=[{'version':'namtab.mk1/1','id':1,'operation':self.hello()}, {'version':'namtab.mk1/1','id':2,'operation':{'op':'observe'}}]
        pipe.sendall(b''.join(json.dumps(f).encode()+b'\n' for f in frames))
        with pipe.makefile('rb') as reader:
            h=json.loads(reader.readline(1_048_577));suffix=json.loads(reader.readline(1_048_577))
        require(h['ok'] and suffix['ok'] and suffix['id']==2 and suffix['snapshot']['endpoint']==h['endpoint'],'prefetched suffix lost, duplicated or misbound')
        self.record['pipelined_suffix']={'hello_id':h['id'],'request_id':suffix['id'],'endpoint':h['endpoint'],'ok':True}
        # Two real concurrent arrivals; record BOTH outcomes including refusal.
        barrier=threading.Barrier(3);arrivals=[];arrival_lock=threading.Lock()
        def arrival():
            sock=None
            barrier.wait();t=time.monotonic_ns()
            try:
                sock=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);sock.settimeout(2);sock.connect(str(self.socket_path))
                sock.sendall(json.dumps({'version':'namtab.mk1/1','id':1,'operation':self.hello()}).encode()+b'\n')
                with sock.makefile('rb') as reader:line=reader.readline(1_048_577)
                require(line.endswith(b'\n') and len(line)<=1_048_576,'closed/oversize concurrent Hello')
                response=json.loads(line);arrival_binding(response,human.epoch)
                row={'ok':response['ok'],'protocol_ok':True,'endpoint':response.get('endpoint'),'error':response.get('error'),'elapsed_ns':time.monotonic_ns()-t}
                if response['ok']:
                    with arrival_lock:self.raw_sockets.append(sock)
                    sock=None
            except Exception as e:row={'ok':False,'protocol_ok':False,'error':type(e).__name__+': '+str(e),'elapsed_ns':time.monotonic_ns()-t}
            finally:
                if sock:sock.close()
            with arrival_lock:arrivals.append(row)
        threads=[threading.Thread(target=arrival) for _ in range(2)]
        for t in threads:t.start()
        barrier.wait()
        for t in threads:t.join(timeout=4);require(not t.is_alive(),'arrival thread exceeded bound')
        self.record['concurrent_arrivals']=arrivals
        require(len(arrivals)==2 and all(r['protocol_ok'] for r in arrivals) and any(r['ok'] for r in arrivals),'concurrent protocol failure or no progress: '+str(arrivals));self.sockets()
        # Surviving clients are discoverable from the one protected Human.
        live=self.work(human)['endpoints']
        while any(e['connected'] and e['role']=='agent' for e in live):
            self.connect('human');live=self.work(human)['endpoints']
        require(sum(e['connected'] for e in live)==4,'expected exactly four protected connections')
        no_victim=self.raw(self.hello())
        require(not no_victim['ok'] and no_victim['error']=='control_admission_no_eligible_transport','protected humans evicted')
        # Oversized and trickled pending handshakes do not pin the sole acceptor.
        s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);s.settimeout(1);s.connect(str(self.socket_path));s.sendall(b' '*1025)
        try:require(s.recv(1)==b'','oversize candidate remained open')
        except ConnectionResetError:pass
        finally:s.close()
        s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);s.settimeout(1);s.connect(str(self.socket_path));t=time.monotonic()
        for _ in range(10):
            try:s.sendall(b' ')
            except (BrokenPipeError,ConnectionResetError):break
            time.sleep(.04)
        try:require(s.recv(1)==b'','trickle candidate remained open')
        except ConnectionResetError:pass
        finally:s.close()
        require(time.monotonic()-t<1,'pending framing lease not finite')
        # A fourth slot can disappear during a pending Hello. Its replacement
        # must not evict any of the three protected survivors.
        candidate=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);candidate.settimeout(2);candidate.connect(str(self.socket_path));self.raw_sockets.append(candidate)
        frame=json.dumps({'version':'namtab.mk1/1','id':1,'operation':self.hello()}).encode()+b'\n'
        candidate.sendall(frame[:1]);time.sleep(.04)
        pipe.shutdown(socket.SHUT_RDWR);pipe.close()
        survivors=self.wait(lambda:[e['id'] for e in self.work(human)['endpoints'] if e['connected']] if sum(e['connected'] for e in self.work(human)['endpoints'])==3 else None,'three surviving protected transports',timeout=.15)
        candidate.sendall(frame[1:])
        with candidate.makefile('rb') as reader:response=json.loads(reader.readline(1_048_577))
        arrival_binding(response,human.epoch);require(response['ok'],'newly free slot was refused')
        require(set(e['id'] for e in self.work(human)['endpoints'] if e['connected'])==set(survivors)|{response['endpoint']},'free-slot promotion retired a survivor')
        self.record['free_slot_during_hello']={'survivors':survivors,'new_endpoint':response['endpoint'],'ok':True,'limit':'public ordering proof; deterministic private unit separately forces pending phase'}
        self.record['all_protected_refusal']=no_victim['error']
        self.record['socket_samples']=self.samples
        self.stop_monitor.set();self.monitor.join(timeout=3);require(not self.monitor.is_alive() and not self.monitor_errors and len(self.resources)>=2,'resource monitor incomplete/failed: '+str(self.monitor_errors))
        self.record['resources']=self.resources;self.record['monitor_errors']=self.monitor_errors
        (self.workspace/'finish').write_bytes(b'finish')
        self.wait_snapshot(lambda s:all(a['state']=='exited' for a in s['attempts']),'both attributed managed jobs drain')
        attempts=self.work(human)['attempts'];result_population(attempts,origins)
        for attempt in attempts:
            detail=scoped(human,'attempt',attempt['id'])['attempt']
            require(detail['origin']==origins[attempt['id']] and detail['task_revision']==revision and detail['exit_code']==0,'attempt origin/result changed')
            own='ONLY_A_' if detail['origin']['endpoint']==b.endpoint else 'ONLY_B_';other='ONLY_B_' if own=='ONLY_A_' else 'ONLY_A_'
            require(own in detail['stdout'] and other not in detail['stdout'],'stream attribution crossed')
        require(time.monotonic()-started<30,'finite journey30s deadline')
        self.record['elapsed_s']=time.monotonic()-started
        self.record['checks'].extend({'name':n,'verdict':'PASS'} for n in ['four_agent_slots_fresh_human_real_effect','invalid_agent_resume_no_eviction','retired_buffered_canary_never_dispatched','generation_fence_zero_bytes','explicit_same_identity_resume_without_replay','concurrent_arrivals_count_all_outcomes','four_protected_humans_refuse','oversize_and_absolute_trickle_cleanup','noisy_peer_progress_two_exact_attempt_origins'])

    def cleanup(self):
        self.stop_monitor.set()
        faults=[]
        try:
            if self.monitor:
                self.monitor.join(timeout=3)
                if self.monitor.is_alive():faults.append('monitor still running')
        finally:
            self.record.setdefault('resources',self.resources);self.record['monitor_errors']=self.monitor_errors
            # A failed client close must not short-circuit other handles or make
            # the superclass close the same failing client before process cleanup.
            self.observer=None
            try:
                for handle in [*self.raw_sockets,*self.clients]:
                    try:handle.close()
                    except Exception as error:faults.append(type(error).__name__+': '+str(error))
                self.raw_sockets=[];self.clients=[]
                if self.root_pidfd is not None:
                    try:os.close(self.root_pidfd)
                    except OSError as error:faults.append(type(error).__name__+': '+str(error))
                    self.root_pidfd=None
            finally:
                self.record['owned_cleanup_faults']=faults
                super().cleanup()
        require(not faults,'owned cleanup had failures: '+str(faults))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--terminfo',type=Path,default=Path('.'));p.add_argument('--artifacts',type=Path,required=True);p.set_defaults(native=False)
    raise SystemExit(Admission(p.parse_args()).run())
