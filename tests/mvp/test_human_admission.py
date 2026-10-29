"""Damaged evidence must not pass admission's ownership/population oracles."""
import copy
import unittest
import threading
from unittest.mock import Mock, patch
from human_admission import Admission, Adapter
from human_admission import stale_fence, result_population, arrival_binding

class Evidence(unittest.TestCase):
    def test_only_exact_writer_fence_passes(self):
        r={'ok':False,'id':12,'error':'writer_or_generation_stale','receipt':{'endpoint':'old','id':12,'operation':'input','state':'rejected','detail':'writer_or_generation_stale','delivered_bytes':0}}
        stale_fence(r,'old',12)
        for field,value in [('endpoint','other'),('id',13),('operation','resize'),('state','reserved'),('detail','human_live_requires_human_endpoint'),('delivered_bytes',1)]:
            bad=copy.deepcopy(r);bad['receipt'][field]=value
            with self.subTest(field=field),self.assertRaises(AssertionError):stale_fence(bad,'old',12)
        for field,value in [('id',13),('ok',True),('error','screen_revision_stale')]:
            bad=copy.deepcopy(r);bad[field]=value
            with self.subTest(field=field),self.assertRaises(AssertionError):stale_fence(bad,'old',12)

    def test_result_population_cannot_pass_vacuously(self):
        origins={'a':{},'b':{}}
        result_population([{'id':'b'},{'id':'a'}],origins)
        for ids in [[],['a'],['a','a'],['a','b','c'],['a','c']]:
            with self.subTest(ids=ids),self.assertRaises(AssertionError):result_population([{'id':i} for i in ids],origins)

    def test_concurrent_response_errors_cannot_hide_as_refusals(self):
        good={'id':1,'epoch':'e','ok':True,'endpoint':'e2','resume':'RAM-only-fixture'}
        arrival_binding(good,'e')
        refused={'id':1,'epoch':'e','ok':False,'error':'control_admission_no_eligible_transport'}
        arrival_binding(refused,'e')
        for field,value in [('id',2),('epoch','other'),('resume',None),('endpoint',None)]:
            bad=dict(good);bad[field]=value
            with self.subTest(field=field),self.assertRaises(AssertionError):arrival_binding(bad,'e')
        for field,value in [('error','unexpected'),('endpoint','should-not-bind'),('resume','should-not-bind')]:
            bad=dict(refused);bad[field]=value
            with self.subTest(field=field),self.assertRaises(AssertionError):arrival_binding(bad,'e')

    def test_client_close_failure_still_cleans_every_owned_handle(self):
        r=object.__new__(Admission);r.stop_monitor=threading.Event();r.monitor=None;r.record={};r.resources=[];r.monitor_errors=[];r.raw_sockets=[];r.root_pidfd=None;r.observer=object()
        bad=Mock();bad.close.side_effect=OSError('injected close');good=Mock();r.clients=[bad,good]
        with patch.object(Adapter,'cleanup') as owned:
            with self.assertRaises(AssertionError):r.cleanup()
            owned.assert_called_once();good.close.assert_called_once()
        self.assertIsNone(r.observer);self.assertEqual(r.clients,[]);self.assertTrue(r.record['owned_cleanup_faults'])

if __name__=='__main__':unittest.main()
