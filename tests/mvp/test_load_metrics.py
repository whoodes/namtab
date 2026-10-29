"""Independent damaged-result checks; starts no process and grants no authority."""
import copy
import unittest
from load_metrics import assess,PHASES,RSS_CAP

def good():
    samples=[dict(repetition=r,phase=p,cycle=c,ok=True,stale_bytes=0,probe_exact=True,peer_progress=True,takeover_ns=1_000_000,text_ns=1_000_000,enter_ns=1_000_000,probe_ns=2_000_000) for r in range(3) for p in PHASES for c in range(6)]
    resources=[dict(time_ns=1,rss_bytes=10_000_000,processes=7),dict(time_ns=50_000_001,rss_bytes=10_500_000,processes=7)]
    reads=[dict(repetition=r,phase=p,scope=k,ok=True,elapsed_ns=1_000_000) for r in range(3) for p in PHASES for k in ['work_export','session','attempt']]
    return samples,resources,reads
class Results(unittest.TestCase):
    def test_good_and_unearned_100ms_candidate(self):
        s,r,q=good();self.assertEqual(assess(s,r,q)['control_candidate'],'PASS')
        for x in s:x['takeover_ns']=101_000_000
        result=assess(s,r,q);self.assertEqual(result['functional'],'PASS');self.assertEqual(result['control_candidate'],'FAIL')
    def test_missing_duplicate_or_failed_intervention_cannot_pass(self):
        for mode in ['drop','duplicate','failure','stale','effect','starved','deadline']:
            s,r,q=good()
            if mode=='drop':s.pop()
            if mode=='duplicate':s[-1]=copy.deepcopy(s[0])
            if mode=='failure':s[0]['ok']=False
            if mode=='stale':s[0]['stale_bytes']=1
            if mode=='effect':s[0]['probe_exact']=False
            if mode=='starved':s[0]['peer_progress']=False
            if mode=='deadline':s[0]['takeover_ns']=2_000_000_001
            with self.assertRaises(AssertionError,msg=mode):assess(s,r,q)
    def test_missing_starved_failed_or_slow_reader_not_dropped(self):
        for mode in ['empty','missing','failed','timeout']:
            s,r,q=good()
            if mode=='empty':q=[]
            if mode=='missing':q.pop()
            if mode=='failed':q[0]['ok']=False
            if mode=='timeout':q[0]['elapsed_ns']=2_000_000_001
            with self.assertRaises(AssertionError,msg=mode):assess(s,r,q)
    def test_resource_overflow_or_missing_data_cannot_pass(self):
        for rows in [[],[dict(time_ns=1,rss_bytes=RSS_CAP+1,processes=7)],[dict(time_ns=1,rss_bytes=1,processes=0)]]:
            s,_,q=good()
            with self.assertRaises(AssertionError):assess(s,rows,q)
    def test_percentiles_are_raw_samples_not_percentiles_of_percentiles(self):
        s,r,q=good()
        for i,x in enumerate(s):x['takeover_ns']=(i+1)*1_000_000
        result=assess(s,r,q);self.assertEqual(result['takeover_all']['n'],54);self.assertEqual(result['takeover_all']['p95_ns'],51_000_000);self.assertEqual(result['takeover_all']['max_ns'],54_000_000)
if __name__=='__main__':unittest.main()
