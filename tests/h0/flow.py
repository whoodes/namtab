#!/usr/bin/env python3
"""Real fixed-process workflow + independently checked bounded report. No user attach."""
import argparse, copy, hashlib, json, math, os, platform, subprocess
from pathlib import Path


def sha(b):
    return hashlib.sha256(b).hexdigest()


def audit(r, takeover):
    p=r['provenance']
    assert p['version']=='namtab.flow/0' and p['profile']=='h0-flow-once/0'
    assert p['environment']==[['PATH','/usr/bin:/bin'],['LC_ALL','C']]
    assert p['producer_argv']==['noisy' if takeover else 'producer']
    assert p['consumer_format']=='consumed:%s\n'
    assert p['expected_artifact_sha256']==sha(b'namtab-artifact-v1')
    assert r['context']==sha(json.dumps(p,ensure_ascii=False,separators=(',',':')).encode())
    for f in ['fixture_sha256','consumer_sha256','request_schema_sha256','response_schema_sha256']:
        assert len(p[f])==64 and all(c in '0123456789abcdef' for c in p[f])
    rs=r['receipts']
    assert len(rs)==(1 if takeover else 2)
    for i,receipt in enumerate(rs):
        origin=receipt['origin']
        assert origin['run']==('producer' if i==0 else 'consumer')
        assert origin['context']==r['context'] and origin['control']==1 and origin['connection']==1
        assert origin['task']==f'task-{i}' and origin['attempt']==f'attempt-{i}-1'
        assert origin['profile']=='h0-flow-once/0' and len(origin['epoch'])==32
        assert receipt['sequence']==1 and receipt['calls']==1 and receipt['state']=='completed'
        assert receipt['exit_code']==0 and receipt['drained'] is True
    if takeover:
        assert r['artifact'] is None and r['takeover_during_running_output'] is True
        assert isinstance(r['takeover_us'],int) and r['takeover_us']>=0
        assert rs[0]['dependency'] is None
        assert r['output_counts']==[[len(b'namtab-artifact-v1'),2048],[0,0]]
    else:
        artifact=r['artifact']
        assert artifact==rs[1]['dependency'] and artifact['revision']==1
        assert artifact['context']==r['context'] and artifact['epoch']==rs[0]['origin']['epoch']
        assert artifact['producer_attempt']==rs[0]['origin']['attempt']
        assert artifact['sha256']==sha(b'namtab-artifact-v1') and artifact['bytes']==len(b'namtab-artifact-v1')
        assert rs[0]['stdout_sha256']==artifact['sha256']
        assert rs[1]['stdout_sha256']==sha(b'consumed:namtab-artifact-v1\n')
        assert r['output_counts']==[[len(b'namtab-artifact-v1'),len(b'producer-diagnostic')],[len(b'consumed:namtab-artifact-v1\n'),0]]
        assert r['takeover_us'] is None and r['takeover_during_running_output'] is False
    # Consumer projection is scoped; process diagnostics never become shared data.
    consumer=r['updates'][1 if takeover else 0]
    assert consumer['binding']['run']=='consumer'
    assert all(e['origin'] is None or e['origin']['run']=='consumer' for e in consumer['events'])
    final=r['updates'][-1]
    assert final['human'] is True and final['control']==2
    assert all(0<=n<=4096 for counts in r['output_counts'] for n in counts)
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
    ]
    for mutate in mutations:
        bad=copy.deepcopy(normal);mutate(bad)
        try:audit(bad,False)
        except (AssertionError,KeyError):pass
        else:raise AssertionError('corruption escaped independent report audit')
    for mutate in [lambda r:r.update(artifact=normal['artifact']),lambda r:r.update(takeover_during_running_output=False)]:
        bad=copy.deepcopy(reports[-1]);mutate(bad)
        try:audit(bad,True)
        except AssertionError:pass
        else:raise AssertionError('takeover corruption escaped report audit')
    samples.sort()
    percentile=lambda p:samples[max(0,math.ceil(len(samples)*p)-1)]
    summary={'status':'PASS','normal_invocations':4,'takeover_invocations':16,'fault_invocations':2,'negative_report_mutations_rejected':9,'takeover_microseconds':{'samples':samples,'p50':percentile(.5),'p95':percentile(.95),'p99':percentile(.99)},'workload':'Per invocation two issued scripted run handles + human handle; one producer emits17artifact bytes and2048stderr bytes over32writes/2ms sleeps during takeover; consumer not launched in takeover trials. Normal journey launches producer and exact artifact consumer. No concurrent invocation load.','environment':{'system':platform.platform(),'machine':platform.machine(),'cpu_count':os.cpu_count()},'limits':'In-process decode/control/encode latency, excluding 1ms observation polling, launch, external IPC/model/PTY/render. Small n, p95/p99 both uppermost sample; descriptive, no SLO/capacity or physical human input claim.'}
    # Derive workload byte count rather than preserve a mistaken display constant.
    summary['workload']=summary['workload'].replace('17artifact',str(len(b'namtab-artifact-v1'))+'artifact')
    (root/'checks.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))

if __name__=='__main__':main()
