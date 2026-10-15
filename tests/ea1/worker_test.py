"""EA1 synthetic subprocess proof. No PTY, shell, network, capture file or frontend.
All expected bytes below are independent protocol expectations, not copied engine output.
"""
from contextlib import contextmanager
import copy
import hashlib
import json
import os
from pathlib import Path
import select
import struct
import subprocess
import time
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[2]
BINARY = ROOT / 'target/debug/project-x-engine'
EVIDENCE = ROOT / 'evidence/ea1'
MODS = {'shift': False, 'alt': False, 'ctrl': False}

def key(name, **mods):
    return {'kind': 'key', 'key': name, **MODS, **mods}

class Worker:
    def __init__(self, cols=12, rows=5):
        self.p = subprocess.Popen([BINARY], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, cwd=ROOT)
        self.seq = 0
        self.stamp = None
        r = self.call({'op': 'init', 'session': 'fixture', 'incarnation': 'ea1-'+uuid.uuid4().hex, 'cols': cols, 'rows': rows})
        assert r['status'] == 'ok', r
    def exact(self, n):
        out = b''
        deadline = time.monotonic() + 5
        while len(out) < n:
            ready, _, _ = select.select([self.p.stdout], [], [], max(0, deadline-time.monotonic()))
            if not ready:
                raise TimeoutError('worker response timeout')
            chunk = os.read(self.p.stdout.fileno(), n-len(out))
            if not chunk:
                raise EOFError('worker closed')
            out += chunk
        return out
    def read(self):
        n = struct.unpack('>I', self.exact(4))[0]
        assert 0 < n <= 2097152
        return json.loads(self.exact(n))
    def call(self, operation, expected='current', seq=None, version=1):
        if seq is None:
            self.seq += 1
            seq = self.seq
        obj = {'version': version, 'seq': seq, 'expected': self.stamp if expected == 'current' else expected, 'operation': operation}
        payload = json.dumps(obj, ensure_ascii=False).encode()
        self.p.stdin.write(struct.pack('>I', len(payload)) + payload)
        self.p.stdin.flush()
        out = self.read()
        assert out['seq'] == seq
        self.stamp = out['stamp']
        return out
    def feed(self, data):
        if isinstance(data, str): data = data.encode()
        return self.call({'op': 'feed', 'bytes': list(data)})
    def observe(self):
        r = self.call({'op': 'observe'})
        assert r['status'] == 'ok', r
        return r['frame']
    def encode(self, intent):
        r = self.call({'op': 'encode', 'input': intent})
        assert r['status'] == 'ok', r
        assert r['reply'] is None, 'application input generated a reply lane'
        return bytes(r['input_bytes'])
    def close(self):
        if self.p.poll() is None:
            self.p.stdin.close()
            try: self.p.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.p.kill(); self.p.wait(timeout=2)
        for stream in [self.p.stdin, self.p.stdout, self.p.stderr]: stream.close()

@contextmanager
def worker(cols=12, rows=5):
    w = Worker(cols, rows)
    try: yield w
    finally: w.close()

def lines(frame):
    c = frame['cols']
    return [''.join(x['text'] or (' ' if x['wide'] == 0 else '') for x in frame['cells'][y*c:(y+1)*c]).rstrip() for y in range(frame['rows'])]

