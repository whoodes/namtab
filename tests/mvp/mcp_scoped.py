#!/usr/bin/env python3
"""Synthetic real MCP/engine scoped-read proof; no model, GUI or user session.
The owned serial proxy records actual broker frames, redacting resume capabilities.
"""
import argparse
import copy
import json
import platform
from pathlib import Path
import socket
import threading
import time
from adapter import Adapter
from mcp_client import MCP
from scoped_observation import MeasuredConnection, quantiles
from e2e import Connection, require, sha256

class Proxy:
    def __init__(self,path,target):
        self.path=path;self.target=target;self.rows=[];self.error=None
        self.listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);self.listener.bind(str(path));self.listener.listen(1);self.listener.settimeout(5)
        self.thread=threading.Thread(target=self.run);self.thread.start()
    def run(self):
        try:
            with self.listener.accept()[0] as client, socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as upstream:
                upstream.settimeout(4);upstream.connect(str(self.target));client.settimeout(15)
                with client.makefile('rb') as source,upstream.makefile('rb') as broker:
                    while True:
                        request=source.readline(1_048_577)
                        if not request:break
                        require(len(request)<=1_048_576 and request.endswith(b'\n'),'proxy request framing')
                        q=json.loads(request);start=time.perf_counter_ns();upstream.sendall(request)
                        response=broker.readline(1_048_577)
                        require(response and len(response)<=1_048_576 and response.endswith(b'\n'),'proxy response framing')
                        r=json.loads(response);safe=copy.deepcopy(r)
                        if safe.get('resume') is not None:safe['resume']='<redacted>'
                        require(not q['operation'].get('resume'),'fixture may not log a resume capability')
                        self.rows.append({'request':q,'response':safe,'request_wire_bytes':len(request),'response_wire_bytes':len(response),'elapsed_ns':time.perf_counter_ns()-start})
                        client.sendall(response)
        except BaseException as error:self.error=type(error).__name__+': '+str(error)
        finally:self.listener.close()
    def close(self):
        self.thread.join(timeout=5);require(not self.thread.is_alive(),'owned MCP proxy did not detach');require(self.error is None,'proxy failure: '+str(self.error))

def validate_read_trace(rows):
    require(rows and rows[0]['request']['operation']['op']=='hello','missing initial hello')
    require(sum(x['request']['operation']['op']=='observe' for x in rows)==1,'expected exactly one full discovery, no fallback')
    require(rows[1]['request']['operation']['op']=='observe','discovery must precede scoped dispatch')
    endpoint=rows[0]['response']['endpoint'];epoch=rows[0]['response']['epoch']
    for row in rows:
        q=row['request'];r=row['response'];require(q['id']==r['id'],'trace correlation')
        if q['operation']['op']!='observe_scoped':continue
        require(q['operation']['epoch']==r['epoch']==epoch,'trace epoch')
        require(all(r[k] is None for k in ['snapshot','receipt','resume']),'mixed scoped response')
        require(r['ok']==(r.get('observation') is not None),'trace success/view consistency')
        if not r['ok']:require(isinstance(r.get('error'),str),'read refusal reason required');continue
        require(r.get('error') is None,'successful read carried error')
        o=r['observation'];scope=q['operation']['scope'];view=o['view']
        require(o['version']=='namtab.mk1/1' and o['epoch']==q['operation']['epoch'] and o['endpoint']==endpoint and (r.get('endpoint') is None or r['endpoint']==endpoint) and o['profile']=='namtab.scoped-observation/0','trace projection binding')
        require(view['kind']==scope['kind'],'trace scope')
        if scope['kind'] in ['attempt','session']:require(view[scope['kind']]['id']==scope[scope['kind']],'cross-target projection')
        if scope['kind']=='work_export':
            require(all(k not in view for k in ['endpoints','receipts','resume','screen','stdout','stderr']),'work metadata export boundary')
            require(all('screen' not in s for s in view['sessions']) and all('stdout' not in a and 'stderr' not in a for a in view['attempts']),'nested work body leakage')
    return True

