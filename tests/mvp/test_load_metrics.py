"""Damaged evidence and finalizer checks; no process/model/listener is started."""
import contextlib
import io
import copy
import json
from pathlib import Path
import tempfile
import unittest
from load_metrics import assess,assess_workload,PHASES,RSS_CAP
from load_evidence import execute

def good():
    expected=[dict(pid=i,starttime=str(i*10)) for i in range(1,8)]
    windows=[dict(repetition=r,phase=p,start_ns=1_100_000_000+(r*3+i)*1_000_000_000,end_ns=1_850_000_000+(r*3+i)*1_000_000_000) for r in range(3) for i,p in enumerate(PHASES)]
    samples=[dict(repetition=w['repetition'],phase=w['phase'],cycle=c,start_ns=w['start_ns']+c*100_000_000,ok=True,stale_bytes=0,probe_exact=True,peer_progress_a=True,peer_progress_b=True,takeover_ns=1_000_000,text_ns=1_000_000,enter_ns=1_000_000,probe_ns=2_000_000) for w in windows for c in range(6)]
    resources=[dict(time_ns=1_000_000_000+i*50_000_000,rss_bytes=10_000_000,processes=7,identities=copy.deepcopy(expected)) for i in range(181)]
    reads=[dict(repetition=w['repetition'],phase=w['phase'],scope=k,start_ns=w['start_ns']+50_000_000+i*200_000_000,ok=True,elapsed_ns=1_000_000) for w in windows for i,k in enumerate(['work_export','session','attempt'])]
    return samples,resources,reads,expected,windows
class Results(unittest.TestCase):
    def test_good_and_unearned_100ms_candidate(self):
        s,r,q,e,w=good();self.assertEqual(assess(s,r,q,e,w)['control_candidate'],'PASS')
        for x in s:x['takeover_ns']=101_000_000
        result=assess(s,r,q,e,w);self.assertEqual(result['functional'],'PASS');self.assertEqual(result['control_candidate'],'FAIL')
    def test_missing_duplicate_or_failed_intervention_cannot_pass(self):
        for mode in ['drop','duplicate','failure','stale','effect','starved_a','starved_b','deadline','wrong_window']:
            s,r,q,e,w=good()
            if mode=='drop':s.pop()
            if mode=='duplicate':s[-1]=copy.deepcopy(s[0])
            if mode=='failure':s[0]['ok']=False
            if mode=='stale':s[0]['stale_bytes']=1
            if mode=='effect':s[0]['probe_exact']=False
            if mode=='starved_a':s[0]['peer_progress_a']=False
            if mode=='starved_b':s[0]['peer_progress_b']=False
            if mode=='deadline':s[0]['takeover_ns']=2_000_000_001
            if mode=='wrong_window':s[0]['start_ns']=1
            with self.assertRaises(AssertionError,msg=mode):assess(s,r,q,e,w)
    def test_missing_starved_failed_or_slow_reader_not_dropped(self):
        for mode in ['empty','missing','failed','timeout','old_time']:
            s,r,q,e,w=good()
            if mode=='empty':q=[]
            if mode=='missing':q.pop()
            if mode=='failed':q[0]['ok']=False
            if mode=='timeout':q[0]['elapsed_ns']=2_000_000_001
            if mode=='old_time':q[0]['start_ns']=1
            with self.assertRaises(AssertionError,msg=mode):assess(s,r,q,e,w)
    def test_resource_overflow_old_missing_reversed_census_cannot_pass(self):
        for mode in ['empty','one_old','root_missing','wrong_identity','old_window','reversed','gap','missing_stage','over_cap']:
            s,r,q,e,w=good()
            if mode=='empty':r=[]
            if mode=='one_old':r=[dict(time_ns=1,rss_bytes=1,processes=1,identities=e[:1])]
            if mode=='root_missing':r[0]['identities']=e[1:];r[0]['processes']=6
            if mode=='wrong_identity':r[0]['identities'][0]['starttime']='wrong'
            if mode=='old_window':w[-1]['start_ns']+=20_000_000_000;w[-1]['end_ns']+=20_000_000_000
            if mode=='reversed':r.reverse()
            if mode=='gap':r=r[::10]
            if mode=='missing_stage':r=[x for x in r if not w[3]['start_ns']<=x['time_ns']<=w[3]['end_ns']]
            if mode=='over_cap':r[0]['rss_bytes']=RSS_CAP+1
            with self.assertRaises(AssertionError,msg=mode):assess(s,r,q,e,w)
    def test_raw_sample_percentiles_not_percentiles_of_percentiles(self):
        s,r,q,e,w=good()
        for i,x in enumerate(s):x['takeover_ns']=(i+1)*1_000_000
        result=assess(s,r,q,e,w);self.assertEqual(result['takeover_all']['n'],54);self.assertEqual(result['takeover_all']['p95_ns'],51_000_000);self.assertEqual(result['takeover_all']['max_ns'],54_000_000)
    def test_failed_missing_positive_stream_or_starved_job_cannot_pass(self):
        origins={'a':{'endpoint':'ea','request_id':3},'b':{'endpoint':'eb','request_id':3}};canaries={'a':'ONLY_A_','b':'ONLY_B_'}
        jobs=[dict(id=k,origin=v,task_revision='r',state='exited',exit_code=0) for k,v in origins.items()];details={'a':{'stdout':'ONLY_A_'},'b':{'stdout':'ONLY_B_'}}
        phases=[dict(repetition=r,phase=p,a_start=1,a_end=2,b_start=1,b_end=2) for r in range(3) for p in PHASES]
        assess_workload(jobs,details,origins,'r',phases,canaries)
        for mode in ['failed_A','empty_A','crossed','starved_A','missing_phase']:
            j,d,p=copy.deepcopy((jobs,details,phases))
            if mode=='failed_A':j[0]['exit_code']=1
            if mode=='empty_A':d['a']['stdout']=''
            if mode=='crossed':d['a']['stdout']+='ONLY_B_'
            if mode=='starved_A':p[0]['a_end']=p[0]['a_start']
            if mode=='missing_phase':p.pop()
            with self.assertRaises(AssertionError,msg=mode):assess_workload(j,d,origins,'r',p,canaries)

