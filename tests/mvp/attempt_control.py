#!/usr/bin/env python3
"""Real isolated ENG1 direct-process proof; two SCRIPTED clients, never a model."""
import argparse
import json
from pathlib import Path
import subprocess
import select
import time
from adapter import Adapter
from e2e import Connection, require, sha256

class AttemptControl(Adapter):
    def journey(self):
        epoch=self.snapshot()['epoch'];a=Connection(self.socket_path,'scripted-owner-A','agent');b=Connection(self.socket_path,'scripted-peer-B','agent')
        self.record['profile']='namtab.attempt-control/0'
        self.record['harness_sha256']=sha256(Path(__file__))
        (self.artifacts/'attempt_control.py').write_bytes(Path(__file__).read_bytes())
        try:
            require('attempt_cancel_v0' in a.observe()['capabilities'],'missing cancellation capability; no fallback')
            task=a.call({'op':'define_task','task':'shared-control','goal':'Synthetic independent attempts; cancelA while peer continues','expected_artifacts':[]})
            revision=task['receipt']['result']['revision']
            argv_a=['/usr/bin/python3','-u','-c',"import time,os;print('A_READY');print('A_PID='+str(os.getpid()));print('A_ERR',file=__import__('sys').stderr);time.sleep(30)"]
            argv_b=['/usr/bin/python3','-u','-c',"import pathlib,time,sys,os;print('B_READY');print('B_PID='+str(os.getpid()));print('B'*100000);p=pathlib.Path('peer-ticks');\nwhile not pathlib.Path('finish-peer').exists():\n with p.open('ab') as f:f.write(b'x')\n print('B_TICK',file=sys.stderr);time.sleep(.02)"]
            ra=a.call({'op':'run','task':'shared-control','task_revision':revision,'argv':argv_a});rb=b.call({'op':'run','task':'shared-control','task_revision':revision,'argv':argv_b})
            require(ra['ok'] and rb['ok'],'attempt launch refused')
            aid=ra['receipt']['result']['attempt'];bid=rb['receipt']['result']['attempt']
            s=self.wait_snapshot(lambda s:len(s['attempts'])==2 and all(x['stdout'] for x in s['attempts']) and (self.workspace/'peer-ticks').exists(),'two actual process streams')
            one=next(x for x in s['attempts'] if x['id']==aid);two=next(x for x in s['attempts'] if x['id']==bid)
            require(one['origin']['endpoint']==a.endpoint and two['origin']['endpoint']==b.endpoint and one['task_revision']==two['task_revision']==revision,'cross-attribution or task revision')
            # Output is not authority: first retain kernel-derived pidfds only
            # for descendants of our owned broker, then correlate the canaries.
            self.descendants()
            direct={int(pid) for pid in (Path('/proc')/str(self.process.pid)/'task'/str(self.process.pid)/'children').read_text().split()}
            apid=int(next(line.removeprefix('A_PID=') for line in one['stdout'].splitlines() if line.startswith('A_PID=')))
            bpid=int(next(line.removeprefix('B_PID=') for line in two['stdout'].splitlines() if line.startswith('B_PID=')))
            require(apid!=bpid and {apid,bpid}<=direct and all(pid in self.owned_pidfds for pid in [apid,bpid]),'markers not matched to verified owned direct children')
            afd=self.owned_pidfds[apid];bfd=self.owned_pidfds[bpid]
            require(not select.select([afd,bfd],[],[],0)[0],'fixture child exited before cancellation')
            target={'op':'cancel_attempt','epoch':epoch,'attempt':aid,'origin_endpoint':one['origin']['endpoint'],'origin_request_id':one['origin']['request_id']}
            rejected=[]
            for client,op in [(b,target),(a,{**target,'epoch':'0'*64}),(a,{**target,'origin_request_id':one['origin']['request_id']+99}),(a,{**target,'attempt':'missing'})]:
                r=client.call(op);require(not r['ok'] and r['receipt']['state']=='rejected','bad target acquired cancellation');rejected.append(r['receipt'])
            require(all(x['cancellation'] is None for x in self.snapshot()['attempts']),'refusal changed cancellation')
            before=(self.workspace/'peer-ticks').stat().st_size
            rid=a.next_id;start=time.perf_counter_ns();result=a.call(target);elapsed=time.perf_counter_ns()-start
            require(result['ok'] and result['receipt']['result']=={'kind':'cancellation_requested','attempt':aid,'status':'requested'},'request receipt mislabelled')
            repeat=a.call(target,request_id=rid);require(repeat['receipt']==result['receipt'],'duplicate request changed original receipt')
            conflict=a.call({**target,'attempt':bid,'origin_endpoint':b.endpoint,'origin_request_id':two['origin']['request_id']},request_id=rid);require(not conflict['ok'],'ID retarget admitted')
            s=self.wait_snapshot(lambda s:any(x['id']==aid and x['state']=='exited' for x in s['attempts']),'independently observed A exit/drain')
            ended=next(x for x in s['attempts'] if x['id']==aid);peer=next(x for x in s['attempts'] if x['id']==bid)
            self.wait(lambda:select.select([afd],[],[],0)[0],'independent A pidfd exit before cleanup')
            require(not select.select([bfd],[],[],0)[0],'B exited before natural finish')
            require(ended['exit_signal']==9 and ended['cancellation']=={'requested_by':a.endpoint,'request_id':rid,'status':'exit_observed'},'request/exit provenance differs')
            require(peer['state']=='running' and peer['cancellation'] is None,'peer was cancelled')
            after_exit=(self.workspace/'peer-ticks').stat().st_size
            self.wait(lambda:(self.workspace/'peer-ticks').stat().st_size>after_exit+2,'peer continued AFTER observed A exit')
            require('B_READY' not in ended['stdout'] and 'A_READY' not in peer['stdout'] and 'A_ERR' in ended['stderr'] and 'B_TICK' not in ended['stderr'] and 'B_TICK' in peer['stderr'] and 'A_ERR' not in peer['stderr'],'stdout/stderr origins crossed')
            peer=self.wait_snapshot(lambda s:next(x for x in s['attempts'] if x['id']==bid)['truncated'],'bounded flood truncation')['attempts']
            require(all(len(x['stdout'].encode())<=65536 and len(x['stderr'].encode())<=65536 for x in peer),'projection exceeded bound')
            again=self.observer.call(target);require(again['receipt']['result']['status']=='already_exited','human exact exited target not recognized')
            (self.workspace/'finish-peer').write_bytes(b'finish')
            final=self.wait_snapshot(lambda s:all(x['state']=='exited' for x in s['attempts']),'peer natural completion')
            require(next(x for x in final['attempts'] if x['id']==bid)['exit_code']==0,'peer failed')
            self.wait(lambda:select.select([bfd],[],[],0)[0],'independent B natural exit')
            self.record['checks']=[{'name':n,'verdict':'PASS'} for n in ['exact_run_task_origin_streams','stale_unknown_unauthorized_zero_effect','request_dedupe_conflict_no_retarget','requested_receipt_separate_observed_exit','peer_continues_and_naturally_exits','bounded_flood_no_cross_attribution']]
            self.record['independent_os_exit']={'A_pidfd_ready_before_cleanup':True,'B_alive_after_A_exit':True,'B_pidfd_ready_after_finish':True,'verified_direct_children':[apid,bpid],'source':'Kernel-derived own-broker children and retained pidfds; emitted markers correlate only, never signal authority','limit':'pidfd readiness proves exit, not signal/cause/status; exit_signal9 and exit_code0 are broker observations'}
            self.record['outcomes']={'attempts':final['attempts'],'cancel_receipt':result['receipt'],'negative_receipts':rejected,'peer_ticks_before':before,'peer_ticks_at_observed_A_exit':after_exit,'peer_ticks_after':(self.workspace/'peer-ticks').stat().st_size,'cancel_rpc_sample_ns':elapsed,'timing_limit':'One local synchronous RPC sample, not latency distribution/SLO/capacity or model inference'}
            self.record['limitations']=['Two scripted public clients and one observer, fresh namespace; no model integration','Only owned direct child signal/wait observed; no descendant/all-effect termination guarantee','Snapshot streams are bounded projections, not durable capture; captureoff; no fullH0/production qualification']
        finally:a.close();b.close()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--terminfo',type=Path,default=Path('.'));p.add_argument('--artifacts',type=Path,required=True)
    args=p.parse_args();args.native=False;AttemptControl(args).run()
