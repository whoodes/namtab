#!/usr/bin/env python3
"""Real fixed-process workflow + independently checked bounded report. No user attach."""
import argparse, copy, hashlib, json, math, os, platform, subprocess
from pathlib import Path


def sha(b):
    return hashlib.sha256(b).hexdigest()


def audit(r, takeover):
    p = r['provenance']
    assert p['version'] == 'namtab.flow/0' and p['profile'] == 'h0-flow-once/0'
    assert p['environment'] == [['PATH', '/usr/bin:/bin'], ['LC_ALL', 'C']]
    assert p['producer_argv'] == ['noisy' if takeover else 'producer']
    assert p['consumer_format'] == 'consumed:%s\n'
    assert p['expected_artifact_sha256'] == sha(b'namtab-artifact-v1')
    assert r['context'] == sha(json.dumps(p, ensure_ascii=False, separators=(',', ':')).encode())
    for f in ['fixture_sha256', 'consumer_sha256', 'request_schema_sha256', 'response_schema_sha256']:
        assert len(p[f]) == 64 and all(c in '0123456789abcdef' for c in p[f])
    rs = r['receipts']
    assert len(rs) == (1 if takeover else 2)
    epoch = rs[0]['origin']['epoch']
    assert len(epoch) == 32 and all(c in '0123456789abcdef' for c in epoch)
    names = ['producer', 'consumer']

    def binding(run, control):
        i = names.index(run)
        return dict(epoch=epoch, context=r['context'], run=run, task=f'task-{i}',
                    attempt=f'attempt-{i}-1', connection=1, control=control, profile='h0-flow-once/0')

    expected_streams = {
        ('producer', 'stdout'): b'namtab-artifact-v1',
        ('producer', 'stderr'): b'N' * 2048 if takeover else b'producer-diagnostic',
        ('consumer', 'stdout'): b'' if takeover else b'consumed:namtab-artifact-v1\n',
        ('consumer', 'stderr'): b'',
    }
    for i, receipt in enumerate(rs):
        run = names[i]
        assert receipt['origin'] == binding(run, 1)
        assert receipt['sequence'] == 1 and receipt['calls'] == 1 and receipt['state'] == 'completed'
        assert receipt['exit_code'] == 0 and receipt['drained'] is True
        assert receipt['stdout_bytes'] == len(expected_streams[run, 'stdout'])
        assert receipt['stderr_bytes'] == len(expected_streams[run, 'stderr'])
        assert receipt['stdout_sha256'] == sha(expected_streams[run, 'stdout'])
    assert rs[0]['dependency'] is None
    if takeover:
        assert r['artifact'] is None and r['takeover_during_running_output'] is True
        assert isinstance(r['takeover_us'], int) and r['takeover_us'] >= 0
    else:
        artifact = r['artifact']
        assert artifact == rs[1]['dependency']
        assert artifact == dict(epoch=epoch, context=r['context'], producer_attempt='attempt-0-1',
                                revision=1, sha256=sha(b'namtab-artifact-v1'), bytes=18)
        assert r['takeover_us'] is None and r['takeover_during_running_output'] is False
    assert r['takeover_ack'] == dict(kind='taken', control=2, running=1 if takeover else 0)
    assert r['output_counts'] == [[len(expected_streams[run, stream]) for stream in ['stdout', 'stderr']] for run in names]

    # Authoritative final human pagination supplies the entire bounded event cut.
    pages = r['final_human']
    assert pages and pages[-1]['more'] is False
    events = []
    previous = 0
    for i, page in enumerate(pages):
        assert len(page['events']) <= 16 and page['cursor'] <= 128
        assert page['cursor'] >= previous and (page['cursor'] > previous or not page['more'])
        assert page['more'] is (i < len(pages) - 1)
        assert page['cursor'] == (page['events'][-1]['id'] if page['events'] else previous)
        events.extend(page['events'])
        previous = page['cursor']
    assert [e['id'] for e in events] == list(range(1, previous + 1))
    assert len(events) <= 128
    offsets = {key: 0 for key in expected_streams}
    for e in events:
        origin = e['origin']
        if origin is not None:
            assert origin == binding(origin['run'], 1)
        if e['stream'] is None:
            assert e['bytes'] == 0 and e['offset'] == 0 and e['sha256'] is None
        else:
            assert e['kind'] == 'output' and origin is not None
            key = (origin['run'], e['stream'])
            n = e['bytes']
            assert 0 < n <= 256 and e['offset'] == offsets[key]
            expected = expected_streams[key][e['offset']:e['offset'] + n]
            assert len(expected) == n and e['sha256'] == sha(expected)
            offsets[key] += n
    assert offsets == {key: len(value) for key, value in expected_streams.items()}
    for run in names[:len(rs)]:
        for kind in ['queued', 'launch_boundary', 'result']:
            assert sum(e['kind'] == kind and e['origin'] == binding(run, 1) for e in events) == 1
    assert sum(e['kind'] == 'artifact_published' for e in events) == (0 if takeover else 1)
    assert sum(e['kind'] == 'human_takeover_future_effects_fenced' for e in events) == 1

    def observation(v, scope, after=0, final=False):
        assert 0 <= after <= v['cursor'] <= previous <= 128
        assert v['control'] in [1, 2] and v['human'] is (v['control'] == 2)
        assert v['binding'] == (None if scope is None else binding(scope, v['control']))
        assert isinstance(v['admission_known'], bool) and len(v['events']) <= 16
        visible = [e for e in events if after < e['id'] <= v['cursor'] and
                   (scope is None or e['origin'] is None or e['origin']['run'] == scope)]
        assert v['events'] == visible
        if v['more']:
            assert len(v['events']) == 16 and v['cursor'] > after
        for receipt in v['receipts']:
            run = receipt['origin']['run']
            assert scope is None or run == scope
            assert receipt['origin'] == binding(run, 1) and receipt['sequence'] == 1
        if final:
            assert v['human'] and v['control'] == 2 and v['admission_known'] is True
            assert v['artifact'] == r['artifact']
            assert v['receipts'] == [receipt for receipt in rs if scope is None or receipt['origin']['run'] == scope]

    for page in pages:
        observation(page, None, after=page['events'][0]['id']-1 if page['events'] else 0, final=True)
    consumer_pages = r['final_consumer']
    assert consumer_pages and consumer_pages[-1]['more'] is False
    after = 0
    for i, page in enumerate(consumer_pages):
        observation(page, 'consumer', after, final=True)
        assert page['more'] is (i < len(consumer_pages) - 1)
        after = page['cursor']
    assert after == previous
    assert [e for page in consumer_pages for e in page['events']] == [e for e in events if e['origin'] is None or e['origin']['run'] == 'consumer']
    consumer = r['updates'][1 if takeover else 0]
    observation(consumer, 'consumer')
    if takeover:
        witness = r['updates'][0]
        observation(witness, None)
        assert not witness['human'] and witness['control'] == 1
        assert any(receipt['state'] == 'running' and receipt['calls'] == 1 and receipt['origin'] == binding('producer', 1) for receipt in witness['receipts'])
        assert any(e['kind'] == 'output' and e['origin'] == binding('producer', 1) and e['bytes'] > 0 for e in witness['events'])
    else:
        observation(r['updates'][-1], None, final=True)
    return True


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--bin-dir',required=True,type=Path);parser.add_argument('--artifacts',required=True,type=Path);args=parser.parse_args()
    root=args.artifacts.resolve();root.mkdir(mode=0o700)  # Never overwrite a prior run.
    bindir=args.bin_dir.resolve();samples=[];reports=[]
    for i in range(20):
        takeover=i>=4;dest=root/f'run-{i:02d}'
        cmd=[str(bindir/'namtab-flow-probe'),str(dest),str(bindir/'namtab-flow-fixture')]+(['takeover'] if takeover else [])
        result=subprocess.run(cmd,capture_output=True,timeout=12,check=False)
        (root/f'command-{i:02d}.json').write_text(json.dumps({'command':cmd,'exit':result.returncode,'stderr':result.stderr.decode(errors='replace')},indent=2)+'\n')
        assert result.returncode==0,result.stderr.decode(errors='replace')
        report=json.loads(result.stdout);assert report==json.loads((dest/'report.json').read_text());audit(report,takeover)
        assert report['provenance']['fixture_sha256']==sha((bindir/'namtab-flow-fixture').read_bytes())
        assert report['provenance']['consumer_sha256']==sha(Path('/usr/bin/printf').read_bytes())
        for direction in ['request','response']:
            assert report['provenance'][f'{direction}_schema_sha256']==sha(Path(f'schemas/experimental/flow-{direction}.schema.json').read_bytes())
        reports.append(report)
        if takeover:samples.append(report['takeover_us'])
    for mode in ['nonzero','overflow']:
        cmd=[str(bindir/'namtab-flow-probe'),str(root/mode),str(bindir/'namtab-flow-fixture'),mode]
        result=subprocess.run(cmd,capture_output=True,timeout=12,check=False)
        (root/f'command-{mode}.json').write_text(json.dumps({'command':cmd,'exit':result.returncode,'stderr':result.stderr.decode(errors='replace')},indent=2)+'\n')
        assert result.returncode==0,result.stderr.decode(errors='replace')
        r=json.loads(result.stdout);receipt=r['receipts'][0]
        assert r['artifact'] is None and receipt['state']=='failed' and receipt['calls']==1
        assert r['updates'][-1]['human'] is True
        if mode=='nonzero':assert receipt['exit_code']==3 and receipt['drained'] is True
        else:assert receipt['drained'] is False and r['updates'][-1]['admission_known'] is False
    # Counterexample sensitivity: do not trust the probe's own checks[] labels.
    normal=reports[0];mutations=[
        lambda r:r['receipts'][1]['origin'].update(run='producer'),
        lambda r:r['receipts'][1].update(calls=2),
        lambda r:r['receipts'][1]['dependency'].update(revision=2),
        lambda r:r['artifact'].update(sha256='0'*64),
        lambda r:r['receipts'][0].update(drained=False),
        lambda r:r['updates'][0]['events'].append({'origin':r['receipts'][0]['origin']}),
        lambda r:r['receipts'][1].update(stdout_sha256='0'*64),
        lambda r:r['receipts'][1]['origin'].update(epoch='0'*32),
        lambda r:r['updates'][0]['receipts'].append(copy.deepcopy(r['receipts'][0])),
        lambda r:r['updates'][0]['binding'].update(context='0'*64),
        lambda r:r['updates'][0].update(cursor=65535,more=False),
        lambda r:r['updates'][0].update(events=[]),
        lambda r:r['receipts'][1].update(stdout_bytes=0,stderr_bytes=4096),
        lambda r:r['final_human'][0]['receipts'][0]['origin'].update(control=2),
        lambda r:r['final_human'][0]['events'][2].update(sha256='0'*64),
        lambda r:r['final_consumer'][0].update(events=[]),
    ]
    for mutate in mutations:
        bad=copy.deepcopy(normal);mutate(bad)
        try:audit(bad,False)
        except (AssertionError,KeyError):pass
        else:raise AssertionError('corruption escaped independent report audit')
    takeover_mutations=[lambda r:r.update(artifact=normal['artifact']),lambda r:r.update(takeover_during_running_output=False),lambda r:r['updates'][0].update(receipts=[],events=[]),lambda r:r.update(updates=[None,r['updates'][1]]),lambda r:r['takeover_ack'].update(running=0),lambda r:r['final_human'][0]['receipts'][0]['origin'].update(control=2)]
    for mutate in takeover_mutations:
        bad=copy.deepcopy(reports[-1]);mutate(bad)
        try:audit(bad,True)
        except (AssertionError,KeyError,TypeError):pass
        else:raise AssertionError('takeover corruption escaped report audit')
    samples.sort()
    percentile=lambda p:samples[max(0,math.ceil(len(samples)*p)-1)]
    summary={'status':'PASS','normal_invocations':4,'takeover_invocations':16,'fault_invocations':2,'negative_report_mutations_rejected':len(mutations)+len(takeover_mutations),'takeover_microseconds':{'samples':samples,'p50':percentile(.5),'p95':percentile(.95),'p99':percentile(.99)},'workload':'Per invocation two issued scripted run handles + human handle; one producer emits17artifact bytes and2048stderr bytes over32writes/2ms sleeps during takeover; consumer not launched in takeover trials. Normal journey launches producer and exact artifact consumer. No concurrent invocation load.','environment':{'system':platform.platform(),'machine':platform.machine(),'cpu_count':os.cpu_count()},'limits':'In-process decode/control/encode latency, excluding 1ms observation polling, launch, external IPC/model/PTY/render. Small n, p95/p99 both uppermost sample; descriptive, no SLO/capacity or physical human input claim.'}
    # Derive workload byte count rather than preserve a mistaken display constant.
    summary['workload']=summary['workload'].replace('17artifact',str(len(b'namtab-artifact-v1'))+'artifact')
    (root/'checks.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))

if __name__=='__main__':
    if not __debug__:raise SystemExit('Flow verification requires Python assertions; disable optimization.')
    main()
