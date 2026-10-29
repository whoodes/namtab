"""Pure result checks for the finite engine-load/1 experiment, not a scheduler model."""
import os
from pathlib import Path
import select
import time

def require(value,message):
    if not value:raise AssertionError(message)

PROFILE='namtab.engine-load/1'
CONTROL_P95_NS=100_000_000
SERVICE_NS=2_000_000_000
RSS_CAP=1_073_741_824
PHASES=('quiet','noisy','stalled')

def quantiles(values):
    require(bool(values),'missing latency samples')
    v=sorted(values)
    return dict(n=len(v),p50_ns=v[int((len(v)-1)*.5)],p95_ns=v[int((len(v)-1)*.95)],p99_ns=v[int((len(v)-1)*.99)],max_ns=max(v),method='floor((n-1)*p), no interpolation; sparse tails unqualified')

def assess(samples, resources, reads, expected, windows):
    require(len(samples)==54,'expected54retained interventions; failures cannot vanish')
    require({(s['repetition'],s['phase'],s['cycle']) for s in samples}=={(r,p,c) for r in range(3) for p in PHASES for c in range(6)},'missing/duplicate intervention')
    for s in samples:
        require(s['ok'] and s['stale_bytes']==0 and s['probe_exact'],'failed/unknown/duplicated intervention')
        for k in ['takeover_ns','text_ns','enter_ns','probe_ns']:require(0<s[k]<=SERVICE_NS,'service deadline exceeded')
        require(s['peer_progress_a'] and s['peer_progress_b'],'independent peer did not progress')
    for sample in samples:
        require(any(w['repetition']==sample['repetition'] and w['phase']==sample['phase'] and w['start_ns']<=sample['start_ns']<w['end_ns'] for w in windows),'intervention outside admitted phase')
    wanted={(p['pid'],p['starttime']) for p in expected}
    require(len(wanted)==7,'expected owned broker plus6known supervised processes')
    require(len(windows)==9 and {(w['repetition'],w['phase']) for w in windows}=={(i,p) for i in range(3) for p in PHASES},'missing/duplicate stage windows')
    require(len(resources)>=2,'missing resource campaign')
    for row in resources:
        require(0<row['rss_bytes']<=RSS_CAP and row['processes']==len(wanted),'resource over-budget/incomplete census')
        require({(p['pid'],p['starttime']) for p in row['identities']}==wanted,'resource process identity gap')
    require(all(0<b['time_ns']-a['time_ns']<=250_000_000 for a,b in zip(resources,resources[1:])),'reversed/missing resource sampling interval')
    for window in windows:
        start,end=window['start_ns'],window['end_ns'];require(start<end,'invalid stage window')
        cohort=[x for x in resources if start<=x['time_ns']<=end]
        require(len(cohort)>=max(2,int((end-start)/100_000_000)),'insufficient resource coverage within stage')
        require(resources[0]['time_ns']<=start and resources[-1]['time_ns']>=end-100_000_000,'stale resource campaign')
        for scope in ['work_export','session','attempt']:
            require(any(r['repetition']==window['repetition'] and r['phase']==window['phase'] and r['scope']==scope and start<=r['start_ns'] and r['start_ns']+r['elapsed_ns']<=end for r in reads),'eligible reader missing stage/scope progress')
    require(all(r['ok'] and 0<r['elapsed_ns']<=SERVICE_NS for r in reads),'read failure/timeout excluded from service')
    summaries={p:quantiles([s['takeover_ns'] for s in samples if s['phase']==p]) for p in PHASES}
    return {'functional':'PASS','control_candidate':'PASS' if all(s['p95_ns']<=CONTROL_P95_NS for s in summaries.values()) else 'FAIL','takeover_by_phase':summaries,'takeover_all':quantiles([s['takeover_ns'] for s in samples]),'sampled_rss_max':max(x['rss_bytes'] for x in resources),'resource_samples':len(resources),'resource_gap_max_ns':max([b['time_ns']-a['time_ns'] for a,b in zip(resources,resources[1:])]+[0]),'limits':'Sampled RSS sum double-counts shared pages and misses between-sample peaks. No physical input/paint or rare-tail qualification.'}

def owned_resources(descriptors):
    """Read metadata only while each retained owned pidfd still denotes live process."""
    result={'time_ns':time.monotonic_ns(),'rss_bytes':0,'threads':0,'fds':0,'cpu_ticks':0,'processes':0,'identities':[]}
    for pid,fd in list(descriptors.items()):
        if select.select([fd],[],[],0)[0]:raise AssertionError('owned resource process exited during measured campaign')
        root=Path('/proc')/str(pid)
        try:
            fields=(root/'stat').read_text().rsplit(')',1)[1].split();status=(root/'status').read_text().splitlines()
            memory=next((int(l.split()[1])*1024 for l in status if l.startswith('VmRSS:')),0)
            fds=len(list((root/'fd').iterdir()))
            if select.select([fd],[],[],0)[0]:raise AssertionError('owned resource process exited during measured campaign')
            result['rss_bytes']+=memory;result['threads']+=int(fields[17]);result['fds']+=fds;result['cpu_ticks']+=int(fields[11])+int(fields[12]);result['processes']+=1;result['identities'].append({'pid':pid,'starttime':fields[19]})
        except FileNotFoundError:raise AssertionError('owned process missing in resource census')
    return result

def assess_workload(attempts, details, origins, revision, phase_progress, canaries):
    require(set(a['id'] for a in attempts)==set(origins),'missing attributed attempt')
    for i,a in enumerate(attempts):
        require(a['origin']==origins[a['id']] and a['task_revision']==revision and a['state']=='exited' and a['exit_code']==0,'failed/misattributed managed job')
        own=canaries[a['id']];other='ONLY_B_' if own=='ONLY_A_' else 'ONLY_A_'
        require(own in details[a['id']]['stdout'] and other not in details[a['id']]['stdout'],'missing/crossed stream canary')
    require(len(phase_progress)==9 and {(p['repetition'],p['phase']) for p in phase_progress}=={(r,k) for r in range(3) for k in PHASES},'missing workload phase')
    require(all(p['a_end']>p['a_start'] and p['b_end']>p['b_start'] for p in phase_progress),'managed job starved/exited during stage')