class Tests(unittest.TestCase):
    def test_original_continuations_and_all_prefix_splits(self):
        fixtures = json.loads((ROOT/'tests/fixtures/terminal/continuations.json').read_text())
        expected = json.loads((ROOT/'tests/assessment/expectations.json').read_text())['cases']
        trials = []
        for f in fixtures:
            before, after = f['before'].encode(), f['after'].encode()
            for split in range(len(before)+1):
                with worker() as w:
                    self.assertEqual(w.feed(before[:split])['status'], 'ok')
                    w.observe()  # includes partial-parser cuts; no alternate parser involved
                    self.assertEqual(w.feed(before[split:])['status'], 'ok')
                    attached = w.observe()
                    attached_copy = copy.deepcopy(attached)
                    w.feed(after)
                    result = w.observe()
                    self.assertEqual(lines(result), expected[f['name']]['lines'])
                    self.assertEqual(result['cursor'], expected[f['name']]['cursor'])
                    self.assertEqual(result['alternate'], expected[f['name']]['buffer'] == 'alternate')
                    if f['name'] == 'partial-csi': self.assertEqual(result['cells'][0]['fg_palette'], 1)
                    self.assertEqual(attached, attached_copy, 'old frame mutated')
                    trials.append({'case': f['name'], 'split': split, 'cells_sha256': hashlib.sha256(json.dumps(result['cells'], sort_keys=True).encode()).hexdigest()})
        EVIDENCE.joinpath('continuations.json').write_text(json.dumps({'verdict':'PASS', 'trials':trials}, indent=2)+'\n')

    def test_unicode_styles_resize_alternate_and_attach(self):
        with worker() as w:
            w.feed(b'\xe7\x95'); self.assertFalse(w.observe()['ground'])
            w.feed(b'\x8c'); f = w.observe()
            self.assertEqual(f['cells'][0]['text'], '界'); self.assertEqual(f['cells'][0]['wide'], 1)
            self.assertEqual(f['cells'][1]['wide'], 2)
            w.feed('\r\n\x1b[1;3;4;38;2;10;20;30;48;2;40;50;60me\u0301')
            f = w.observe(); c = f['cells'][12]
            self.assertEqual(c['text'], 'e\u0301'); self.assertEqual(c['flags'] & 3, 3)
            self.assertEqual(c['underline'], 1); self.assertEqual(c['fg'], 0x0a141e); self.assertEqual(c['bg'], 0x28323c)
            w.feed('\x1b[?1049hALT'); self.assertTrue(w.observe()['alternate'])
            w.feed('\x1b[?1049l'); self.assertFalse(w.observe()['alternate'])
            self.assertEqual(w.call({'op':'resize','cols':9,'rows':4})['status'], 'ok')
            first = w.observe(); second = w.observe()
            self.assertEqual((second['cols'],second['rows']), (9,4))
            first.pop('dirty'); second.pop('dirty'); self.assertEqual(first, second)
            self.assertEqual(w.stamp['geometry'], 1)
            # No transcript/snapshot persistence or reconstruction is requested for late reads.
            w.feed('\x1b[?25l\x1b[5 q\x1b[?2026h'); f=w.observe()
            self.assertFalse(f['cursor_visible']); self.assertEqual(f['cursor_style'], 0)
            self.assertTrue(f['modes'] & 32, 'must expose render hold, not claim held paint')
            w.feed('\x1b[?2026l'); self.assertFalse(w.observe()['modes'] & 32)

    def test_named_key_families_modes_and_modifiers(self):
        normals = {'Up':b'\x1b[A','Down':b'\x1b[B','Right':b'\x1b[C','Left':b'\x1b[D','Home':b'\x1b[H','End':b'\x1b[F','PageUp':b'\x1b[5~','PageDown':b'\x1b[6~','Insert':b'\x1b[2~','Delete':b'\x1b[3~','Tab':b'\t','Enter':b'\r','Escape':b'\x1b','Backspace':b'\x7f'}
        normals.update({f'F{i}': x for i,x in enumerate([b'\x1bOP',b'\x1bOQ',b'\x1bOR',b'\x1bOS',b'\x1b[15~',b'\x1b[17~',b'\x1b[18~',b'\x1b[19~',b'\x1b[20~',b'\x1b[21~',b'\x1b[23~',b'\x1b[24~'],1)})
        normal_pad={**{f'Keypad{i}':str(i).encode() for i in range(10)},'KeypadEnter':b'\r','KeypadAdd':b'+','KeypadSubtract':b'-','KeypadMultiply':b'*','KeypadDivide':b'/','KeypadDecimal':b'.'}
        results={}
        with worker() as w:
            for name, expected in {**normals,**normal_pad}.items():
                actual=w.encode(key(name));self.assertEqual(actual,expected,name);results[name]=actual.hex()
            self.assertEqual(w.encode(key('Tab',shift=True)),b'\x1b[Z')
            self.assertEqual(w.encode(key('Up',ctrl=True)),b'\x1b[1;5A')
            self.assertEqual(w.encode(key('Up',shift=True,alt=True,ctrl=True)),b'\x1b[1;8A')
            w.feed('\x1b[?1h\x1b=')
            for name, code in [('Up','A'),('Down','B'),('Right','C'),('Left','D'),('Home','H'),('End','F')]: self.assertEqual(w.encode(key(name)), ('\x1bO'+code).encode())
            self.assertEqual(w.encode(key('KeypadEnter')),b'\r','mode 1035 remains numeric by default')
            w.feed('\x1b[?1035l')
            app_pad={**{f'Keypad{i}':('\x1bO'+chr(ord('p')+i)).encode() for i in range(10)},'KeypadEnter':b'\x1bOM','KeypadAdd':b'\x1bOk','KeypadSubtract':b'\x1bOm','KeypadMultiply':b'\x1bOj','KeypadDivide':b'\x1bOo','KeypadDecimal':b'\x1bOn'}
            for name, expected in app_pad.items():self.assertEqual(w.encode(key(name)),expected,name)
            w.feed('\x1b[?1l\x1b>');self.assertEqual(w.encode(key('Up')),b'\x1b[A')
            w.feed('\x1b[2h');self.assertEqual(w.call({'op':'encode','input':key('Up')})['error'],'unsupported')
        EVIDENCE.joinpath('keys.json').write_text(json.dumps({'verdict':'PASS','normal_key_bytes':results,'application_keypad_cases':len(app_pad),'notes':'No physical keyboard/layout/IME or opaque application was exercised.'},indent=2)+'\n')

    def test_character_commits_enhanced_protocol_and_paste_focus_mouse(self):
        with worker() as w:
            character=lambda c,**mods:{'kind':'character','text':c,**MODS,**mods}
            self.assertEqual(w.encode(character('c',ctrl=True)),b'\x03')
            self.assertEqual(w.encode(character('x',alt=True)),b'\x1bx')
            self.assertEqual(w.encode({'kind':'text','text':'界e\u0301'}),'界e\u0301'.encode())
            w.feed('\x1b[>1u')  # Kitty disambiguation: logical Ctrl-C, no physical layout inference.
            self.assertEqual(w.encode(character('c',ctrl=True)),b'\x1b[99;5u')
            w.feed('\x1b[<u\x1b[>4;2m')  # xterm modifyOtherKeys level 2
            self.assertEqual(w.encode(character('c',ctrl=True)),b'\x1b[27;5;99~')
            w.feed('\x1b[>4;0m')
            self.assertEqual(w.encode({'kind':'paste','text':'a\nb'}),b'a\rb')
            w.feed('\x1b[?2004h');self.assertEqual(w.encode({'kind':'paste','text':'a\nb'}),b'\x1b[200~a\nb\x1b[201~')
            self.assertEqual(w.call({'op':'encode','input':{'kind':'paste','text':'bad\x1b[201~'}})['error'],'unsupported')
            self.assertEqual(w.encode({'kind':'focus','gained':True}),b'')
            w.feed('\x1b[?1004h');self.assertEqual(w.encode({'kind':'focus','gained':True}),b'\x1b[I');self.assertEqual(w.encode({'kind':'focus','gained':False}),b'\x1b[O')
            mouse={'kind':'mouse','x':2,'y':1,'button':1,'release':False,**MODS}
            self.assertEqual(w.encode(mouse),b'')
            w.feed('\x1b[?1000h\x1b[?1006h');self.assertEqual(w.encode(mouse),b'\x1b[<0;3;2M')
            self.assertEqual(w.encode({**mouse,'release':True}),b'\x1b[<0;3;2m')
            self.assertEqual(w.call({'op':'encode','input':{**mouse,'x':12}})['error'],'unsupported')

    def test_shared_frontend_intents_use_one_encoder(self):
        fixtures=json.loads((ROOT/'.artifacts/ea1/shared-inputs.json').read_text())
        results=[]
        for origin,intents in fixtures.items():
            with worker() as w:
                for application in [False,True]:
                    if application:w.feed('\x1b[?1h')
                    expected=[b'\x1bOA' if application else b'\x1b[A',b'\x1b[Z',b'\x1b[24;5~']
                    for intent,wanted in zip(intents,expected):
                        actual=w.encode(intent);self.assertEqual(actual,wanted)
                        results.append({'origin':origin,'application_cursor':application,'intent':intent,'bytes_hex':actual.hex()})
        EVIDENCE.joinpath('shared-inputs.json').write_text(json.dumps({'verdict':'PASS','results':results,'limits':'Synthetic normalization and one worker contract; no actual frontend connection, focus, screen reader, keyboard or application.'},indent=2)+'\n')

    def test_required_legacy_control_aliases(self):
        # Selection gate, intentionally NOT relaxed to match upstream fixterms behavior.
        results=[]
        with worker() as w:
            for char, expected in [('i',b'\t'),('m',b'\r'),('[',b'\x1b')]:
                actual=w.encode({'kind':'character','text':char,**MODS,'ctrl':True})
                results.append({'chord':'Ctrl-'+char,'expected_hex':expected.hex(),'actual_hex':actual.hex(),'verdict':'PASS' if actual==expected else 'FAIL'})
            EVIDENCE.joinpath('legacy-control-aliases.json').write_text(json.dumps({'verdict':'FAIL' if any(x['verdict']=='FAIL' for x in results) else 'PASS','cases':results,'requirements':'Legacy control aliases for existing Linux shells/editor navigation, no Kitty/MOK negotiated. Named Tab/Enter/Escape are distinct intents, not a fix.'},indent=2)+'\n')
            for item in results:
                with self.subTest(chord=item['chord']):self.assertEqual(item['actual_hex'],item['expected_hex'])

    def test_single_reply_owner_exact_stream_no_retries(self):
        with worker() as w:
            r=w.feed('\x1b[2;3H\x1b[6n\x1b[6n')
            self.assertEqual(bytes(r['reply']['bytes']),b'\x1b[2;3R\x1b[2;3R')
            self.assertEqual(r['reply']['origin'],'terminal_reply');self.assertEqual(r['reply']['work'],r['seq'])
            old_stamp=copy.deepcopy(w.stamp);seq=w.seq
            r=w.call({'op':'feed','bytes':list(b'\x1b[6n')},seq=seq)
            self.assertEqual(r['error'],'sequence');self.assertIsNone(r['reply'])
            self.assertEqual(w.stamp,old_stamp)
            for _ in range(3):self.assertIsNone(w.call({'op':'observe'})['reply'])
            r=w.feed('\x1b[');self.assertIsNone(r['reply']);r=w.feed('6n')
            self.assertEqual(bytes(r['reply']['bytes']),b'\x1b[2;3R')
            self.assertEqual(w.encode(key('Up')),b'\x1b[A')
            # Query reports are data returned to the parent; no PTY dispatch has occurred.
            r=w.feed('\x1b[5n\x1b[?1$p')
            self.assertEqual(bytes(r['reply']['bytes']),b'\x1b[0n\x1b[?1;2$y')

    def test_stale_identity_resize_and_shutdown(self):
        with worker() as w:
            stale=copy.deepcopy(w.stamp);w.feed('X')
            self.assertEqual(w.call({'op':'encode','input':key('Enter')},expected=stale)['error'],'stale')
            wrong={**w.stamp,'session':'wrong'}
            self.assertEqual(w.call({'op':'encode','input':key('Up')},expected=wrong)['error'],'stale')
            stale=copy.deepcopy(w.stamp);w.call({'op':'resize','cols':9,'rows':4})
            self.assertEqual(w.call({'op':'encode','input':key('Up')},expected=stale)['error'],'stale')
            self.assertEqual(w.call({'op':'shutdown'},expected=stale)['error'],'stale')
            self.assertEqual(w.observe()['cols'],9,'rejected shutdown must not kill owner')
            self.assertEqual(w.call({'op':'shutdown'})['status'],'ok');self.assertEqual(w.p.wait(timeout=2),0)
        with worker() as fresh:
            self.assertEqual(fresh.call({'op':'observe'},expected=stale)['error'],'stale')
        with worker() as first:
            initial=copy.deepcopy(first.stamp)  # revision/geometry both zero
        with worker() as second:
            self.assertEqual(second.stamp['revision'],initial['revision'])
            self.assertNotEqual(second.stamp['incarnation'],initial['incarnation'])
            self.assertEqual(second.call({'op':'encode','input':key('Enter')},expected=initial)['error'],'stale')

    def test_reply_overflow_latches_unknown_and_no_partial_bytes(self):
        with worker() as w:
            r=w.feed(b'\x1b[6n'*800)
            self.assertEqual(r['status'],'unknown');self.assertIsNone(r['reply']);self.assertIsNone(r['input_bytes'])
            self.assertEqual(w.call({'op':'encode','input':key('Up')})['error'],'unknown')
            self.assertEqual(w.call({'op':'observe'})['error'],'unknown')

    def test_output_flood_and_resource_refusals(self):
        with worker(80,24) as w:
            self.assertEqual(w.feed(b'x'*8193)['error'],'limit')
            self.assertEqual(w.call({'op':'resize','cols':240,'rows':100})['error'],'limit')
            for _ in range(24):self.assertEqual(w.feed(b'0123456789\r\n'*600)['status'],'ok')
            f=w.observe();self.assertEqual(len(f['cells']),1920)
            self.assertEqual(w.encode(key('Enter')),b'\r')
        with worker() as w:
            # Refuse unrepresentable clusters, never claim silently truncated observations.
            w.feed('e'+'\u0301'*100)
            r=w.call({'op':'observe'});self.assertEqual(r['status'],'unknown');self.assertIsNone(r['frame'])

    def test_malformed_protocol_and_truncated_transport(self):
        for payload in [b'{"version":1,"version":1}',b'{"version":1,"seq":2,"expected":null,"operation":{"op":"observe","surprise":true}}',b'{"version":1,"seq":2,"expected":null,"operation":{"op":"encode","input":{"kind":"character","text":"\\ud800","shift":false,"alt":false,"ctrl":false}}}']:
            with worker() as w:
                w.p.stdin.write(struct.pack('>I',len(payload))+payload);w.p.stdin.flush()
                r=w.read();self.assertEqual(r['error'],'malformed');self.assertEqual(w.p.wait(timeout=2),0)
        for prefix in [b'\x00\x00',struct.pack('>I',65537),struct.pack('>I',2)+b'{']:
            p=subprocess.Popen([BINARY],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            out,err=p.communicate(prefix,timeout=2)
            self.assertEqual(p.returncode,2);self.assertEqual(out,b'');self.assertNotIn(b'{',err)

    def test_slow_consumer_preserves_frame_boundaries(self):
        with worker(80,24) as w:
            data=json.dumps({'version':1,'seq':2,'expected':w.stamp,'operation':{'op':'observe'}}).encode()
            w.p.stdin.write(struct.pack('>I',len(data))+data);w.p.stdin.flush()
            # Drain the >pipe-sized frame in deliberately small reads; no second queue/parser.
            n=struct.unpack('>I',w.exact(4))[0];parts=[]
            while n:
                count=min(n,113);parts.append(w.exact(count));n-=count
            frame=json.loads(b''.join(parts))
            self.assertEqual(len(frame['frame']['cells']),1920)
            w.seq=2;self.assertEqual(w.feed('after')['status'],'ok')
            self.assertEqual(lines(w.observe())[0],'after')

    def test_shutdown_eof_broken_consumer_and_worker_loss(self):
        for _ in range(20):
            with worker() as w:
                w.feed('live');self.assertEqual(lines(w.observe())[0],'live')
                w.p.stdin.close();self.assertEqual(w.p.wait(timeout=2),0)
        with worker() as w:
            w.p.stdout.close()
            data=json.dumps({'version':1,'seq':2,'expected':w.stamp,'operation':{'op':'observe'}}).encode()
            w.p.stdin.write(struct.pack('>I',len(data))+data);w.p.stdin.flush()
            self.assertEqual(w.p.wait(timeout=2),2)
        with worker() as w:
            w.feed('not durable');w.p.kill();self.assertLess(w.p.wait(timeout=2),0)
        with worker() as fresh:self.assertEqual(lines(fresh.observe()),['']*5)

if __name__ == '__main__':
    unittest.main(verbosity=2)
