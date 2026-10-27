#!/usr/bin/env python3
"""Actual native-window journey. Synthetic X11 events target only our verified PID/window.
No physical-human, IME, screen-reader, broad compatibility or performance claim.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import traceback
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'mk1'))
from e2e import Journey as Base, Connection, require, sha256
from x11 import X11

class Native(Base):
    def start(self):
        self.window=None;self.x=None;self.sink=None
        self.trace_path=self.artifacts/"events.jsonl"
        for source in [Path(__file__), Path(__file__).with_name("x11.py")]:
            target=self.artifacts/source.name;target.write_bytes(source.read_bytes())
            self.record.setdefault("native_harness",[]).append({"path":str(source.relative_to(Path.cwd())),"sha256":sha256(source)})
        command=[str(self.bin_dir/'namtabd'),'--socket',str(self.socket_path),'--workspace',str(self.workspace),'--worker',str(self.bin_dir/'namtab-terminal-worker'),'--pty-helper',str(self.bin_dir/'namtab-pty-helper')]
        self.daemon_log=(self.artifacts/'daemon.log').open('xb')
        self.process=subprocess.Popen(command,stdin=subprocess.DEVNULL,stdout=self.daemon_log,stderr=subprocess.STDOUT,start_new_session=True)
        self.wait(lambda:self.socket_path.exists() or self.process.poll() is not None,'owned broker socket')
        require(self.process.poll() is None,'owned broker startup failed')
        self.observer=Connection(self.socket_path,'native-test-observer','human')
        self.window_log=(self.artifacts/'native.log').open('xb')
        self.window=subprocess.Popen([str(self.bin_dir/'namtab-window'),'--socket',str(self.socket_path),'--new-shell'],stdin=subprocess.DEVNULL,stdout=self.window_log,stderr=subprocess.STDOUT,env={**os.environ,"NAMTAB_MVP_TRACE":str(self.trace_path)})
        self.x=X11();self.window_id=self.wait(lambda:self.x.find(self.window.pid) if self.window.poll() is None else None,'owned native window ID')
        self.wait_snapshot(lambda s:len(s['sessions'])==1,'native-created shell')
        s=self.snapshot();self.first=s['sessions'][0]['id'];self.native_endpoint=next(e['id'] for e in s['endpoints'] if e['role']=='human' and e['id']!=self.observer.endpoint)
        self.record['native']={'pid':self.window.pid,'window_id':self.window_id,'binary_sha256':sha256(self.bin_dir/'namtab-window'),'font_sha256':sha256(Path('/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf')),'input':'Synthetic XSendEvent to own PID-bound window; actual X focus transfers only between two owned windows; not physical keyboard','display':'Actual existing X11 display, no Xvfb/headless substitution'}
        self.capture('01-work-empty.png')
    def events(self):
        result=[]
        if self.trace_path.exists():
            for line in self.trace_path.read_text().splitlines():
                try:row=json.loads(line)
                except ValueError:continue
                require(row.get('event') not in ['trace_gap','trace_truncated'],'Trace incomplete; evidence invalid')
                result.append(row)
        return result
    def mark(self):return max([r['sequence'] for r in self.events()]+[0])
    def event(self,name,after=0,predicate=lambda d:True):
        return self.wait(lambda:next((r for r in self.events() if r['sequence']>after and r['data']['event']==name and predicate(r['data'])),None),'native trace '+name)
    def arm(self):
        mark=self.mark();self.hotkey('t');return self.event('input_armed',mark,lambda d:d['armed'])
    def reconnect(self):
        mark=self.mark();self.hotkey('c');requested=self.event('reconnect_requested',mark)
        return self.event('connection_ready',requested['sequence'],lambda d:d['resumed'] and d['endpoint']==self.native_endpoint)
    def capture(self,name):
        time.sleep(.15)
        subprocess.run(['import','-window',str(self.window_id),str(self.artifacts/name)],check=True,timeout=5)
        self.record.setdefault('screenshots',[]).append({'path':name,'sha256':sha256(self.artifacts/name),'source':'actual owned native window pixels'})
    def hotkey(self,name):
        self.x.key(self.window_id,name,5);time.sleep(.12)
    def type(self,text):
        names={'#':'numbersign',' ':'space',"'":'apostrophe','"':'quotedbl','\\':'backslash',';':'semicolon','>':'greater','-':'minus','_':'underscore','/':'slash','(':'parenleft',')':'parenright','[':'bracketleft',']':'bracketright',',':'comma',':':'colon','.' : 'period'}
        for c in text:
            self.x.key(self.window_id,names.get(c,c),1 if c.isupper() or c in '#_>"()' else 0);time.sleep(.02)
    def command(self,text,marker):
        self.type(text);self.x.key(self.window_id,'Return');self.wait_session(self.first,lambda s:marker in [line.strip() for line in s['screen']['lines']],'native real output '+marker)
    def journey(self):
        wrong=subprocess.run([str(self.bin_dir/'namtab-demo'),'--socket',str(self.socket_path),'--epoch','not-the-observed-epoch'],capture_output=True,timeout=3)
        require(wrong.returncode!=0 and not self.snapshot()['attempts'],'wrong-epoch scripted helper admitted effects')
        self.wait(lambda:self.x.focus()==self.window_id,'actual owned native focus')
        self.event('focus_event',predicate=lambda d:d['focused']);self.hotkey('2');self.arm()
        self.wait_session(self.first,lambda s:s['owner']==self.native_endpoint,'native human takeover')
        self.command('printf x>>native-count;echo NATIVE_ONE','NATIVE_ONE')
        self.wait(lambda:(self.workspace/'native-count').exists(),'native side effect')
        require((self.workspace/'native-count').read_bytes()==b'x','first command side effect not exact')
        self.command('printf x>>native-count;echo NATIVE_TWO','NATIVE_TWO');require((self.workspace/'native-count').read_bytes()==b'xx','repeated command side effect not exact')
        self.x.key(self.window_id,'Up');self.x.key(self.window_id,'Return');self.wait(lambda:(self.workspace/'native-count').read_bytes()==b'xxx','native history navigation')
        self.capture('02-terminal.png')
        self.type('sleep 20');self.x.key(self.window_id,'Return');time.sleep(.2);self.descendants();self.x.key(self.window_id,'c',4)
        self.wait_session(self.first,self.at_prompt,'native Ctrl-C returns this shell prompt')
        self.hotkey('d')
        self.wait_snapshot(lambda s:len(s['attempts'])==2 and len(s['sessions'])==2,'two scripted real attempts and second shell')
        self.wait_snapshot(lambda s:all(a['state']=='exited' and a['exit_code']==0 for a in s['attempts']) and bool(s['artifacts']),'scripted file observation and complete process facts')
        s=self.snapshot();attempts=s['attempts'];require(attempts[0]['origin']['endpoint']!=attempts[1]['origin']['endpoint'],'cross-client attribution missing')
        require(attempts[0]['task_revision']==attempts[1]['task_revision'] and attempts[0]['task_revision'],'shared exact contract absent')
        self.capture('03-scripted-work.png')
        self.hotkey('2');self.hotkey('Tab');self.arm()
        second=next(s['id'] for s in s['sessions'] if s['id']!=self.first)
        self.wait_session(second,lambda s:s['owner']==self.native_endpoint,'human takeover from scripted client')
        self.wait(lambda:b'"stale_input_refused":true' in (self.artifacts/'native.log').read_bytes(),'scripted stale writer refused')
        require(all('STALE_SCRIPT' not in line for s in self.snapshot()['sessions'] for line in s['screen']['lines']),'stale bytes entered canonical screen')
        self.capture('04-takeover.png')
        old=self.native_endpoint;resumed=self.reconnect()
        require(resumed['data']['epoch']==s['epoch'],'reconnect changed epoch')
        # Resize changes window only until explicit broker resize, then input remains paused.
        before=self.snapshot()['sessions'][1]['screen']['cols'];self.x.resize(self.window_id,920,800);time.sleep(.15)
        self.hotkey('r');self.wait_session(second,lambda s:s['screen']['cols']==88,'explicit native broker resize');self.arm()
        self.capture('05-resize.png')
        # Reproduce the original harness assumption without executing a command.
        # A core synthetic FocusOut does NOT establish XInput2/window-library focus loss.
        actual_before=self.x.focus();mark=self.mark();self.x.focus_event(self.window_id,False)
        self.type('#CORE_FOCUS_CANARY');self.x.key(self.window_id,'Return')
        self.event('input_result',mark);time.sleep(.3)
        core=[r for r in self.events() if r['sequence']>mark]
        delivered=sum((r['data'].get('delivered_bytes') or 0) for r in core if r['data']['event']=='input_result')
        require(self.x.focus()==actual_before==self.window_id,'Core-focus reproduction changed actual focus')
        require(delivered==len('#CORE_FOCUS_CANARY')+1,'Core-focus delivered bytes not exact')
        require(not any(r['data']['event']=='focus_event' and not r['data']['focused'] for r in core),'Unexpected real focus loss in core-focus reproduction')
        self.record['core_focus_reproduction']={'actual_before':actual_before,'actual_after':self.x.focus(),'delivered_bytes':delivered,'events':core,'interpretation':'Fake core FocusOut did not establish focus loss; accepted input is consistent with actual focus. Original eighth run lacked these measurements.'}
        # Actual server focus transfer: only our native window and newly created neutral sink.
        before=self.snapshot();lines=next(s['screen']['lines'] for s in before['sessions'] if s['id']==second)
        mark=self.mark();self.sink=self.x.sink(self.window_id);time.sleep(.15)
        self.x.focus_owned(self.sink,{self.window_id,self.sink})
        loss=self.event('focus_event',mark,lambda d:not d['focused'])
        actual_lost=self.x.focus();self.type('WRONG_FOCUS');self.x.key(self.window_id,'Return');time.sleep(.25)
        end=self.mark();lost_events=[r for r in self.events() if loss['sequence']<=r['sequence']<=end]
        require(actual_lost==self.sink and self.x.focus()==self.sink,'Actual focus not retained by owned sink')
        require(not any(r['data']['event'] in ['input_queued','input_dispatch','input_result'] for r in lost_events),'Input routed after confirmed focus loss')
        require(next(s['screen']['lines'] for s in self.snapshot()['sessions'] if s['id']==second)==lines,'Actual unfocused bytes reached the app')
        self.x.focus_owned(self.window_id,{self.window_id,self.sink})
        gained=self.event('focus_event',end,lambda d:d['focused']);self.type('RESTORED_BUT_PAUSED');self.x.key(self.window_id,'Return');time.sleep(.2)
        require(not any(r['data']['event']=='input_queued' and r['sequence']>gained['sequence'] for r in self.events()),'Focus restoration replayed/rearmed input')
        self.record['actual_focus_regression']={'native_window':self.window_id,'sink_window':self.sink,'actual_focus_during':actual_lost,'loss_event':loss,'gain_event':gained,'events_during_loss':lost_events,'application_delivered_bytes_inferred':0,'no_input_receipt_exists':True,'scope':'X server + XI2 library delivery, frontend no-queue check; unit/connector tests independently test local fence, not a physical-human guarantee'}
        self.x.destroy(self.sink);self.sink=None;self.arm()
        # Native paint/control remains serviceable while only our owned broker is stopped.
        import signal
        self.process.send_signal(signal.SIGSTOP)
        try:
            time.sleep(2.4);self.hotkey('1');self.capture('06-disconnected-work.png')
        finally:self.process.send_signal(signal.SIGCONT)
        # The independent observer also expired while the owned broker was stopped.
        # Explicitly resume this read-only test endpoint; no mutation is replayed.
        token=self.observer.resume;next_id=self.observer.next_id;self.observer.close()
        self.observer=Connection(self.socket_path,'native-test-observer','human',resume=token,next_id=next_id)
        resumed_stall=self.reconnect()
        require(resumed_stall['data']['epoch']==resumed['data']['epoch'] and resumed_stall['data']['next_request_id']>resumed['data']['next_request_id'],'explicit resume did not preserve bound identity and advance request counter')
        after_resume=self.events()
        require(not any(r['data']['event']=='input_dispatch' and r['sequence']>resumed_stall['sequence'] for r in after_resume),'input replay observed after resume')
        self.record['reconnect']={'before_stall':resumed,'after_stall':resumed_stall,'input_dispatch_after_resume_observed':0,'limit':'No queued mutation was outstanding in this GUI stall. Connector fault-injection tests separately cover partial/unknown pending input.'}
        self.hotkey('a');self.type('["/usr/bin/touch","SHOULD_NOT_RUN"]');self.x.key(self.window_id,'Return')
        def refused_run():
            for line in (self.artifacts/'native.log').read_text().splitlines():
                try:r=json.loads(line)
                except ValueError:continue
                if r.get('event')=='native_receipt' and r.get('operation')=='run' and r.get('state')=='rejected':return True
            return False
        self.wait(refused_run,'actual native managed-capacity refusal receipt')
        require(len(self.snapshot()['attempts'])==2 and not (self.workspace/'SHOULD_NOT_RUN').exists(),'capacity silently expanded or ran refused command')
        self.capture('07-capacity-refusal.png')

        self.record['checks'].extend([{'name':n,'verdict':'PASS'} for n in ['actual_native_window_and_pixel_capture','real_shell_repeated_commands_history_and_interrupt','two_scripted_clients_exact_contract_and_stream_origins','human_takeover_stale_agent_refused','live_owner_reconnect_no_input_replay','explicit_resize_and_rearm','focus_loss_refuses_application_input','native_paints_while_own_broker_stopped','capacity_refusal_no_hidden_restart_or_effect']])
        self.record['limitations']=['Actual native X11 window with targeted synthetic OS keyboard events; no physical-human/IME/a11y/shaping/Wayland/production qualification','Scripted clients, no model/provider integration; observed file is not sealed causal dependency','Finite MK1 limits; no capacity/performance or durable recovery claim']
    def cleanup(self):
        if self.window and self.window.poll() is None:
            self.descendants([self.process.pid,self.window.pid]);self.window.terminate();self.window.wait(timeout=3)
        if self.x:
            if self.sink:self.x.destroy(self.sink)
            self.x.close()
        if hasattr(self,'window_log'):self.window_log.close()
        super().cleanup()
    def run(self):
        try:self.start();self.journey();self.record['verdict']='PASS'
        except Exception as e:
            if self.window and self.window.poll() is None:
                try:self.capture('failure.png')
                except Exception:pass
            self.record['verdict']='FAIL';self.record['error']={'type':type(e).__name__,'message':str(e)};(self.artifacts/'failure.txt').write_text(traceback.format_exc());raise
        finally:
            try:self.cleanup()
            finally:(self.artifacts/'checks.json').write_text(json.dumps(self.record,indent=2)+'\n')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--terminfo',type=Path,required=True);p.add_argument('--artifacts',type=Path,required=True)
    Native(p.parse_args()).run()
