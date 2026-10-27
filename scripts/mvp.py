#!/usr/bin/env python3
"""Fresh native feedback launch. Never deletes sockets or stops user sessions."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time
import uuid
import shlex

ROOT = Path(__file__).resolve().parent.parent

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('command', choices=['start', 'attach', 'status', 'stop', 'agent-config'])
    parser.add_argument('--run', type=Path)
    args = parser.parse_args()
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target'))
    if not target.is_absolute():
        target = ROOT / target
    binary = target / 'debug'
    if os.environ.get('CARGO_BUILD_TARGET'):
        raise SystemExit('Native feedback launch requires native Linux binaries, no cross target.')
    graphical=args.command in ['start','attach']
    if graphical and not os.environ.get('DISPLAY'):
        raise SystemExit('This admitted MVP requires an accessible X11 DISPLAY. No headless substitute or security workaround.')
    required=['namtab-window','namtabd','namtab-terminal-worker','namtab-pty-helper','namtab-demo','namtabctl','namtab-mcp'] if graphical else ['namtabctl']
    if args.command=='agent-config':required.append('namtab-mcp')
    for name in required:
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
        print('New owned run directory:',run,flush=True)
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
        info = subprocess.run([str(binary / 'namtabctl'), '--socket', str(sock), 'info'], capture_output=True, text=True, check=True)
        epoch = json.loads(info.stdout)['epoch']
        (run / 'launch.json').write_text(json.dumps({'socket': str(sock), 'workspace': str(workspace), 'daemon_pid': daemon.pid, 'epoch': epoch,
                                                    'limits': 'captureoff,2PTY/2managedattempt lifetime; closing view detaches; no process recovery'}, indent=2) + '\n')
        print('Fresh native run:', run, '\nWorkspace:', workspace, '\nOwned broker PID:', daemon.pid, flush=True)
    else:
        if args.run is None:
            raise SystemExit('attach requires --run with an existing project-local run directory')
        run = args.run.resolve(strict=True)
        if not run.is_relative_to(ROOT / '.artifacts') or not run.is_dir():
            raise SystemExit('attach run must be inside this project .artifacts; no automatic socket discovery')
        sock = run / 's'
    if args.command in ['status', 'stop', 'agent-config']:
        manifest_path=run/'launch.json'
        if manifest_path.is_symlink() or not manifest_path.is_file() or manifest_path.stat().st_size>8192:
            raise SystemExit('Launch record must be a small regular file; no PID fallback')
        manifest=json.loads(manifest_path.read_text())
        epoch=manifest.get('epoch')
        if manifest.get('socket')!=str(sock) or not isinstance(epoch,str) or not epoch:
            raise SystemExit('Run has no matching epoch-bound launch record; older runs require separate operator review, never a PID/signal fallback')
        if args.command=='agent-config':
            command=['codex','-C',manifest.get('workspace',str(run/'workspace')),'-c','mcp_servers.namtab.command='+json.dumps(str(binary/'namtab-mcp')),'-c','mcp_servers.namtab.args='+json.dumps(['--socket',str(sock),'--epoch',epoch]),'-c','mcp_servers.namtab.default_tools_approval_mode="prompt"']
            print('NOT STARTED: manual Codex configuration for an explicitly authorized workspace/provider. Other Codex tools are outside Namtab mediation.')
            print(shlex.join(command));return
        if not sock.exists():
            print('No socket at explicit run; liveness unknown, no shutdown sent. Files retained:',run)
            return
        identity=(sock.stat().st_dev,sock.stat().st_ino)
        command=[str(binary/'namtabctl'),'--socket',str(sock),'info','--epoch',epoch]
        if args.command=='stop':command=[str(binary/'namtabctl'),'--socket',str(sock),'stop','--epoch',epoch]
        result=subprocess.run(command,check=False)
        if result.returncode not in ([0,3] if args.command=='stop' else [0]):raise SystemExit(result.returncode)
        if args.command=='stop':
            deadline=time.monotonic()+5
            while True:
                try: current=sock.stat()
                except FileNotFoundError:break
                if (current.st_dev,current.st_ino)!=identity:
                    raise SystemExit('Socket replaced; shutdown completion uncertain; replacement left untouched')
                if time.monotonic()>deadline:
                    raise SystemExit('Shutdown requested; owned socket still present after5s; completion unknown. No automatic retry, PID signal or file removal.')
                time.sleep(.025)
            if result.returncode==3:print('Shutdown receipt persistence UNKNOWN; completion below is separate observation.')
            print('Owned broker socket removed after orderly shutdown; direct handles drained. Descendants/external effects may continue. Files retained:',run)
        return
    print('Stop explicitly: python3 -B scripts/mvp.py stop --run '+str(run),flush=True)
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