class ScopedMCP(Adapter):
    def __init__(self,args):
        super().__init__(args);self.peer=None;self.proxies=[]
    def open_bridge(self):
        proxy=Proxy(self.runtime/('p'+str(len(self.proxies))),self.socket_path);self.proxies.append(proxy)
        self.record.setdefault('proxy_traces',[]).append(proxy.rows)
        self.bridge=MCP(self.bin_dir/'namtab-mcp',proxy.path,self.observer.epoch,self.artifacts/('mcp-'+str(len(self.proxies))+'.log'),Path.cwd())
        self.record.setdefault('mcp_transcripts',[]).append(self.bridge.transcript)
        return self.bridge,proxy
    def journey(self):
        sources=[Path(__file__),Path(__file__).with_name('adapter.py'),Path(__file__).with_name('mcp_client.py'),Path(__file__).with_name('scoped_observation.py'),Path(__file__).with_name('native.py'),Path(__file__).parents[1]/'mk1/e2e.py']
        (self.artifacts/'sources').mkdir();self.record['fixture_source_hashes']={str(p.relative_to(Path.cwd())):sha256(p) for p in sources}
        for p in sources:(self.artifacts/'sources'/p.name).write_bytes(p.read_bytes())
        before_hashes={n:sha256(self.bin_dir/n) for n in self.record['binary_hashes']};self.record['binary_hashes_before']=before_hashes
        m,proxy=self.open_bridge();self.peer=MeasuredConnection(self.socket_path,'MCP-scoped-peer-B','agent');peer=self.peer
        task=m.value('namtab_task',{'task':'shared','goal':'Synthetic attributed MCP scoped-read fixture','expected_artifacts':['artifact']})['receipt']['result'];revision=task['revision']
        m.value('namtab_create_terminal',{'name':'MCP-A'});require(peer.call({'op':'create_session','name':'peer-B'})['ok'],'peer terminal create')
        s=self.snapshot();sid=next(x['id'] for x in s['sessions'] if x['name']=='MCP-A');other=next(x['id'] for x in s['sessions'] if x['name']=='peer-B')
        for terminal in [sid,other]:self.wait_session(terminal,self.at_prompt,'owned quiet Bash prompt')
        a=m.value('namtab_run',{'task':'shared','task_revision':revision,'argv':['/bin/sh','-c','printf ONLY_A; printf ERR_A >&2']})['receipt']['result']['attempt']
        # B emits a bounded control-heavy flood, then heartbeats until explicit fixture completion.
        code="import pathlib,sys,time;sys.stdout.buffer.write(b'ONLY_B'+b'\\x00'*20000);sys.stdout.flush();p=pathlib.Path('ticks');\nwhile not pathlib.Path('finish').exists():\n with p.open('ab') as f:f.write(b'x')\n print('B_TICK',file=sys.stderr,flush=True);time.sleep(.02)"
        r=peer.call({'op':'run','task':'shared','task_revision':revision,'argv':['/usr/bin/python3','-u','-c',code]});require(r['ok'],'peer managed launch');b=r['receipt']['result']['attempt']
        self.wait_snapshot(lambda s:next(x for x in s['attempts'] if x['id']==a)['state']=='exited' and next(x for x in s['attempts'] if x['id']==b)['truncated'],'A exit and B sanitized expansion loss')
        path=self.workspace/'artifact';path.write_bytes(b'F'*1_048_576);m.value('namtab_artifact',{'task':'shared','path':'artifact'})
        work=m.value('namtab_observe',{});artifact=work['artifacts'][0]
        require(next(x for x in work['attempts'] if x['id']==b)['source_truncated'],'MCP work underreported sanitized loss')
        require(self.observer.call({'op':'validate_artifact','artifact':artifact['id'],'revision':artifact['revision']})['ok'],'human artifact review')
        path.write_bytes(b'changed')
        # No legacy Observe between edit and this MCP read: scope itself must refresh.
        changed=m.value('namtab_observe',{})['artifacts'][0];require(changed['stale'] and changed['revision']!=artifact['revision'] and changed['validated_revision']==artifact['revision'],'scope lost fresh artifact staleness')
        path.unlink();missing=m.value('namtab_observe',{})['artifacts'][0];require(missing['revision']=='unavailable' and missing['stale'],'missing artifact not explicitly unavailable')
        path.write_bytes(b'F'*1_048_576)
        detail=m.value('namtab_observe',{'attempt':a});require(detail['stdout_preview']=='ONLY_A' and detail['stderr_preview']=='ERR_A' and detail['origin']['endpoint']!=peer.endpoint and detail['task_revision']==revision,'MCP selected attempt attribution')
        read=m.value('namtab_observe',{'session':sid});m.value('namtab_claim_terminal',{'session':sid,'generation':read['generation']});owned=m.value('namtab_observe',{'session':sid})
        before_tick=(self.workspace/'ticks').stat().st_size
        require(self.observer.call({'op':'take_control','session':sid,'generation':owned['generation'],'force':True})['ok'],'human takeover')
        stale=m.tool('namtab_input',{'session':sid,'generation':owned['generation'],'revision':owned['screen']['revision'],'mode_revision':owned['screen']['mode_revision'],'intent':{'kind':'text','text':'STALE_MCP_SCOPE'}},ok=False)
        require(stale['result']['isError'] and stale['result']['structuredContent']['observation_or_response']['receipt']['delivered_bytes']==0,'stale input effects')
        self.wait(lambda:(self.workspace/'ticks').stat().st_size>before_tick+2,'peer continues after human takeover')
        bad=m.tool('namtab_observe',{'attempt':'missing'},ok=False);require(bad['result']['isError'] and bad['result']['structuredContent']['error']=='unknown_attempt','read refusal semantics')
        # 26 iterations ×3 reads +setup stays below existing128accepted MCP IDs.
        samples={k:[] for k in ['work_export','session','attempt','legacy']};journal=Path(str(self.socket_path)+'.journal');size=journal.stat().st_size
        for i in range(26):
            order=['legacy','work_export','session','attempt'] if i%2==0 else ['attempt','session','work_export','legacy']
            for kind in order:
                if kind=='legacy':peer.observe();sample=dict(peer.last_sample)
                else:
                    args={} if kind=='work_export' else {kind:sid if kind=='session' else a};n=len(proxy.rows);start=time.perf_counter_ns();result=m.tool('namtab_observe',args);elapsed=time.perf_counter_ns()-start
                    require(len(proxy.rows)==n+1,'MCP read performed extra/full discovery');row=proxy.rows[-1];require(row['request']['operation']['scope']['kind']==kind,'wrong dispatched scope')
                    sample={'elapsed_ns':elapsed,'response_wire_bytes':row['response_wire_bytes'],'broker_elapsed_ns':row['elapsed_ns'],'mcp_response_reserialized_bytes':len(json.dumps(result,separators=(',',':')).encode())}
                if i>=2:samples[kind].append(dict(sample,iteration=i-2,order=order))
        require(journal.stat().st_size==size,'scoped reads appended mutation journal')
        require(max(v['response_wire_bytes'] for v in samples['work_export'])<min(v['response_wire_bytes'] for v in samples['legacy']),'work export did not omit unrelated bodies')
        self.record['measurement']={'samples':samples,'summary':{k:quantiles(v) for k,v in samples.items()},'workload':'Shared Ubuntu VM, Cargo debug;2quiet PTYs,2attempts(A exited,B control-heavy capped stream+20ms heartbeat),one fresh synthetic1MiB registered file,3live public connections;2warmup+24retained iterations/scope AB/BA order','scope':'MCP elapsed includes stdio JSONencode/send/read/decode+owned proxy/broker; legacy is direct public JSON RPC, so timings are not equal-path speed comparison. Response_wire_bytes are actual broker line bytes; reserialized MCP size is labeled, not wire bytes. Work export still rehashes artifacts and scans stream loss; targeted reads omit that work. No model/GUI/human paint/serverCPU/SLO/fleet/fairness claim.','host':{'kernel':platform.release(),'machine':platform.machine(),'logical_cpus':__import__('os').cpu_count()}}
        require(m.next<=128,'fixture exceeded existing MCP namespace')
        self.record['mcp_request_ids_used']=m.next-1
        # Close one real bridge, explicitly start another, rediscover once; no resume/replay.
        first_transcript=m.transcript;m.close();self.bridge=None;proxy.close();validate_read_trace(proxy.rows)
        self.wait_snapshot(lambda s:sum(e['connected'] for e in s['endpoints'])==2,'MCP detach')
        m,second=self.open_bridge();new=m.value('namtab_observe',{});require(len(new['attempts'])==2,'new bridge lost work');m.close();self.bridge=None;second.close();validate_read_trace(second.rows)
        require(proxy.rows[0]['response']['endpoint']!=second.rows[0]['response']['endpoint'],'fresh bridge falsely resumed old endpoint')
        require(not any(x['request']['operation']['op'] in ['run','input','artifact','define_task'] for x in second.rows),'fresh bridge replayed mutation')
        (self.workspace/'finish').write_bytes(b'finish');self.wait_snapshot(lambda s:all(x['state']=='exited' for x in s['attempts']),'peer direct process exit')
        self.record['proxy_traces']=[p.rows for p in self.proxies];self.record['mcp_transcript']=first_transcript
        self.record['binary_hashes_after']={n:sha256(self.bin_dir/n) for n in before_hashes};require(self.record['binary_hashes_after']==before_hashes,'executed bytes changed')
        self.record['checks'].extend({'name':n,'verdict':'PASS'} for n in ['actual_mcp_stdio_discovery_once_scoped_only_after_bootstrap','exact_selected_attempt_session_attribution_export_limits','fresh_edit_missing_validated_artifact_staleness','legacy_sanitized_loss_flag_preserved','human_takeover_stale_zero_bytes_peer_continues','missing_target_explicit_read_refusal','bounded_workload_request_response_samples_no_mutation_journal','fresh_bridge_new_identity_renegotiates_no_mutation_replay','executed_binary_provenance_unchanged'])
        self.record['limitations']=['Synthetic scripted MCP protocol clients, no model or GUI execution','Work export still performs bounded file rehash/sanitized loss scan; plain Work remains cheaper','Shared VM/debug proxy timing,24samples per scope; no performance SLO/capacity/fairness qualification','Capture-off does not make explicitly captured synthetic fixture output private']
    def cleanup(self):
        try:
            if self.peer:self.peer.close();self.peer=None
            if self.bridge:self.bridge.close();self.bridge=None
            for p in self.proxies:p.close()
        finally:super().cleanup()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--terminfo',type=Path,default=Path('.'));p.add_argument('--artifacts',type=Path,required=True);p.set_defaults(native=False)
    ScopedMCP(p.parse_args()).run()
