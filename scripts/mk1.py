#!/usr/bin/env python3
"""Small local MK1 launch helper. No installs, shell interpolation or server exposure."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
RUNTIME = ROOT / '.artifacts/mk1'
SOCKET = RUNTIME / 'socket'
TARGET = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target'))
if not TARGET.is_absolute():
    TARGET = ROOT / TARGET
BIN = TARGET / 'debug'
ELECTRON = ROOT / '.tools/electron/44.8.0/runtime/electron'
ELECTRON_SHA = '447dc011502a0a5ab4da365d3004646c86aee4eb03aa1643cdf672c7491b64cd'
PACKAGES = ['terminal', 'platform', 'service', 'client', 'console']


def run(args):
    subprocess.run([str(x) for x in args], cwd=ROOT, check=True)


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in ['build', 'daemon', 'tui', 'desktop', 'agent', 'terminfo']:
        raise SystemExit('usage: python3 scripts/mk1.py build|daemon|tui|desktop|agent|terminfo')
    command = sys.argv[1]
    if os.environ.get("CARGO_BUILD_TARGET"):
        raise SystemExit("MK1 launches native Linux binaries; clear CARGO_BUILD_TARGET. Cross-target launch is unsupported.")
    os.environ['CARGO_HOME'] = str(ROOT / '.cache/cargo')
    if command == 'build':
        args = ['cargo', 'build', '--locked']
        for package in PACKAGES:
            args.extend(['-p', 'project-x-' + package])
        run(args + ['--bins'])
        terminfo()
    elif command == 'terminfo':
        terminfo()
    elif command == 'daemon':
        if len(os.fsencode(SOCKET)) > 107:
            raise SystemExit('Default socket path exceeds Linux 107-byte limit in this nested checkout; use explicit namtabd paths with a short project-local endpoint.')
        # Never remove a preexisting socket/journal or chmod an existing directory.
        run([BIN / 'namtabd', '--socket', SOCKET, '--workspace', ROOT,
             '--worker', BIN / 'namtab-terminal-worker', '--pty-helper', BIN / 'namtab-pty-helper',
             '--terminfo', ROOT / '.artifacts/terminfo'])
    elif command == 'tui':
        run([BIN / 'namtab-tui', '--socket', SOCKET])
    elif command == 'agent':
        run([BIN / 'namtab-agent', '--socket', SOCKET])
    elif command == 'desktop':
        if not ELECTRON.is_file():
            raise SystemExit('Verified Electron 44.8.0 runtime missing; see docs/engineering/mk1-quickstart.md. No implicit download or sandbox bypass.')
        digest = hashlib.sha256()
        with ELECTRON.open('rb') as executable:
            for chunk in iter(lambda: executable.read(1_048_576), b''):
                digest.update(chunk)
        if digest.hexdigest() != ELECTRON_SHA:
            raise SystemExit('Electron executable hash differs from reviewed artifact; stop and inspect provenance.')
        run([ELECTRON, ROOT / 'apps/desktop/mk1', '--socket', SOCKET])


def terminfo():
    if not shutil.which('tic'):
        raise SystemExit('ncurses tic is required for the finite local TERM entry; no system installation attempted.')
    output = ROOT / '.artifacts/terminfo'
    output.mkdir(parents=True, exist_ok=True)
    run(['tic', '-x', '-o', output, ROOT / 'config/namtab-mk1.terminfo'])


if __name__ == '__main__':
    main()
