"""Deterministic protocol fixture, never a model. Owns only its bridge child."""
import json
import os
import select
import subprocess
import time
class MCP:
    def __init__(self,binary,socket,epoch,log,cwd):
        self.stderr=log.open('xb');self.pending=b'';self.next=1;self.transcript=[]
        self.process=subprocess.Popen([str(binary),'--socket',str(socket),'--epoch',epoch],cwd=cwd,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.stderr)
        self.call('initialize',{'protocolVersion':'2025-11-25','clientInfo':{'name':'deterministic-protocol-fixture','version':'0'},'capabilities':{}})
        self.send({'jsonrpc':'2.0','method':'notifications/initialized'})
    def send(self,value):
        raw=json.dumps(value,separators=(',',':')).encode()+b'\n';assert len(raw)<=65536
        self.process.stdin.write(raw);self.process.stdin.flush()
    def response(self):
        end=time.monotonic()+4
        while b'\n' not in self.pending:
            remaining=end-time.monotonic()
            if remaining<=0 or not select.select([self.process.stdout],[],[],remaining)[0]:raise AssertionError('MCP response timeout; never retry mutation')
            raw=os.read(self.process.stdout.fileno(),8192)
            if not raw:raise AssertionError('MCP transport EOF; effects may be unknown')
            self.pending+=raw
            if len(self.pending)>262144:raise AssertionError('MCP response cap')
        line,self.pending=self.pending.split(b'\n',1);return json.loads(line)
    def call(self,method,params,repeat_id=None):
        id=self.next if repeat_id is None else repeat_id
        if repeat_id is None:self.next+=1
        value={'jsonrpc':'2.0','id':id,'method':method,'params':params}
        self.send(value);response=self.response();assert response['id']==id
        self.transcript.append({'request':value,'response':response})
        return response
    def tool(self,name,args,ok=True):
        r=self.call('tools/call',{'name':name,'arguments':args})
        if ok:assert not r.get('error') and not r['result'].get('isError'),r
        return r
    def value(self,name,args):return self.tool(name,args)['result']['structuredContent']['observation_or_response']
    def close(self):
        if self.process.poll() is None:
            self.process.stdin.close()
            try:self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:self.process.terminate();self.process.wait(timeout=3)
        self.process.stdout.close();self.stderr.close()
