"""Bounded pinned-public-API diagnosis. PASS here characterizes a selection FAIL.
No adapter substitution, PTY, private mode access, byte-based mode inference or UI.
Use a fresh EA1_EVIDENCE_DIR for every invocation; reports are exclusive-create.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import struct
from evidence_dir import EVIDENCE, ROOT
from worker_test import MODS, key, worker

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
pin = json.loads((ROOT/'evidence/ea1/artifact-pin.json').read_text())
for path, digest in {pin['library']: pin['library_sha256'], pin['zig']: pin['zig_sha256'], **pin['headers']}.items():
    assert sha(ROOT/path) == digest, path
output = ROOT/'.artifacts/input-contract-probe'
compile_command = [pin['zig'], 'cc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-O1', '-g',
    '-I.artifacts/engine-assessment/ghostty/include', 'tests/ea1/input_contract.c',
    '-L.artifacts/engine-assessment/ghostty/lib', '-lghostty-vt',
    '-Wl,-rpath,'+str((ROOT/pin['library']).parent), '-o', str(output)]
subprocess.run(compile_command, cwd=ROOT, check=True, timeout=60)
native_records, worker_records, checks = [], [], []

def native(setup='', physical='Unknown', text='', cp=None, mods=0, consumed=0, action=1):
    cp = ord(text.lower()) if cp is None and text else cp or 0
    command = [str(output.relative_to(ROOT)), setup.encode().hex(), physical,
        text.encode().hex(), str(cp), str(mods), str(consumed), str(action)]
    result = json.loads(subprocess.check_output(command, cwd=ROOT, text=True, timeout=5))
    native_records.append({'argv': command, **result})
    return bytes.fromhex(result['encoded_hex'])

def expect(name, actual, expected):
    assert actual == expected, (name, actual.hex(), expected.hex())
    checks.append(name)

def character(c, bits):
    return {'kind': 'character', 'text': c, 'shift': bool(bits&1), 'ctrl': bool(bits&2), 'alt': bool(bits&4)}

def encode(w, intent, expected):
    actual = w.encode(intent)
    expect('worker matches public encoder', actual, expected)
    worker_records.append({'stamp': dict(w.stamp), 'intent': intent, 'actual_hex': actual.hex()})
    return actual

# Independent expected bytes: upstream fixterms contract versus retained product
# requirement. These assertions deliberately do NOT call the selection gate green.
aliases = [('i', 'I', b'\t'), ('m', 'M', b'\r'), ('[', 'Bracket', b'\x1b')]
legacy_failures = []
for c, physical, required in aliases:
    for setup, prefix in [('', 'fixterms'), ('\x1b[>1u', 'kitty'), ('\x1b[>4;2m', 'mok2')]:
        wanted = (f'\x1b[27;5;{ord(c)}~' if prefix == 'mok2' else f'\x1b[{ord(c)};5u').encode()
        actual = native(setup, physical, c, mods=2)
        expect(prefix+' Ctrl-'+c, actual, wanted)
        with worker() as w:
            w.feed(setup); encode(w, character(c, 2), wanted)
        if not setup:
            assert actual != required
            legacy_failures.append({'chord': 'Ctrl-'+c, 'required_hex': required.hex(), 'actual_hex': actual.hex(), 'verdict': 'FAIL'})
    expect('unidentified preserves logical text', native(physical='Unknown', text=c, mods=2), f'\x1b[{ord(c)};5u'.encode())
    expect('missing text loses legacy alias', native(physical=physical, cp=ord(c), mods=2), b'')
    expect('consumed Ctrl does not fix legacy', native(physical=physical, text=c, mods=2, consumed=2), f'\x1b[{ord(c)};5u'.encode())

# Proposed unconditional alias-key normalization is a counterexample, not policy:
# it changes Kitty Ctrl-I into Tab. No such event is constructed by the worker.
expect('unconditional remap legacy', native(physical='Tab'), b'\t')
expect('unconditional remap corrupts Kitty identity', native('\x1b[>1u', 'Tab'), b'\t')
expect('real Kitty Ctrl-I remains distinct', native('\x1b[>1u', 'I', 'i', mods=2), b'\x1b[105;5u')

# Explicit modifier oracles, distinct from the larger native/worker consistency matrix.
for setup, values in [('', [b'\x03', b'\x1bi', b'I', b'\x1bI', b'\x1b[105;6u', b'\x1b[105;7u']),
                      ('\x1b[>1u', [b'\x1b[99;5u', b'\x1b[105;3u', b'\x1b[105;2u', b'\x1b[105;4u', b'\x1b[105;6u', b'\x1b[105;7u']),
                      ('\x1b[>4;2m', [b'\x1b[27;5;99~', b'\x1b[27;3;105~', b'\x1b[27;2;73~', b'\x1b[27;4;73~', b'\x1b[27;6;73~', b'\x1b[27;7;105~'])]:
    for (c, physical, bits), wanted in zip([('c','C',2),('i','I',4),('I','I',1),('I','I',5),('I','I',3),('i','I',6)], values):
        expect('modifier oracle', native(setup, physical, c, mods=bits), wanted)

# The private Character DTO supplies no consumed-modifier facts. Do not confuse
# its unconsumed logical Shift with an OS text-producing key event or IME commit.
expect('Kitty unconsumed Shift-I', native('\x1b[>1u','I','I',mods=1), b'\x1b[105;2u')
expect('Kitty consumed Shift-I', native('\x1b[>1u','I','I',mods=1,consumed=1), b'I')

# Both synthetic human normalizers assert this same profile in Rust/JS tests;
# agents use these exact intents. This is not physical host/DOM event decoding.
profile = json.loads((ROOT/'tests/ea1/input-profile.json').read_text())
for setup in ['', '\x1b[>1u', '\x1b[>3u', '\x1b[>31u', '\x1b[>4;1m', '\x1b[>4;2m']:
    with worker() as w:
        w.feed(setup)
        for item in profile['keys']:
            for bits in range(8):
                intent = key(item['key'], shift=bool(bits&1), ctrl=bool(bits&2), alt=bool(bits&4))
                encode(w, intent, native(setup, item['key'], mods=bits))
        for c, physical in [('i','I'),('m','M'),('[','Bracket'),('c','C'),('界','Unknown'),('é','Unknown'),('\u0301','Unknown')]:
            for bits in ([0,1,2,3,4,5,6,7] if c.isascii() else [0,1]):
                encode(w, character(c, bits), native(setup, physical, c, mods=bits))
        for text in ['界e\u0301', 'é', '🙂']:
            # Text is currently encoded as scalar presses, NOT an IME commit API.
            expected = b''.join(native(setup, text=c, cp=ord(c)) for c in text)
            encode(w, {'kind':'text', 'text':text}, expected)
            if setup in ['', '\x1b[>1u', '\x1b[>3u', '\x1b[>4;1m', '\x1b[>4;2m']:
                expect('unmodified Unicode bytes', expected, text.encode())

# Modes are changed only by feeding the ONE canonical parser. Compare every
# split (including incomplete CSI), stack precedence/pop, MOK reset and RIS.
transition_cases = [
    ('', '\x1b[>1u'), ('\x1b[>1u', '\x1b[>3u'),
    ('\x1b[>1u\x1b[>3u', '\x1b[<u'), ('\x1b[>1u', '\x1b[<u'),
    ('', '\x1b[>4;2m'), ('\x1b[>4;2m', '\x1b[>1u'),
    ('\x1b[>4;2m\x1b[>1u', '\x1b[<u'), ('\x1b[>4;2m', '\x1b[>4;0m'),
    ('\x1b[>4;2m\x1b[>3u', '\x1bc')]
for before, transition in transition_cases:
    for split in range(len(transition)+1):
        with worker() as w:
            w.feed(before); w.feed(transition[:split])
            encode(w, character('i',2), native(before+transition[:split], 'I', 'i', mods=2))
            w.feed(transition[split:])
            encode(w, character('i',2), native(before+transition, 'I', 'i', mods=2))
expect('MOK pop after Kitty', native('\x1b[>4;2m\x1b[>1u\x1b[<u','I','i',mods=2), b'\x1b[27;5;105~')
expect('MOK disabled', native('\x1b[>4;2m\x1b[>4;0m','I','i',mods=2), b'\x1b[105;5u')
expect('RIS reset', native('\x1b[>4;2m\x1b[>3u\x1bc','I','i',mods=2), b'\x1b[105;5u')

for setup, action, wanted in [('',0,b''),('',2,b'\x1b[105;5u'),('\x1b[>1u',0,b''),
        ('\x1b[>3u',0,b'\x1b[105;5:3u'),('\x1b[>3u',2,b'\x1b[105;5:2u')]:
    expect('native release/repeat boundary', native(setup,'I','i',mods=2,action=action), wanted)
for action in ['release','repeat','unknown']:
    with worker() as w:
        # The strict decoder cannot trust a malformed request's sequence/stamp;
        # its uncorrelated response uses seq=0. Do not use the valid-call helper.
        request = {'version':1,'seq':2,'expected':w.stamp,'operation':{
            'op':'encode','input':{**key('Tab'),'action':action}}}
        payload = json.dumps(request).encode()
        w.p.stdin.write(struct.pack('>I',len(payload))+payload);w.p.stdin.flush()
        r = w.read()
        assert r['seq'] == 0 and r['error'] == 'malformed' and not r['input_bytes']
        checks.append('worker rejects unavailable action')
for intent in [character('界',2), character('é',4), character('\t',0)]:
    with worker() as w:
        r = w.call({'op':'encode','input':intent})
        assert r['error'] == 'unsupported' and not r['input_bytes']
        checks.append('worker rejects unsupported character')
with worker() as w:
    # Unknown terminal state after bounded reply overflow must never encode.
    assert w.feed('\x1b[6n'*800)['status'] == 'unknown'
    assert w.call({'op':'encode','input':key('Tab')})['error'] == 'unknown'
    checks.append('unknown state refuses input')

for destination in ['evidence/ea1', 'evidence', '../outside-project']:
    env = {**os.environ, 'PYTHONPATH':str(ROOT/'tests/ea1'), 'EA1_EVIDENCE_DIR':destination}
    r = subprocess.run(['python3','-B','-c','import evidence_dir'],cwd=ROOT,env=env,capture_output=True,text=True,timeout=5)
    assert r.returncode != 0 and 'ValueError' in r.stderr
    checks.append('unsafe or historical evidence destination refused')

baseline = json.loads((ROOT/'evidence/ea1-input-investigation/baseline.json').read_text())
preserved = {}
for name, digest in baseline['files'].items():
    if (name.startswith(('evidence/', 'crates/engine/', 'crates/protocol/')) and name != 'evidence/README.md') or name in [
        'Cargo.lock','pnpm-lock.yaml','tests/system/engine-probe.mjs',
        'tests/fixtures/terminal/continuations.json','tests/fixtures/manifest.json']:
        assert sha(ROOT/name) == digest, name
        preserved[name] = digest
assert subprocess.check_output(['git','rev-list','--reverse',baseline['base_commit']],cwd=ROOT,text=True).splitlines() == baseline['prior_commits']
report = {'diagnostic_checks':'PASS', 'selection_gate':'FAIL', 'correction':'BLOCKED_PUBLIC_SEAM',
    'legacy_failures':legacy_failures, 'compile_command':compile_command,
    'probe_source_sha256':sha(ROOT/'tests/ea1/input_contract.c'), 'probe_binary_sha256':sha(output),
    'native_cases':native_records, 'worker_cases':worker_records, 'assertions':len(checks),
    'shift_consumption_gap':{'kitty_unconsumed_shift_i_hex':'1b5b3130353b3275','kitty_consumed_shift_i_hex':'49','worker_supplies_consumed_modifiers':False,'status':'Native keyboard/layout/IME translation remains unimplemented; do not certify this private logical-character profile as human event normalization.'},
    'preserved_sha256':preserved, 'prior_commits':baseline['prior_commits'],
    'limits':'Synthetic press profile and public API probes only. Native release/repeat is not exposed by the worker. Scalar Text encoding is not complete IME/layout support. No PTY, frontend, performance, rendering, a11y or recovery qualification. Matching the same native core is consistency evidence; hand-authored byte oracles are separate.'}
with (EVIDENCE/'input-contract.json').open('x') as out:
    json.dump(report,out,indent=2);out.write('\n')
print(f'Diagnosis PASS ({len(native_records)} native cases, {len(worker_records)} worker encodings, {len(checks)} assertions).')
print(f'Preservation PASS ({len(preserved)} paths, five existing commits). EA1 selection FAIL (three legacy aliases); correction BLOCKED_PUBLIC_SEAM.')
