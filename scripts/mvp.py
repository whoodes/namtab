#!/usr/bin/env python3
"""Fresh native feedback launch. Never deletes sockets or stops user sessions."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parent.parent

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('command', choices=['start', 'attach'])
    parser.add_argument('--run', type=Path)
    args = parser.parse_args()
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target'))
    if not target.is_absolute():
        target = ROOT / target
    binary = target / 'debug'
    if os.environ.get('CARGO_BUILD_TARGET'):
        raise SystemExit('Native feedback launch requires native Linux binaries, no cross target.')
    if not os.environ.get('DISPLAY'):
        raise SystemExit('This admitted MVP requires an accessible X11 DISPLAY. No headless substitute or security workaround.')
    for name in ['namtab-window', 'namtabd', 'namtab-terminal-worker', 'namtab-pty-helper', 'namtab-demo']:
        if not (binary / name).is_file():
            raise SystemExit('Missing same-build binary: ' + name + '; run make mvp-build')
    fresh = args.command == 'start'
    if fresh:
        if args.run:
            raise SystemExit('start always creates a fresh namespace; --run is for explicit attach only')
        (ROOT / '.artifacts').mkdir(mode=0o700, parents=True, exist_ok=True)
        run = ROOT / '.artifacts' / ('mvp-' + uuid.uuid4().hex[:10])
        run.mkdir(mode=0o700)
        workspace = run / 'workspace'
        workspace.mkdir(mode=0o700)
        sock = run / 's'
        if len(os.fsencode(sock)) > 107:
            raise SystemExit('Project-local Linux socket path exceeds107bytes; no outside-project fallback.')
        log = (run / 'daemon.log').open('xb')
        cmd = [binary / 'namtabd', '--socket', sock, '--workspace', workspace,
               '--worker', binary / 'namtab-terminal-worker', '--pty-helper', binary / 'namtab-pty-helper']
        # TERM=dumb fallback is explicit; no global or existing terminfo edits.
        daemon = subprocess.Popen([str(x) for x in cmd], cwd=ROOT, stdin=subprocess.DEVNULL, stdout=log, stderr=log, start_new_session=True)
        deadline = time.monotonic() + 5
        while not sock.exists():
            if daemon.poll() is not None or time.monotonic() > deadline:
                raise SystemExit('Owned broker startup failed/timed out; retained run ' + str(run) + '; inspect daemon.log. No socket deletion or automatic restart.')
            time.sleep(.025)
        (run / 'launch.json').write_text(json.dumps({'socket': str(sock), 'workspace': str(workspace), 'daemon_pid': daemon.pid,
                                                    'limits': 'captureoff,2PTY/2managedattempt lifetime; closing view detaches; no process recovery'}, indent=2) + '\n')
        print('Fresh native run:', run, '\nWorkspace:', workspace, '\nOwned broker PID:', daemon.pid, flush=True)
    else:
        if args.run is None:
            raise SystemExit('attach requires --run with an existing project-local run directory')
        run = args.run.resolve(strict=True)
        if not run.is_relative_to(ROOT / '.artifacts') or not run.is_dir():
            raise SystemExit('attach run must be inside this project .artifacts; no automatic socket discovery')
        sock = run / 's'
    print('Attach again: python3 -B scripts/mvp.py attach --run ' + str(run), flush=True)
    print('Work Ctrl+Shift+1 | Terminal Ctrl+Shift+2 | Take input Ctrl+Shift+T | Demo Ctrl+Shift+D', flush=True)
    window_log = run / ('window-' + uuid.uuid4().hex[:8] + '.log')
    print('Window diagnostics:', window_log, flush=True)
    with window_log.open('xb') as log:
        cmd = [str(binary / 'namtab-window'), '--socket', str(sock)] + (['--new-shell'] if fresh else [])
        result = subprocess.run(cmd, cwd=ROOT, stdout=log, stderr=log, check=False)
    print('Window exited', result.returncode, '; broker/work left running intentionally at', run)
    if result.returncode:
        raise SystemExit(result.returncode)

if __name__ == '__main__':
    main()