class Fake:
    def __init__(self,root,cleanup_error=False):
        self.root=root;self.artifacts=root/'out';self.artifacts.mkdir();self.runtime=root/'runtime';self.runtime.mkdir();self.socket_path=self.runtime/'s';self.record={};self.fail_cleanup=cleanup_error;f=root/'fixture.py';f.write_text('# synthetic test\n');self.fixture_files=[f]
    def start(self):pass
    def journey(self):self.record['qualification']='PASS_NARROW_RECORDED_HOST_ONLY'
    def cleanup(self):
        if self.fail_cleanup:raise AssertionError('owned cleanup failed')

class Sealing(unittest.TestCase):
    def identity(self,_):return dict(head='h',tree='t',dirty=False,index_sha256='i',diff_sha256='d')
    def test_good_seal_has_tree_sources_and_evidence(self):
        with tempfile.TemporaryDirectory() as d:
            f=Fake(Path(d));self.assertEqual(execute(f,self.identity),0);r=json.loads((f.artifacts/'checks.json').read_text());self.assertEqual(r['source_identity']['verdict'],'PASS');self.assertEqual(r['source_identity']['after']['tree'],'t');self.assertIn('sources/fixture.py',r['evidence'])
    def test_failure_stdout_is_bounded_structured_and_not_a_screen_dump(self):
        with tempfile.TemporaryDirectory() as d:
            f=Fake(Path(d));f.record['private_screen']='NEVER_DUMP_THIS'
            def broken():raise AssertionError('synthetic\n\x1b[31m'+('x'*2000))
            f.journey=broken;output=io.StringIO()
            with contextlib.redirect_stdout(output):self.assertEqual(execute(f,self.identity),1)
            lines=output.getvalue().splitlines();self.assertEqual(len(lines),1);r=json.loads(lines[0])
            self.assertNotIn('NEVER_DUMP_THIS',lines[0]);self.assertEqual(r['verdict'],'FAIL');self.assertEqual(set(r['errors']),{'execution_error'})
            self.assertEqual(len(r['errors']['execution_error']['message']),512)
            self.assertEqual(r['errors']['execution_error']['type'],'AssertionError')
    def test_cleanup_failure_never_preserves_pass(self):
        with tempfile.TemporaryDirectory() as d:
            f=Fake(Path(d),True);self.assertEqual(execute(f,self.identity),1);r=json.loads((f.artifacts/'checks.json').read_text());self.assertEqual(r['verdict'],'FAIL');self.assertTrue(r['qualification'].startswith('FAIL'));self.assertIn('cleanup_error',r)
    def test_source_change_or_dirty_input_never_qualifies(self):
        for mode in ['dirty','changed']:
            with tempfile.TemporaryDirectory() as d:
                f=Fake(Path(d));states=[self.identity(None),self.identity(None)]
                if mode=='dirty':states[0]['dirty']=True
                else:states[1]['tree']='other'
                self.assertEqual(execute(f,lambda _:states.pop(0)),1);self.assertEqual(json.loads((f.artifacts/'checks.json').read_text())['verdict'],'FAIL')
    def test_fixture_dependency_change_never_qualifies(self):
        with tempfile.TemporaryDirectory() as d:
            f=Fake(Path(d));f.cleanup=lambda:f.fixture_files[0].write_text('changed')
            self.assertEqual(execute(f,self.identity),1);self.assertIn('seal_error',json.loads((f.artifacts/'checks.json').read_text()))
class Transaction(unittest.TestCase):
    def test_stale_receipt_uses_captured_request_not_mutable_last(self):
        from types import SimpleNamespace
        from engine_load import EngineLoad
        for mode in ['valid','wrong_id','unknown']:
            receipt=dict(endpoint='a',id=5,operation='input',state='rejected',delivered_bytes=0)
            if mode=='wrong_id':receipt['id']=6
            if mode=='unknown':receipt['state']='unknown'
            peer=SimpleNamespace(endpoint='a',next_id=5,last={'request_id':5},call=lambda _:dict(ok=False,id=5,receipt=receipt))
            class PollOnUnlock:
                def __enter__(self):return self
                def __exit__(self,*_):peer.last={'request_id':6};peer.next_id=7
            fixture=SimpleNamespace(a=peer,sid='s',agent_lock=PollOnUnlock());before=dict(generation=1,screen=dict(revision=2,mode_revision=3))
            if mode=='valid':self.assertEqual(EngineLoad.stale_input(fixture,before),0);self.assertEqual(peer.last['request_id'],6)
            else:
                with self.assertRaises(AssertionError):EngineLoad.stale_input(fixture,before)
    def test_nested_checks_file_is_included_in_evidence_seal(self):
        with tempfile.TemporaryDirectory() as d:
            f=Fake(Path(d))
            def create_nested():
                (f.artifacts/'workspace').mkdir();(f.artifacts/'workspace/checks.json').write_text('{"synthetic":true}')
            f.cleanup=create_nested;self.assertEqual(execute(f,Sealing().identity),0)
            self.assertIn('workspace/checks.json',json.loads((f.artifacts/'checks.json').read_text())['evidence'])
if __name__=='__main__':unittest.main()
