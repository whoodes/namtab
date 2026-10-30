"""Independent damaged protocol/evidence counterexamples; no process or provider."""
import copy
from types import SimpleNamespace
import threading
import unittest
from unittest.mock import Mock, patch
from session_watch import checked, WatchJourney, Adapter

def fixture():
    c=SimpleNamespace(epoch='e',endpoint='agent')
    f={'version':'namtab.mk1/1','profile':'namtab.session-watch/0','epoch':'e','endpoint':'agent','subscription':'s','session':'one','cursor':{'sample':1,'revision':'sha256:'+'a'*64},'coverage':'latest_published_view','history_complete':False,'canonical_pending':False,'degraded':None,'body':{'kind':'baseline','view':{'id':'one'}}}
    r={'ok':True,'error':None,'receipt':None,'snapshot':None,'observation':None,'resume':None,'watch':f}
    return c,r

class WatchEvidence(unittest.TestCase):
    def test_headers_and_missing_data_cannot_attest_a_baseline(self):
        c,r=fixture();q={'kind':'open','session':'one'};checked(r,c,q)
        for key,value in [('profile','other'),('version','other'),('epoch','other'),('endpoint','other'),('session','two'),('history_complete',True),('coverage','no_observation'),('canonical_pending',None)]:
            bad=copy.deepcopy(r);bad['watch'][key]=value
            with self.subTest(key=key),self.assertRaises(AssertionError):checked(bad,c,q)
        for field in ['receipt','snapshot','observation','resume','error']:
            bad=copy.deepcopy(r);bad[field]='unrequested'
            with self.subTest(field=field),self.assertRaises(AssertionError):checked(bad,c,q)
        for value in [True,0,2,9_007_199_254_740_992]:
            bad=copy.deepcopy(r);bad['watch']['cursor']['sample']=value
            with self.subTest(value=value),self.assertRaises(AssertionError):checked(bad,c,q)

    def test_incremental_reply_never_silently_becomes_a_baseline(self):
        c,r=fixture();old=copy.deepcopy(r['watch']);q={'kind':'next','subscription':'s','cursor':old['cursor'],'credit':1}
        r['watch']['cursor']['sample']=2;r['watch']['body']={'kind':'unchanged'};checked(r,c,q,old)
        for body in [{'kind':'baseline','view':{'id':'one'}},{'kind':'unchanged','view':{'id':'one'}},{'kind':'replaced','from':{'sample':9,'revision':old['cursor']['revision']},'view':{'id':'one'}}]:
            bad=copy.deepcopy(r);bad['watch']['body']=body
            with self.subTest(body=body),self.assertRaises(AssertionError):checked(bad,c,q,old)
        bad=copy.deepcopy(r);bad['watch']['cursor']['revision']='sha256:'+'b'*64
        with self.assertRaises(AssertionError):checked(bad,c,q,old)
        bad=copy.deepcopy(r);bad['watch']['subscription']='different'
        with self.assertRaises(AssertionError):checked(bad,c,q,old)

    def test_gap_and_close_are_no_observation_and_never_advance(self):
        c,r=fixture();old=copy.deepcopy(r['watch']);r['watch'].update(coverage='no_observation',canonical_pending=None,body={'kind':'resync_required'});q={'kind':'next','subscription':'s','cursor':old['cursor'],'credit':1};checked(r,c,q,old)
        for change in ['data','cut','pending']:
            bad=copy.deepcopy(r)
            if change=='data':bad['watch']['body']['view']={'id':'one'}
            if change=='cut':bad['watch']['cursor']['sample']=2
            if change=='pending':bad['watch']['canonical_pending']=False
            with self.subTest(change=change),self.assertRaises(AssertionError):checked(bad,c,q,old)
        r['watch']['body']={'kind':'closed'};checked(r,c,{'kind':'close','subscription':'s'},old)

    def test_failing_observer_close_does_not_skip_owned_process_cleanup(self):
        r=object.__new__(WatchJourney);r.stop_monitor=threading.Event();r.monitor=None;r.record={};r.calls=[];r.resources=[];r.faults=[];r.root_pidfd=None
        bad=Mock();bad.close.side_effect=OSError('injected');good=Mock();r.clients=[good];r.observer=bad
        with patch.object(Adapter,'cleanup') as owned:
            with self.assertRaises(AssertionError):r.cleanup()
            owned.assert_called_once();good.close.assert_called_once()
        self.assertIsNone(r.observer);self.assertEqual(r.clients,[])

if __name__=='__main__':unittest.main()
