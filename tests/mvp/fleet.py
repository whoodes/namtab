#!/usr/bin/env python3
"""NAT1 actual owned X11 journey; two SCRIPTED public agents, never a model."""
import argparse
import json
from pathlib import Path
import select
import signal
import subprocess
import time
from native import Native
from e2e import Connection, require, sha256

class Fleet(Native):
    def input_once(self,c,sid,intent):
        s=next(x for x in c.observe()['sessions'] if x['id']==sid)
        r=c.call({'op':'input','session':sid,'generation':s['generation'],'revision':s['screen']['revision'],'mode_revision':s['screen']['mode_revision'],'policy':'observed','intent':intent})
        require(r['ok'],'scripted legacy input refused; never retry blindly')
        return r
    def cleanup(self):
        # Destroy our owned focus sink before closing the native test window.
        # Always finish cleanup; a failure must never leave a PASS record.
        try:
            try:
                if self.sink and self.x:self.x.destroy(self.sink)
            finally:
                self.sink=None
                super().cleanup()
        except Exception as error:
            self.record['verdict']='FAIL'
            self.record['cleanup_error']={'type':type(error).__name__,'message':str(error)}
            raise
    def journey(self):
        (self.artifacts/'fleet.py').write_bytes(Path(__file__).read_bytes());self.record['fleet_harness_sha256']=sha256(Path(__file__))
        self.record['binary_hashes']={n:sha256(self.bin_dir/n) for n in ['namtab-window','namtabd','namtab-terminal-worker','namtab-pty-helper']}
        self.wait(lambda:self.x.focus()==self.window_id,'actual own native focus');self.event('focus_event',predicate=lambda d:d['focused'])
        a=Connection(self.socket_path,'SCRIPTED-A','agent');b=Connection(self.socket_path,'SCRIPTED-B','agent')
        try:
            s=self.session(self.first);require(a.call({'op':'take_control','session':s['id'],'generation':s['generation'],'force':False})['ok'],'A shell claim')
            second=b.call({'op':'create_session','name':'scripted-peer-terminal'});require(second['ok'],'second shell create')
            s2=next(s for s in b.observe()['sessions'] if s['id']!=self.first);sid2=s2['id']
            require(b.call({'op':'take_control','session':sid2,'generation':s2['generation'],'force':False})['ok'],'B shell claim')
            self.wait_session(sid2,self.at_prompt,'second real shell ready')
            self.input_once(b,sid2,{'kind':'text','text':'while ! test -f stoppty; do echo TICK_B; sleep .1; done'})
            self.wait_session(sid2,lambda s:any('while ! test' in x for x in s['screen']['lines']),'single text echo before Enter')
            self.input_once(b,sid2,{'kind':'key','key':'Enter','ctrl':False,'alt':False,'shift':False})
            self.wait_session(sid2,lambda s:any(x.strip()=='TICK_B' for x in s['screen']['lines']),'other terminal live output')
            contract=a.call({'op':'define_task','task':'fleet-shared','goal':'Scripted independent work and focused human control','expected_artifacts':[]});rev=contract['receipt']['result']['revision']
            ra=a.call({'op':'run','task':'fleet-shared','task_revision':rev,'argv':['/usr/bin/python3','-u','-c',"import os,time;print('A_NATIVE_PID='+str(os.getpid()));time.sleep(60)"]})
            rb=b.call({'op':'run','task':'fleet-shared','task_revision':rev,'argv':['/usr/bin/python3','-u','-c',"import os,time,pathlib,sys;print('B_NATIVE_PID='+str(os.getpid()));print('B'*100000);p=pathlib.Path('native-peer-ticks');\nwhile not pathlib.Path('finish-native-peer').exists():\n with p.open('ab') as f:f.write(b'x')\n print('B_NATIVE_TICK',file=sys.stderr);time.sleep(.02)"]})
            require(ra['ok'] and rb['ok'],'two real managed attempts')
            aid=ra['receipt']['result']['attempt'];bid=rb['receipt']['result']['attempt']
            s=self.wait_snapshot(lambda s:len(s['attempts'])==2 and all('\n' in x['stdout'] for x in s['attempts']) and (self.workspace/'native-peer-ticks').exists(),'actual attributed workflow')
            require({x['origin']['endpoint'] for x in s['attempts']}=={a.endpoint,b.endpoint} and all(x['task_revision']==rev for x in s['attempts']),'attribution/revision mismatch')
            self.descendants();apid=int(next(x['stdout'].splitlines()[0].split('=')[1] for x in s['attempts'] if x['id']==aid));bfdpid=int(next(x['stdout'].splitlines()[0].split('=')[1] for x in s['attempts'] if x['id']==bid))
            require(apid in self.owned_pidfds and bfdpid in self.owned_pidfds,'own verified child markers unavailable')
            afd=self.owned_pidfds[apid];bfd=self.owned_pidfds[bfdpid]
            self.hotkey('1');self.x.key(self.window_id,'Home');time.sleep(.15)
            self.capture('01-fleet-roster.png')
            mark=self.mark();self.hotkey('x');self.event('cancel_confirmation_open',mark,lambda d:d['attempt']==aid);self.capture('02-confirm-A.png')
            self.x.key(self.window_id,'Escape');self.event('cancel_confirmation_dismissed',mark)
            self.hotkey('x');self.x.key(self.window_id,'Down');self.x.key(self.window_id,'Return');time.sleep(.2)
            require(not any(x['cancellation'] for x in self.snapshot()['attempts']),'selection retargeted or Escape sent cancellation')
            self.capture('03-selected-peer-detail.png')
            self.x.key(self.window_id,'Home');mark=self.mark();self.hotkey('x');self.event('cancel_confirmation_open',mark)
            self.sink=self.x.sink(self.window_id);self.x.focus_owned(self.sink,{self.window_id,self.sink})
            self.event('focus_event',mark,lambda d:not d['focused']);self.x.key(self.window_id,'Return');time.sleep(.2)
            require(not any(x['cancellation'] for x in self.snapshot()['attempts']),'unfocused confirmation acted')
            self.x.focus_owned(self.window_id,{self.window_id,self.sink});self.event('focus_event',mark,lambda d:d['focused'])
            self.x.key(self.window_id,'Return');time.sleep(.2)
            require(not any(x['cancellation'] for x in self.snapshot()['attempts']),'focus restoration replayed confirmation')
            self.x.destroy(self.sink);self.sink=None
            # Exact selected terminal; the managed attempt does not imply a PTY.
            self.x.key(self.window_id,'Home');self.x.key(self.window_id,'Down');self.x.key(self.window_id,'Down');self.x.key(self.window_id,'Return');time.sleep(.15)
            stale=self.session(self.first);peer_before=self.session(sid2)
            self.arm();self.wait_session(self.first,lambda s:s['owner']==self.native_endpoint,'selected human takeover')
            require(self.session(sid2)['generation']==peer_before['generation'] and self.session(sid2)['owner']==b.endpoint,'takeover changed other terminal')
            refused=a.call({'op':'input','session':self.first,'generation':stale['generation'],'revision':stale['screen']['revision'],'mode_revision':stale['screen']['mode_revision'],'policy':'observed','intent':{'kind':'text','text':'STALE_NAT'}})
            require(not refused['ok'] and refused['receipt']['delivered_bytes']==0,'stale writer acted')
            self.command('printf x>>nat-count;echo NAT_ONE','NAT_ONE');self.command('printf x>>nat-count;echo NAT_TWO','NAT_TWO')
            self.x.key(self.window_id,'Up');self.x.key(self.window_id,'Return');self.wait(lambda:(self.workspace/'nat-count').read_bytes()==b'xxx','repeated input/history exact side effect')
            self.type('sleep 20');self.x.key(self.window_id,'Return');time.sleep(.2);self.descendants();self.x.key(self.window_id,'c',4);self.wait_session(self.first,self.at_prompt,'this shell Ctrl-C interruption')
            self.capture('04-targeted-takeover.png')
            self.hotkey('Tab');mark=self.mark();self.type('UNARMED_NAT');self.x.key(self.window_id,'Return');time.sleep(.2)
            require(not any(r['data']['event']=='input_dispatch' and r['sequence']>mark for r in self.events()),'session navigation reused armed input')
            self.hotkey('1');self.x.key(self.window_id,'Home');mark=self.mark();self.hotkey('x');self.event('cancel_confirmation_open',mark,lambda d:d['attempt']==aid)
            self.x.key(self.window_id,'Return');self.x.key(self.window_id,'Return')
            self.event('cancel_result',mark,lambda d:d['ok'])
            stopped=self.wait_snapshot(lambda s:any(x['id']==aid and x['state']=='exited' for x in s['attempts']),'A observed direct child exit')
            self.wait(lambda:select.select([afd],[],[],0)[0],'independent owned A exit')
            require(not select.select([bfd],[],[],0)[0],'peer exited with target')
            ticks=(self.workspace/'native-peer-ticks').stat().st_size;self.wait(lambda:(self.workspace/'native-peer-ticks').stat().st_size>ticks+2,'peer writes after A exit')
            ended=next(x for x in stopped['attempts'] if x['id']==aid);require(ended['origin']['endpoint']==a.endpoint and ended['cancellation']['requested_by']==self.native_endpoint,'handoff relabelled launch/cancel origin')
            self.capture('05-selected-cancel-observed.png')
            # The second attempt naturally exits with its confirmation open.
            self.x.key(self.window_id,'Home');self.x.key(self.window_id,'Down');mark=self.mark();self.hotkey('x');self.event('cancel_confirmation_open',mark,lambda d:d['attempt']==bid)
            (self.workspace/'finish-native-peer').write_bytes(b'finish')
            self.wait_snapshot(lambda s:all(x['state']=='exited' for x in s['attempts']),'peer natural exit');self.event('cancel_confirmation_dismissed',mark)
            self.x.key(self.window_id,'Return');self.x.key(self.window_id,'Return');time.sleep(.2)
            require(next(x for x in self.snapshot()['attempts'] if x['id']==bid)['cancellation'] is None,'stale confirmation cancelled exited peer')
            require(sum(r['data']['event']=='cancel_dispatch' for r in self.events())==1,'confirmation replayed cancellation')
            # Owned broker only: UI must stay navigable while its RPC expires.
            self.process.send_signal(signal.SIGSTOP)
            try:
                time.sleep(2.5);self.hotkey('1');self.x.key(self.window_id,'Home');self.hotkey('x');self.capture('06-disconnected-roster.png')
                require(any(r['data']['event']=='key_received' and not r['data']['online'] for r in self.events()),'actual UI offline state not observed')
            finally:self.process.send_signal(signal.SIGCONT)
            self.observer.close()
            time.sleep(.05)  # Let this owned broker release the expired observer slot.
            self.observer=Connection(self.socket_path,'fleet-observer-after-pause','human')
            self.reconnect();require(self.snapshot()['epoch']==s['epoch'],'live-owner reconnect changed namespace')
            # Closed owned target must become a checked Python error, not fatal
            # Xlib exit that skips finally. The native window remains usable.
            closed=self.x.sink(self.window_id);self.x.destroy(closed)
            try:self.x.focus_owned(closed,{self.window_id,closed})
            except RuntimeError as error:require('Owned X11 fixture request failed' in str(error),'focus guard is not the X error-handler regression')
            else:raise AssertionError('Invalid owned X target did not report a checked failure')
            (self.workspace/'stoppty').write_bytes(b'stop')
            self.record['checks']=[{'name':n,'verdict':'PASS'} for n in ['two_scripted_clients_exact_task_attempt_attribution','actual_keyboard_roster_selection_and_detail','dismiss_selection_focus_loss_no_cancel','targeted_takeover_stale_writer_zero_bytes','repeated_input_history_and_this_shell_interrupt','one_shot_human_cancel_independent_OS_exit_peer_continuity','stale_natural_exit_confirmation_no_dispatch','disconnected_native_navigation_explicit_live_resume','checked_invalid_owned_x_target_does_not_skip_cleanup']]
            self.record['outcomes']={'attempts':self.snapshot()['attempts'],'native_endpoint':self.native_endpoint,'cancel_dispatch_count':1,'human_input_side_effect':(self.workspace/'nat-count').read_text(),'stale_writer_receipt':refused['receipt'],'peer_ticks_after_A_exit':ticks,'independent_A_pidfd_exit':True,'model_execution':'NOT RUN: SCRIPTED PROTOCOL CLIENTS'}
            self.record['limitations']=['Existing2PTY/2attempt/fourconnection/retainedidentity profile, not fleet capacity','Synthetic X11 events and existing finite core/font tuple; no physical held-key/IME/a11y or final toolkit qualification','Pidfd proves own child exit, not cause/status; no descendant/all-effects guarantee','Disconnected last observation is not current process truth; explicit reconnect, no input or cancel replay']
        finally:a.close();b.close()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bin-dir',type=Path,required=True);p.add_argument('--terminfo',type=Path,required=True);p.add_argument('--artifacts',type=Path,required=True)
    Fleet(p.parse_args()).run()
