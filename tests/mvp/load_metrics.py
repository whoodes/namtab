"""Pure result checks for the finite engine-load/0 experiment, not a scheduler model."""
import os
from pathlib import Path
import select
import time

def require(value,message):
    if not value:raise AssertionError(message)

PROFILE='namtab.engine-load/0'
CONTROL_P95_NS=100_000_000
SERVICE_NS=2_000_000_000
RSS_CAP=1_073_741_824
PHASES=('quiet','noisy','stalled')

def quantiles(values):
    require(bool(values),'missing latency samples')
    v=sorted(values)
    return dict(n=len(v),p50_ns=v[int((len(v)-1)*.5)],p95_ns=v[int((len(v)-1)*.95)],p99_ns=v[int((len(v)-1)*.99)],max_ns=max(v),method='floor((n-1)*p), no interpolation; sparse tails unqualified')

def assess(samples, resources, reads):
    require(len(samples)==54,'expected54retained interventions; failures cannot vanish')
    require({(s['repetition'],s['phase'],s['cycle']) for s in samples}=={(r,p,c) for r in range(3) for p in PHASES for c in range(6)},'missing/duplicate intervention')
    for s in samples:
        require(s['ok'] and s['stale_bytes']==0 and s['probe_exact'],'failed/unknown/duplicated intervention')
        for k in ['takeover_ns','text_ns','enter_ns','probe_ns']:require(0<s[k]<=SERVICE_NS,'service deadline exceeded')
        require(s['peer_progress'],'independent peer did not progress')
    require(resources and all(0<x['rss_bytes']<=RSS_CAP and x['processes']>0 for x in resources),'missing/over-budget process samples')
    require({(r['repetition'],r['phase'],r['scope']) for r in reads} >= {(i,p,k) for i in range(3) for p in PHASES for k in ['work_export','session','attempt']},'eligible reader missing stage/scope progress')
    require(all(r['ok'] and 0<r['elapsed_ns']<=SERVICE_NS for r in reads),'read failure/timeout excluded from service')
    summaries={p:quantiles([s['takeover_ns'] for s in samples if s['phase']==p]) for p in PHASES}
    return {'functional':'PASS','control_candidate':'PASS' if all(s['p95_ns']<=CONTROL_P95_NS for s in summaries.values()) else 'FAIL','takeover_by_phase':summaries,'takeover_all':quantiles([s['takeover_ns'] for s in samples]),'sampled_rss_max':max(x['rss_bytes'] for x in resources),'resource_samples':len(resources),'resource_gap_max_ns':max([b['time_ns']-a['time_ns'] for a,b in zip(resources,resources[1:])]+[0]),'limits':'Sampled RSS sum double-counts shared pages and misses between-sample peaks. No physical input/paint or rare-tail qualification.'}

def owned_resources(descriptors):
    """Read metadata only while each retained owned pidfd still denotes live process."""
    result={'time_ns':time.monotonic_ns(),'rss_bytes':0,'threads':0,'fds':0,'cpu_ticks':0,'processes':0}
    for pid,fd in list(descriptors.items()):
        if select.select([fd],[],[],0)[0]:continue
        root=Path('/proc')/str(pid)
        try:
            fields=(root/'stat').read_text().rsplit(')',1)[1].split();status=(root/'status').read_text().splitlines()
            memory=next((int(l.split()[1])*1024 for l in status if l.startswith('VmRSS:')),0)
            fds=len(list((root/'fd').iterdir()))
            if select.select([fd],[],[],0)[0]:continue
            result['rss_bytes']+=memory;result['threads']+=int(fields[17]);result['fds']+=fds;result['cpu_ticks']+=int(fields[11])+int(fields[12]);result['processes']+=1
        except FileNotFoundError:continue
    return result
