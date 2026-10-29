#!/usr/bin/env python3
"""Finite synthetic workload copied to one owned workspace; no providers/network."""
import json
import os
from pathlib import Path
import select
import sys
import termios
import time
import tty

kind=sys.argv[1]
end=time.monotonic()+30
if kind=='pty':
    saved=termios.tcgetattr(0);tty.setraw(0)
    try:
        Path('pty-ready').write_text('ready');seq=0
        while not Path('finish').exists() and time.monotonic()<end:
            phase=Path('phase').read_text() if Path('phase').exists() else 'quiet'
            if phase in ('noisy','stalled'):
                seq+=1;sys.stdout.buffer.write(b'B_PTY_'+b'P'*1794+b'\x1b[3;7H\x1b[6n');sys.stdout.buffer.flush();reply=b'';deadline=time.monotonic()+.5
                while len(reply)<6 and time.monotonic()<deadline:
                    if select.select([0],[],[],max(0,deadline-time.monotonic()))[0]:reply+=os.read(0,64)
                if reply!=b'\x1b[3;7R':raise RuntimeError('missing/misordered/duplicate terminal reply')
                if select.select([0],[],[],.005)[0]:raise RuntimeError('extra terminal reply/input')
                with Path('replies.jsonl').open('a') as f:f.write(json.dumps({'seq':seq,'reply_hex':reply.hex()})+'\n')
            time.sleep(.05)
    except BaseException as error:
        Path('pty-failure').write_text(type(error).__name__+': '+str(error));raise
    finally:
        termios.tcsetattr(0,termios.TCSANOW,saved)
        sys.stdout.buffer.write(b'\r\n');sys.stdout.buffer.flush()
        Path('pty-returned').write_text('raw-mode restored; clean output row')
else:
    label='A' if kind=='managed-A' else 'B';seq=0
    while not Path('finish').exists() and time.monotonic()<end:
        seq+=1
        with Path('ticks-'+label).open('ab') as f:f.write(b'x')
        phase=Path('phase').read_text() if Path('phase').exists() else 'quiet'
        if label=='B' and phase in ('noisy','stalled'):sys.stdout.write('ONLY_B_'+'B'*1792+'\n');sys.stdout.flush()
        elif label=='A':sys.stdout.write('ONLY_A_'+str(seq)+'\n');sys.stdout.flush()
        time.sleep(.05)
    Path('exit-'+label).write_text('finished')
