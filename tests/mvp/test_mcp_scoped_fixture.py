"""Counterexamples for evidence certification; never starts a process or model."""
import unittest
from mcp_scoped import validate_read_trace

def trace():
    return [{'request':{'id':1,'operation':{'op':'hello'}},'response':{'id':1,'endpoint':'p'}}, {'request':{'id':2,'operation':{'op':'observe'}},'response':{'id':2}}, {'request':{'id':3,'operation':{'op':'observe_scoped','epoch':'e','scope':{'kind':'attempt','attempt':'a'}}},'response':{'id':3,'epoch':'e','endpoint':'p','ok':True,'snapshot':None,'receipt':None,'resume':None,'observation':{'epoch':'e','endpoint':'p','profile':'namtab.scoped-observation/0','view':{'kind':'attempt','attempt':{'id':'a'}}}}}]
class Evidence(unittest.TestCase):
    def test_valid_and_read_refusal(self):
        self.assertTrue(validate_read_trace(trace()))
        rows=trace();rows[-1]['response']['endpoint']=None
        self.assertTrue(validate_read_trace(rows))
        rows=trace();rows[-1]['response'].update(ok=False,observation=None)
        self.assertTrue(validate_read_trace(rows))
    def test_full_read_fallback_not_certified(self):
        rows=trace();rows[-1]['request']['operation']={'op':'observe'}
        with self.assertRaises(AssertionError):validate_read_trace(rows)
    def test_bad_binding_mixed_view_and_cross_target_not_certified(self):
        for path,value in [(('id',),8),(('receipt',),{}),(('observation','endpoint'),'other'),(('observation','view','attempt','id'),'b')]:
            rows=trace();dest=rows[-1]['response']
            for k in path[:-1]:dest=dest[k]
            dest[path[-1]]=value
            with self.assertRaises(AssertionError):validate_read_trace(rows)
    def test_nested_work_stream_leak_not_certified(self):
        rows=trace();rows[-1]['request']['operation']['scope']={'kind':'work_export'};rows[-1]['response']['observation']['view']={'kind':'work_export','sessions':[],'attempts':[{'stdout':'unexpected'}]}
        with self.assertRaises(AssertionError):validate_read_trace(rows)
if __name__=='__main__':unittest.main()
